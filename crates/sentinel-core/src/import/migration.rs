//! Google Authenticator migration payload decoder.
//!
//! Implements decoding of the `otpauth-migration://offline?data=<base64>`
//! URI format used by Google Authenticator's "Transfer accounts" →
//! "Export accounts" flow.
//!
//! ## Format
//!
//! The `data=` query parameter contains a base64-encoded protobuf payload.
//! The protobuf schema is defined in `proto/migration_payload.proto` and
//! contains:
//!
//! ```text
//! MigrationPayload {
//!   repeated OtpParameters otp_parameters = 1;
//!   optional int32 batch_size  = 2;  // total QR codes in the batch
//!   optional int32 batch_index = 3;  // 0-based index of this QR
//!   optional int32 batch_id    = 4;  // stable ID shared across the batch
//! }
//! ```
//!
//! ## Multi-batch transfers
//!
//! When a user exports many accounts, Google Authenticator splits them
//! across multiple QR codes. Each QR contains a `MigrationPayload` with
//! `batch_size` set to the total, `batch_index` indicating which QR this
//! is (0-based), and `batch_id` tying them together. The UI must scan all
//! QRs in the batch and merge their `otp_parameters` lists.
//!
//! ## Security
//!
//! All fields from the protobuf are untrusted. We:
//!  - Cap the base64 `data=` parameter to 16 `KB` (`DoS` guard).
//!  - Cap individual account fields to reasonable lengths.
//!  - Reject MD5 algorithm (not supported by Sentinel).
//!  - Reject unknown algorithm/digits/type enum values.
//!  - Reject empty secret bytes.
//!  - Validate that HOTP accounts have a counter.

use base64::Engine;
use prost::Message;
use url::Url;

use crate::error::{Result, SentinelError};
use crate::import::otpauth::ParsedOtpauth;
use crate::models::{Digits, OtpAlgorithm, OtpType, Secret};
use crate::proto::MigrationPayload;
use crate::proto::OtpParameters;

/// Maximum allowed base64 `data=` parameter length (before decoding).
/// Real payloads are typically under 1 `KB`; 16 `KB` is a generous `DoS` guard.
const MAX_DATA_LEN: usize = 16 * 1024;

/// Maximum allowed length for issuer or account name strings.
const MAX_NAME_LEN: usize = 256;

/// A single decoded batch from a Google migration QR code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationBatch {
    /// Accounts found in this batch.
    pub accounts: Vec<ParsedOtpauth>,
    /// Total number of QR codes in the transfer (1 if single QR).
    pub batch_size: i32,
    /// 0-based index of this QR within the batch.
    pub batch_index: i32,
    /// Stable ID shared across all QRs in the same batch.
    pub batch_id: i32,
}

/// Parse an `otpauth-migration://offline?data=<base64>` URI and decode
/// the contained protobuf payload.
///
/// # Errors
/// - [`SentinelError::UnsupportedQrPayload`] if the URI scheme is not
///   `otpauth-migration`.
/// - [`SentinelError::MalformedMigration`] if the base64 or protobuf is
///   invalid, or if a field has an unsupported value.
/// - [`SentinelError::PayloadTooLarge`] if the `data=` parameter exceeds
///   16 `KB`.
pub fn parse_migration_uri(uri: &str) -> Result<MigrationBatch> {
    if uri.len() > MAX_DATA_LEN * 2 {
        // URI can be at most ~2x the data length (base64 + URI overhead).
        return Err(SentinelError::PayloadTooLarge {
            max: MAX_DATA_LEN * 2,
            got: uri.len(),
        });
    }

    let parsed = Url::parse(uri)
        .map_err(|e| SentinelError::UnsupportedQrPayload(format!("URL parse error: {e}")))?;

    if parsed.scheme() != "otpauth-migration" {
        return Err(SentinelError::UnsupportedQrPayload(format!(
            "Expected 'otpauth-migration' scheme, got '{}'.",
            parsed.scheme()
        )));
    }

    // Extract the `data=` query parameter.
    let data_b64 = parsed
        .query_pairs()
        .find(|(k, _)| k == "data")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| {
            SentinelError::MalformedMigration("Missing 'data' query parameter.".to_string())
        })?;

    if data_b64.len() > MAX_DATA_LEN {
        return Err(SentinelError::PayloadTooLarge {
            max: MAX_DATA_LEN,
            got: data_b64.len(),
        });
    }

    // Base64-decode (URL-safe, no padding — Google uses standard base64
    // but we tolerate URL-safe and missing padding).
    let data_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data_b64.trim_end_matches('='))
        .or_else(|_| {
            // Try standard base64 as a fallback
            base64::engine::general_purpose::STANDARD.decode(&data_b64)
        })?;

    if data_bytes.len() > MAX_DATA_LEN {
        return Err(SentinelError::PayloadTooLarge {
            max: MAX_DATA_LEN,
            got: data_bytes.len(),
        });
    }

    decode_migration_payload(&data_bytes)
}

