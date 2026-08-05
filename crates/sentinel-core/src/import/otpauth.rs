//! `otpauth://` URI parser.
//!
//! Implements the Key URI Format documented at:
//! <https://github.com/google/google-authenticator/wiki/Key-Uri-Format>
//!
//! Format:
//! ```text
//! otpauth://TYPE/LABEL?PARAMETERS
//! ```
//!
//! - `TYPE` = `totp` or `hotp`
//! - `LABEL` = `issuer:account` (issuer is optional but recommended; the
//!   colon-space separator is the convention)
//! - `PARAMETERS`:
//!   - `secret` (required, Base32 per RFC 4648)
//!   - `issuer` (optional, overrides the issuer from the label)
//!   - `algorithm` (optional, default SHA1; one of SHA1, SHA256, SHA512)
//!   - `digits` (optional, default 6; one of 6, 8)
//!   - `period` (optional, TOTP only, default 30 seconds)
//!   - `counter` (required for HOTP; the initial counter value)
//!
//! ## Security
//!
//! All fields from the URI are untrusted. We:
//!  - Reject URIs longer than 2048 `bytes` (`DoS` guard).
//!  - Validate the Base32 secret via the `otp::base32` module (which checks
//!    alphabet, length, and minimum byte count).
//!  - Reject unknown `algorithm` values (default to SHA1 only if the
//!    parameter is absent).
//!  - Reject `digits` values other than 6 and 8.
//!  - Reject `period` values of 0 or > 600 (10 minutes — generous upper bound).
//!  - Reject negative or absurdly large HOTP counters.
//!  - Percent-decode the label and issuer (they may contain encoded colons,
//!    spaces, etc.).
//!  - Cap label and issuer length to 256 `chars` (`DoS` guard).

use url::Url;

use crate::error::{Result, SentinelError};
use crate::models::{AccountRecord, Digits, OtpAlgorithm, OtpType, Secret};
use crate::otp::base32;
use crate::vault::now_ms;

/// Maximum allowed URI length. Real `otpauth://` URIs are typically under
/// 200 `bytes`; 2048 is a generous `DoS` guard.
const MAX_URI_LEN: usize = 2048;

/// Maximum allowed length for issuer or account label strings.
const MAX_LABEL_LEN: usize = 256;

/// Maximum allowed TOTP period (seconds). Real values are 30 or 60.
/// 600 (10 minutes) is a generous upper bound.
const MAX_PERIOD: u32 = 600;

/// Parsed `otpauth://` URI. Contains the decoded fields needed to create
/// an `AccountRecord`. The secret is held as a `Secret` (zeroized on drop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOtpauth {
    pub otp_type: OtpType,
    pub issuer: String,
    pub label: String,
    pub secret: Secret,
    pub algorithm: OtpAlgorithm,
    pub digits: Digits,
    pub period: u32,
    pub counter: u64,
}