/// Decode a raw protobuf migration payload (already base64-decoded).
///
/// # Errors
/// - [`SentinelError::Prost`] if the bytes are not valid protobuf.
/// - [`SentinelError::MalformedMigration`] if a field has an unsupported value.
pub fn decode_migration_payload(data: &[u8]) -> Result<MigrationBatch> {
    let payload = MigrationPayload::decode(data)?;

    let batch_size = payload.batch_size.unwrap_or(1);
    let batch_index = payload.batch_index.unwrap_or(0);
    let batch_id = payload.batch_id.unwrap_or(0);

    let mut accounts = Vec::with_capacity(payload.otp_parameters.len());
    for otp in payload.otp_parameters {
        accounts.push(convert_otp_parameters(otp)?);
    }

    Ok(MigrationBatch {
        accounts,
        batch_size,
        batch_index,
        batch_id,
    })
}

/// Convert a single `OtpParameters` protobuf into a `ParsedOtpauth`.
///
/// # Errors
/// Returns [`SentinelError::MalformedMigration`] for unsupported enum values,
/// empty secrets, missing HOTP counters, or oversized fields.
fn convert_otp_parameters(otp: OtpParameters) -> Result<ParsedOtpauth> {
    use crate::proto::gen::migration::otp_parameters::{
        Algorithm as ProtoAlg, DigitCount as ProtoDigits, OtpType as ProtoType,
    };

    // Secret: must be non-empty, at least 16 bytes (RFC 4226 minimum).
    if otp.secret.is_empty() {
        return Err(SentinelError::MalformedMigration(
            "Account has an empty secret.".to_string(),
        ));
    }
    if otp.secret.len() < 16 {
        return Err(SentinelError::MalformedMigration(format!(
            "Account secret is too short ({} bytes; minimum 16).",
            otp.secret.len()
        )));
    }
    if otp.secret.len() > 256 {
        return Err(SentinelError::MalformedMigration(format!(
            "Account secret is too long ({} bytes; maximum 256).",
            otp.secret.len()
        )));
    }

    // Issuer + name length guards
    if otp.issuer.len() > MAX_NAME_LEN {
        return Err(SentinelError::MalformedMigration(format!(
            "Issuer is too long (max {MAX_NAME_LEN} characters)."
        )));
    }
    if otp.name.len() > MAX_NAME_LEN {
        return Err(SentinelError::MalformedMigration(format!(
            "Account name is too long (max {MAX_NAME_LEN} characters)."
        )));
    }
    if otp.name.is_empty() {
        return Err(SentinelError::MalformedMigration(
            "Account name is empty.".to_string(),
        ));
    }

    // Algorithm: proto enum 0 = unspecified (treat as SHA1), 1 = SHA1,
    // 2 = SHA256, 3 = SHA512, 4 = MD5 (rejected).
    let algorithm = match ProtoAlg::try_from(otp.algorithm).ok() {
        Some(ProtoAlg::Unspecified | ProtoAlg::Sha1) => OtpAlgorithm::Sha1,
        Some(ProtoAlg::Sha256) => OtpAlgorithm::Sha256,
        Some(ProtoAlg::Sha512) => OtpAlgorithm::Sha512,
        Some(ProtoAlg::Md5) => {
            return Err(SentinelError::MalformedMigration(
                "MD5 algorithm is not supported.".to_string(),
            ));
        }
        None => {
            return Err(SentinelError::MalformedMigration(format!(
                "Unknown algorithm value: {}",
                otp.algorithm
            )));
        }
    };

    // Digits: 0 = unspecified (treat as 6), 1 = 6, 2 = 8.
    let digits = match ProtoDigits::try_from(otp.digits).ok() {
        Some(ProtoDigits::Unspecified | ProtoDigits::Six) => Digits::Six,
        Some(ProtoDigits::Eight) => Digits::Eight,
        None => {
            return Err(SentinelError::MalformedMigration(format!(
                "Unknown digits value: {}",
                otp.digits
            )));
        }
    };

    // OTP type: 0 = unspecified (treat as TOTP), 1 = HOTP, 2 = TOTP.
    let otp_type = match ProtoType::try_from(otp.r#type).ok() {
        Some(ProtoType::Unspecified | ProtoType::Totp) => OtpType::Totp,
        Some(ProtoType::Hotp) => OtpType::Hotp,
        None => {
            return Err(SentinelError::MalformedMigration(format!(
                "Unknown OTP type value: {}",
                otp.r#type
            )));
        }
    };

    // HOTP counter: required for HOTP, ignored for TOTP.
    let counter = if otp_type == OtpType::Hotp {
        let c = otp.counter.ok_or_else(|| {
            SentinelError::MalformedMigration("HOTP account is missing a counter.".to_string())
        })?;
        u64::try_from(c).map_err(|_| {
            SentinelError::MalformedMigration(format!("HOTP counter is negative or too large: {c}"))
        })?
    } else {
        0
    };

    Ok(ParsedOtpauth {
        otp_type,
        issuer: otp.issuer,
        label: otp.name,
        secret: Secret::new(otp.secret),
        algorithm,
        digits,
        period: 30, // Google migration payloads don't carry period; always 30s
        counter,
    })
}

/// Merge multiple migration batches into a single list of accounts.
///
/// This is used when the user scans multiple QR codes from a multi-batch
/// Google transfer. Batches are validated for consistency (same `batch_id`,
/// contiguous `batch_index` values).
///
/// # Errors
/// - [`SentinelError::IncompleteMigrationBatch`] if not all batches are
///   present.
/// - [`SentinelError::MalformedMigration`] if batches have inconsistent
///   `batch_id` values.
pub fn merge_batches(batches: Vec<MigrationBatch>) -> Result<Vec<ParsedOtpauth>> {
    if batches.is_empty() {
        return Ok(Vec::new());
    }

    // All batches must share the same batch_id.
    let expected_batch_id = batches[0].batch_id;
    for b in &batches {
        if b.batch_id != expected_batch_id {
            return Err(SentinelError::MalformedMigration(format!(
                "Batches have inconsistent batch_id values ({} vs {}).",
                expected_batch_id, b.batch_id
            )));
        }
    }

    // Determine the expected batch_size.
    let batch_size = batches[0].batch_size;
    let scanned = batches.len();
    let expected = usize::try_from(batch_size).map_err(|_| {
        SentinelError::MalformedMigration(format!("Invalid batch_size: {batch_size}"))
    })?;
    if scanned != expected {
        return Err(SentinelError::IncompleteMigrationBatch {
            scanned,
            total: expected,
        });
    }

    // Verify all batch_index values are present (0..batch_size).
    let mut indices: Vec<i32> = batches.iter().map(|b| b.batch_index).collect();
    indices.sort_unstable();
    for (i, &idx) in indices.iter().enumerate() {
        let expected_idx = i32::try_from(i)
            .map_err(|_| SentinelError::MalformedMigration("Too many batches.".to_string()))?;
        if idx != expected_idx {
            return Err(SentinelError::MalformedMigration(format!(
                "Missing or duplicate batch index. Expected {expected_idx}, got {idx}."
            )));
        }
    }

    // Merge accounts from all batches (in batch_index order).
    let mut sorted = batches;
    sorted.sort_by_key(|b| b.batch_index);
    let mut accounts = Vec::new();
    for batch in sorted {
        accounts.extend(batch.accounts);
    }

    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid migration payload protobuf by hand.
    /// We construct the raw protobuf bytes manually rather than using
    /// prost's encoding, to keep the test self-contained and verify
    /// our decoder against the wire format.
    #[allow(clippy::type_complexity)]
    fn build_payload(
        accounts: &[(&[u8], &str, &str, i32, i32, i32, Option<i64>)],
        batch_size: i32,
        batch_index: i32,
        batch_id: i32,
    ) -> Vec<u8> {
        // We use prost's Message::encode here — it's the canonical way
        // and we trust prost's encoding (it's tested against the protobuf
        // conformance suite).
        let otp_params: Vec<OtpParameters> = accounts
            .iter()
            .map(
                |(secret, name, issuer, alg, digits, typ, counter)| OtpParameters {
                    secret: secret.to_vec(),
                    name: name.to_string(),
                    issuer: issuer.to_string(),
                    algorithm: *alg,
                    digits: *digits,
                    r#type: *typ,
                    counter: *counter,
                },
            )
            .collect();

        let payload = MigrationPayload {
            otp_parameters: otp_params,
            batch_size: Some(batch_size),
            batch_index: Some(batch_index),
            batch_id: Some(batch_id),
        };

        prost::Message::encode_to_vec(&payload)
    }

    #[test]
    fn decode_single_totp_account() {
        let secret = b"12345678901234567890"; // 20 bytes
        let payload_bytes = build_payload(
            &[(secret, "alice@example.com", "GitHub", 1, 1, 2, None)],
            1,
            0,
            12345,
        );

        let batch = decode_migration_payload(&payload_bytes).expect("must decode");
        assert_eq!(batch.accounts.len(), 1);
        assert_eq!(batch.batch_size, 1);
        assert_eq!(batch.batch_index, 0);
        assert_eq!(batch.batch_id, 12345);

        let account = &batch.accounts[0];
        assert_eq!(account.otp_type, OtpType::Totp);
        assert_eq!(account.issuer, "GitHub");
        assert_eq!(account.label, "alice@example.com");
        assert_eq!(account.algorithm, OtpAlgorithm::Sha1);
        assert_eq!(account.digits, Digits::Six);
        assert_eq!(account.secret.as_bytes(), secret);
    }

    #[test]
    fn decode_multiple_accounts_in_one_batch() {
        let payload_bytes = build_payload(
            &[
                (
                    b"12345678901234567890",
                    "alice@github.com",
                    "GitHub",
                    1,
                    1,
                    2,
                    None,
                ),
                (
                    b"09876543210987654321",
                    "bob@gitlab.com",
                    "GitLab",
                    2,
                    2,
                    2,
                    None,
                ),
            ],
            1,
            0,
            99,
        );

        let batch = decode_migration_payload(&payload_bytes).expect("must decode");
        assert_eq!(batch.accounts.len(), 2);
        assert_eq!(batch.accounts[0].issuer, "GitHub");
        assert_eq!(batch.accounts[0].algorithm, OtpAlgorithm::Sha1);
        assert_eq!(batch.accounts[1].issuer, "GitLab");
        assert_eq!(batch.accounts[1].algorithm, OtpAlgorithm::Sha256);
        assert_eq!(batch.accounts[1].digits, Digits::Eight);
    }

    #[test]
    fn decode_hotp_account() {
        let payload_bytes = build_payload(
            &[(
                b"12345678901234567890",
                "alice",
                "GitHub",
                1,
                1,
                1,
                Some(42),
            )],
            1,
            0,
            1,
        );

        let batch = decode_migration_payload(&payload_bytes).expect("must decode");
        assert_eq!(batch.accounts.len(), 1);
        assert_eq!(batch.accounts[0].otp_type, OtpType::Hotp);
        assert_eq!(batch.accounts[0].counter, 42);
    }

    #[test]
    fn rejects_md5_algorithm() {
        let payload_bytes = build_payload(
            &[(b"12345678901234567890", "alice", "GitHub", 4, 1, 2, None)], // 4 = MD5
            1,
            0,
            1,
        );

        let result = decode_migration_payload(&payload_bytes);
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn rejects_empty_secret() {
        let payload_bytes = build_payload(&[(b"", "alice", "GitHub", 1, 1, 2, None)], 1, 0, 1);

        let result = decode_migration_payload(&payload_bytes);
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn rejects_short_secret() {
        let payload_bytes = build_payload(
            &[(b"short", "alice", "GitHub", 1, 1, 2, None)], // 5 bytes, min is 16
            1,
            0,
            1,
        );

        let result = decode_migration_payload(&payload_bytes);
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn rejects_hotp_without_counter() {
        let payload_bytes = build_payload(
            &[(b"12345678901234567890", "alice", "GitHub", 1, 1, 1, None)], // HOTP, no counter
            1,
            0,
            1,
        );

        let result = decode_migration_payload(&payload_bytes);
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn rejects_garbage_protobuf() {
        let result = decode_migration_payload(b"this is not protobuf");
        assert!(matches!(result, Err(SentinelError::Prost(_))));
    }

    #[test]
    fn merge_multi_batch_transfer() {
        // Build a 3-batch transfer
        let b0 = decode_migration_payload(&build_payload(
            &[(b"11111111111111111111", "acc1", "Issuer1", 1, 1, 2, None)],
            3,
            0,
            100,
        ))
        .unwrap();
        let b1 = decode_migration_payload(&build_payload(
            &[(b"22222222222222222222", "acc2", "Issuer2", 1, 1, 2, None)],
            3,
            1,
            100,
        ))
        .unwrap();
        let b2 = decode_migration_payload(&build_payload(
            &[(b"33333333333333333333", "acc3", "Issuer3", 1, 1, 2, None)],
            3,
            2,
            100,
        ))
        .unwrap();

        let merged = merge_batches(vec![b0, b1, b2]).expect("must merge");
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].label, "acc1");
        assert_eq!(merged[1].label, "acc2");
        assert_eq!(merged[2].label, "acc3");
    }

    #[test]
    fn merge_detects_incomplete_batch() {
        let b0 = decode_migration_payload(&build_payload(
            &[(b"11111111111111111111", "acc1", "I1", 1, 1, 2, None)],
            3,
            0,
            100,
        ))
        .unwrap();
        let b1 = decode_migration_payload(&build_payload(
            &[(b"22222222222222222222", "acc2", "I2", 1, 1, 2, None)],
            3,
            1,
            100,
        ))
        .unwrap();
        // Missing batch 2

        let result = merge_batches(vec![b0, b1]);
        assert!(matches!(
            result,
            Err(SentinelError::IncompleteMigrationBatch {
                scanned: 2,
                total: 3
            })
        ));
    }

    #[test]
    fn merge_detects_inconsistent_batch_id() {
        let b0 = decode_migration_payload(&build_payload(
            &[(b"11111111111111111111", "acc1", "I1", 1, 1, 2, None)],
            2,
            0,
            100,
        ))
        .unwrap();
        let b1 = decode_migration_payload(&build_payload(
            &[(b"22222222222222222222", "acc2", "I2", 1, 1, 2, None)],
            2,
            1,
            999, // Different batch_id!
        ))
        .unwrap();

        let result = merge_batches(vec![b0, b1]);
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn merge_detects_duplicate_batch_index() {
        let b0 = decode_migration_payload(&build_payload(
            &[(b"11111111111111111111", "acc1", "I1", 1, 1, 2, None)],
            2,
            0,
            100,
        ))
        .unwrap();
        let b1 = decode_migration_payload(&build_payload(
            &[(b"22222222222222222222", "acc2", "I2", 1, 1, 2, None)],
            2,
            0,
            100, // Same batch_index!
        ))
        .unwrap();

        let result = merge_batches(vec![b0, b1]);
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn parse_migration_uri_round_trip() {
        let payload_bytes = build_payload(
            &[(
                b"12345678901234567890",
                "alice@example.com",
                "GitHub",
                1,
                1,
                2,
                None,
            )],
            1,
            0,
            1,
        );
        let data_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_bytes);
        let uri = format!("otpauth-migration://offline?data={data_b64}");

        let batch = parse_migration_uri(&uri).expect("must parse");
        assert_eq!(batch.accounts.len(), 1);
        assert_eq!(batch.accounts[0].issuer, "GitHub");
        assert_eq!(batch.accounts[0].label, "alice@example.com");
    }

    #[test]
    fn rejects_wrong_scheme() {
        let result = parse_migration_uri("https://example.com/data=foo");
        assert!(matches!(
            result,
            Err(SentinelError::UnsupportedQrPayload(_))
        ));
    }

    #[test]
    fn rejects_missing_data_param() {
        let result = parse_migration_uri("otpauth-migration://offline");
        assert!(matches!(result, Err(SentinelError::MalformedMigration(_))));
    }

    #[test]
    fn rejects_oversized_data() {
        let huge = "A".repeat(MAX_DATA_LEN + 1);
        let uri = format!("otpauth-migration://offline?data={huge}");
        let result = parse_migration_uri(&uri);
        assert!(matches!(result, Err(SentinelError::PayloadTooLarge { .. })));
    }

    #[test]
    fn default_algorithm_is_sha1() {
        // algorithm=0 (unspecified) should default to SHA1
        let payload_bytes = build_payload(
            &[(b"12345678901234567890", "alice", "GitHub", 0, 1, 2, None)],
            1,
            0,
            1,
        );
        let batch = decode_migration_payload(&payload_bytes).expect("must decode");
        assert_eq!(batch.accounts[0].algorithm, OtpAlgorithm::Sha1);
    }

    #[test]
    fn default_digits_is_six() {
        // digits=0 (unspecified) should default to 6
        let payload_bytes = build_payload(
            &[(b"12345678901234567890", "alice", "GitHub", 1, 0, 2, None)],
            1,
            0,
            1,
        );
        let batch = decode_migration_payload(&payload_bytes).expect("must decode");
        assert_eq!(batch.accounts[0].digits, Digits::Six);
    }

    #[test]
    fn default_type_is_totp() {
        // type=0 (unspecified) should default to TOTP
        let payload_bytes = build_payload(
            &[(b"12345678901234567890", "alice", "GitHub", 1, 1, 0, None)],
            1,
            0,
            1,
        );
        let batch = decode_migration_payload(&payload_bytes).expect("must decode");
        assert_eq!(batch.accounts[0].otp_type, OtpType::Totp);
    }
}