impl ParsedOtpauth {
    /// Convert the parsed URI into a new `AccountRecord` with a fresh UUID,
    /// current timestamps, and default sort position.
    #[must_use]
    pub fn into_account_record(self, id: String) -> AccountRecord {
        let now = now_ms();
        AccountRecord {
            id,
            issuer: self.issuer,
            label: self.label,
            secret: self.secret,
            otp_type: self.otp_type,
            algorithm: self.algorithm,
            digits: self.digits,
            period: self.period,
            counter: self.counter,
            tags: Vec::new(),
            favorite: false,
            sort_position: 0,
            icon_color: None,
            icon_text: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Parse an `otpauth://` URI into a `ParsedOtpauth`.
///
/// # Errors
/// - [`SentinelError::InvalidOtpauthUri`] for malformed URIs, missing
///   required fields, or invalid field values.
/// - [`SentinelError::InvalidSecret`] if the secret is not valid Base32
///   or is too short.
/// - [`SentinelError::PayloadTooLarge`] if the URI exceeds 2048 bytes.
#[allow(clippy::too_many_lines)]
pub fn parse_otpauth_uri(uri: &str) -> Result<ParsedOtpauth> {
    if uri.len() > MAX_URI_LEN {
        return Err(SentinelError::PayloadTooLarge {
            max: MAX_URI_LEN,
            got: uri.len(),
        });
    }

    let parsed = Url::parse(uri)
        .map_err(|e| SentinelError::InvalidOtpauthUri(format!("URL parse error: {e}")))?;

    if parsed.scheme() != "otpauth" {
        return Err(SentinelError::InvalidOtpauthUri(format!(
            "Expected 'otpauth' scheme, got '{}'.",
            parsed.scheme()
        )));
    }

    if parsed.host_str().is_none() {
        return Err(SentinelError::InvalidOtpauthUri(
            "Missing OTP type (totp or hotp) in URI host.".to_string(),
        ));
    }

    let otp_type = match parsed.host_str().unwrap_or("") {
        "totp" => OtpType::Totp,
        "hotp" => OtpType::Hotp,
        other => {
            return Err(SentinelError::InvalidOtpauthUri(format!(
                "Unsupported OTP type '{other}'. Expected 'totp' or 'hotp'."
            )));
        }
    };

    // The "path" of an otpauth URI is "/LABEL" — the label is everything
    // after the first slash. The label may itself contain slashes (rare
    // but legal) and colons.
    let path = parsed.path();
    if path.is_empty() || path == "/" {
        return Err(SentinelError::InvalidOtpauthUri(
            "Missing account label in URI path.".to_string(),
        ));
    }
    // Strip the leading "/" — `path` always starts with "/" per URL spec.
    let label_raw = path.strip_prefix('/').unwrap_or(path);

    // Percent-decode the label. `label_decoded` is a String (UTF-8).
    let label_decoded = percent_decode(label_raw);

    // Split label into "issuer:account" if a colon is present.
    // Per the spec, the issuer is optional but recommended. If present,
    // it should be followed by a colon and optional space.
    let (label_issuer, account) = split_label(&label_decoded);

    // Query parameters
    let query_pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Extract required + optional parameters
    let secret_str = get_query_value(&query_pairs, "secret").ok_or_else(|| {
        SentinelError::InvalidOtpauthUri("Missing required 'secret' parameter.".to_string())
    })?;

    let secret_bytes = base32::decode_secret(&secret_str)?;

    let query_issuer = get_query_value(&query_pairs, "issuer");
    let algorithm = match get_query_value(&query_pairs, "algorithm") {
        Some(s) => OtpAlgorithm::from_str_ci(&s).ok_or_else(|| {
            SentinelError::InvalidOtpauthUri(
                "Unknown 'algorithm' value. Use SHA1, SHA256, or SHA512.".to_string(),
            )
        })?,
        None => OtpAlgorithm::Sha1,
    };

    let digits = match get_query_value(&query_pairs, "digits") {
        Some(d) => d
            .parse::<u32>()
            .ok()
            .and_then(Digits::from_u32)
            .ok_or_else(|| {
                SentinelError::InvalidOtpauthUri("Invalid 'digits' value. Use 6 or 8.".to_string())
            })?,
        None => Digits::Six,
    };

    let period = match otp_type {
        OtpType::Totp => match get_query_value(&query_pairs, "period") {
            Some(p) => {
                let p = p.parse::<u32>().map_err(|_| {
                    SentinelError::InvalidOtpauthUri(
                        "Invalid 'period' value. Must be a positive integer.".to_string(),
                    )
                })?;
                if p == 0 || p > MAX_PERIOD {
                    return Err(SentinelError::InvalidOtpauthUri(format!(
                        "Invalid 'period' value. Must be between 1 and {MAX_PERIOD} seconds."
                    )));
                }
                p
            }
            None => 30,
        },
        OtpType::Hotp => 30, // Ignored for HOTP
    };

    let counter = match otp_type {
        OtpType::Hotp => {
            let c = get_query_value(&query_pairs, "counter").ok_or_else(|| {
                SentinelError::InvalidOtpauthUri("HOTP requires a 'counter' parameter.".to_string())
            })?;
            c.parse::<u64>().map_err(|_| {
                SentinelError::InvalidOtpauthUri(
                    "Invalid 'counter' value. Must be a non-negative integer.".to_string(),
                )
            })?
        }
        OtpType::Totp => 0,
    };

    // Issuer resolution: query parameter takes precedence over label issuer.
    // If neither is present, use an empty string (the UI will prompt).
    let issuer = query_issuer
        .or(label_issuer)
        .unwrap_or_default()
        .trim()
        .to_string();

    let account = account.trim().to_string();

    // Length guards
    if issuer.len() > MAX_LABEL_LEN {
        return Err(SentinelError::InvalidOtpauthUri(format!(
            "Issuer is too long (max {MAX_LABEL_LEN} characters)."
        )));
    }
    if account.len() > MAX_LABEL_LEN {
        return Err(SentinelError::InvalidOtpauthUri(format!(
            "Account label is too long (max {MAX_LABEL_LEN} characters)."
        )));
    }
    if account.is_empty() {
        return Err(SentinelError::InvalidOtpauthUri(
            "Account label is empty.".to_string(),
        ));
    }

    Ok(ParsedOtpauth {
        otp_type,
        issuer,
        label: account,
        secret: Secret::new(secret_bytes),
        algorithm,
        digits,
        period,
        counter,
    })
}

/// Get the first value for a query parameter key (case-insensitive).
/// `otpauth://` URIs in the wild use both `secret` and `Secret` — we match
/// case-insensitively for interoperability.
fn get_query_value(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.clone())
}

/// Split a label like "Example:alice@example.com" into (issuer, account).
/// Returns `(None, full_label)` if no colon is present.
/// Per the spec, the issuer is everything before the first colon, and the
/// account is everything after (with an optional leading space stripped).
fn split_label(label: &str) -> (Option<String>, String) {
    label.find(':').map_or_else(
        || (None, label.to_string()),
        |colon_pos| {
            let issuer = label[..colon_pos].trim();
            let account = label[colon_pos + 1..].trim_start_matches(' ').trim();
            (
                if issuer.is_empty() {
                    None
                } else {
                    Some(issuer.to_string())
                },
                account.to_string(),
            )
        },
    )
}

/// Percent-decode a string (e.g. "alice%40example.com" → "alice@example.com").
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[allow(clippy::missing_const_for_fn)]
fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 test secret "12345678901234567890" base32-encoded.
    const RFC_SECRET_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn parse_minimal_totp_uri() {
        let uri = format!("otpauth://totp/Example:alice@example.com?secret={RFC_SECRET_B32}");
        let parsed = parse_otpauth_uri(&uri).expect("minimal TOTP URI must parse");
        assert_eq!(parsed.otp_type, OtpType::Totp);
        assert_eq!(parsed.issuer, "Example");
        assert_eq!(parsed.label, "alice@example.com");
        assert_eq!(parsed.algorithm, OtpAlgorithm::Sha1);
        assert_eq!(parsed.digits, Digits::Six);
        assert_eq!(parsed.period, 30);
        assert_eq!(parsed.counter, 0);
    }

    #[test]
    fn parse_full_totp_uri() {
        let uri = format!(
            "otpauth://totp/ACME:bob%40example.com?secret={RFC_SECRET_B32}&issuer=ACME&algorithm=SHA256&digits=8&period=60"
        );
        let parsed = parse_otpauth_uri(&uri).expect("full TOTP URI must parse");
        assert_eq!(parsed.otp_type, OtpType::Totp);
        assert_eq!(parsed.issuer, "ACME");
        assert_eq!(parsed.label, "bob@example.com");
        assert_eq!(parsed.algorithm, OtpAlgorithm::Sha256);
        assert_eq!(parsed.digits, Digits::Eight);
        assert_eq!(parsed.period, 60);
    }

    #[test]
    fn parse_hotp_uri() {
        let uri = format!("otpauth://hotp/Example:alice?secret={RFC_SECRET_B32}&counter=42");
        let parsed = parse_otpauth_uri(&uri).expect("HOTP URI must parse");
        assert_eq!(parsed.otp_type, OtpType::Hotp);
        assert_eq!(parsed.issuer, "Example");
        assert_eq!(parsed.label, "alice");
        assert_eq!(parsed.counter, 42);
    }

    #[test]
    fn query_issuer_overrides_label_issuer() {
        let uri =
            format!("otpauth://totp/LabelIssuer:alice?secret={RFC_SECRET_B32}&issuer=QueryIssuer");
        let parsed = parse_otpauth_uri(&uri).expect("must parse");
        assert_eq!(parsed.issuer, "QueryIssuer");
        assert_eq!(parsed.label, "alice");
    }

    #[test]
    fn label_without_issuer_uses_empty_issuer() {
        let uri = format!("otpauth://totp/alice@example.com?secret={RFC_SECRET_B32}");
        let parsed = parse_otpauth_uri(&uri).expect("must parse");
        assert_eq!(parsed.issuer, "");
        assert_eq!(parsed.label, "alice@example.com");
    }

    #[test]
    fn label_with_colon_space_separator() {
        let uri = format!("otpauth://totp/ACME: alice?secret={RFC_SECRET_B32}");
        let parsed = parse_otpauth_uri(&uri).expect("must parse");
        assert_eq!(parsed.issuer, "ACME");
        assert_eq!(parsed.label, "alice");
    }

    #[test]
    fn case_insensitive_query_keys() {
        let uri = format!("otpauth://totp/Example:alice?SECRET={RFC_SECRET_B32}&Issuer=Example");
        let parsed = parse_otpauth_uri(&uri).expect("must parse");
        assert_eq!(parsed.issuer, "Example");
        // Secret should still decode correctly regardless of case in the key
        assert_eq!(parsed.secret.as_bytes(), b"12345678901234567890");
    }

    #[test]
    fn algorithm_case_insensitive() {
        for alg in ["SHA1", "sha1", "Sha1", "SHA-1", "sha_1"] {
            let uri =
                format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&algorithm={alg}");
            let parsed = parse_otpauth_uri(&uri).unwrap_or_else(|e| panic!("alg '{alg}': {e}"));
            assert_eq!(parsed.algorithm, OtpAlgorithm::Sha1, "alg '{alg}'");
        }
        for alg in ["SHA256", "sha256", "Sha256", "SHA-256"] {
            let uri =
                format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&algorithm={alg}");
            let parsed = parse_otpauth_uri(&uri).unwrap_or_else(|e| panic!("alg '{alg}': {e}"));
            assert_eq!(parsed.algorithm, OtpAlgorithm::Sha256, "alg '{alg}'");
        }
    }

    #[test]
    fn rejects_wrong_scheme() {
        let result = parse_otpauth_uri("https://totp/Example:alice?secret=foo");
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_unknown_otp_type() {
        let uri = format!("otpauth://xtp/Example:alice?secret={RFC_SECRET_B32}");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_missing_secret() {
        let uri = "otpauth://totp/Example:alice";
        let result = parse_otpauth_uri(uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_invalid_secret() {
        let uri = "otpauth://totp/Example:alice?secret=NOT_VALID_BASE32!";
        let result = parse_otpauth_uri(uri);
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn rejects_unknown_algorithm() {
        let uri = format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&algorithm=MD5");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_invalid_digits() {
        let uri = format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&digits=7");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_invalid_period() {
        let uri = format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&period=0");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));

        let uri = format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&period=99999");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_hotp_without_counter() {
        let uri = format!("otpauth://hotp/Example:alice?secret={RFC_SECRET_B32}");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_missing_label() {
        let uri = format!("otpauth://totp/?secret={RFC_SECRET_B32}");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_empty_label() {
        let uri = format!("otpauth://totp/   ?secret={RFC_SECRET_B32}");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_oversized_uri() {
        let huge = "x".repeat(MAX_URI_LEN + 1);
        let uri = format!("otpauth://totp/Example:alice?secret={RFC_SECRET_B32}&junk={huge}");
        let result = parse_otpauth_uri(&uri);
        assert!(matches!(result, Err(SentinelError::PayloadTooLarge { .. })));
    }

    #[test]
    fn percent_decodes_label() {
        let uri = format!("otpauth://totp/Issue%3AAlice%20Bob?secret={RFC_SECRET_B32}");
        let parsed = parse_otpauth_uri(&uri).expect("must parse");
        // "%3A" is ":", so the label becomes "Issue:Alice Bob"
        // The first colon splits into issuer="Issue", account="Alice Bob"
        assert_eq!(parsed.issuer, "Issue");
        assert_eq!(parsed.label, "Alice Bob");
    }

    #[test]
    fn into_account_record_preserves_all_fields() {
        let uri = format!(
            "otpauth://totp/ACME:bob?secret={RFC_SECRET_B32}&issuer=ACME&algorithm=SHA256&digits=8&period=60"
        );
        let parsed = parse_otpauth_uri(&uri).expect("must parse");
        let account = parsed.into_account_record("test-id".to_string());

        assert_eq!(account.id, "test-id");
        assert_eq!(account.issuer, "ACME");
        assert_eq!(account.label, "bob");
        assert_eq!(account.algorithm, OtpAlgorithm::Sha256);
        assert_eq!(account.digits, Digits::Eight);
        assert_eq!(account.period, 60);
        assert_eq!(account.secret.as_bytes(), b"12345678901234567890");
        assert!(account.created_at > 0);
        assert!(account.updated_at > 0);
    }
}
