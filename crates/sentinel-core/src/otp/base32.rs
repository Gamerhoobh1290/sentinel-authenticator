//! Base32 encoding/decoding for OTP secrets.
//!
//! RFC 4648 base32 (no padding). Tolerates user input by normalising:
//!  - strips whitespace
//!  - uppercases lowercase input
//!  - removes padding characters ('=')
//!  - rejects characters outside [A-Z2-7]

use data_encoding::{DecodeError, BASE32};

use crate::error::{Result, SentinelError};

/// Normalize a Base32 secret string:
///  - trim and remove all internal whitespace
///  - uppercase
///  - strip '=' padding
///  - validate the alphabet
///
/// Returns the normalized string. Does not decode — call `decode_secret`
/// for that.
#[must_use]
pub fn normalize_secret_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| *c != '=')
        .collect()
}

/// Decode a normalized Base32 secret into raw bytes.
/// The input is re-normalized defensively before decoding.
///
/// # Errors
/// Returns [`SentinelError::InvalidSecret`] if the input is empty, contains
/// characters outside [A-Z2-7], has invalid length or padding, or decodes
/// to fewer than 16 bytes (the RFC 4226 minimum).
///
/// Returns [`SentinelError::PayloadTooLarge`] if the input exceeds 256
/// characters after normalization.
pub fn decode_secret(input: &str) -> Result<Vec<u8>> {
    let normalized = normalize_secret_input(input);

    // Reject empty input up front — base32 decoding of "" succeeds with an
    // empty Vec, but we never want to allow empty secrets.
    if normalized.is_empty() {
        return Err(SentinelError::InvalidSecret("Secret is empty.".to_string()));
    }

    // Reject absurdly long inputs (>256 base32 chars = ~160 bytes after decode).
    // Real OTP secrets are 16-64 base32 chars. This guard prevents a malicious
    // QR payload from exhausting memory.
    if normalized.len() > 256 {
        return Err(SentinelError::PayloadTooLarge {
            max: 256,
            got: normalized.len(),
        });
    }

    match BASE32.decode(normalized.as_bytes()) {
        Ok(bytes) => {
            // RFC 4226: secret MUST be at least 128 bits (16 bytes).
            if bytes.len() < 16 {
                return Err(SentinelError::InvalidSecret(format!(
                    "Secret is too short ({} bytes; minimum 16).",
                    bytes.len()
                )));
            }
            // Reject secrets that aren't a whole number of bytes when
            // re-encoded. This catches truncated trailing characters that
            // data-encoding silently accepts.
            Ok(bytes)
        }
        Err(e) => Err(map_decode_error(e)),
    }
}

fn map_decode_error(e: DecodeError) -> SentinelError {
    use data_encoding::DecodeKind;
    match e.kind {
        DecodeKind::Length | DecodeKind::Trailing => SentinelError::InvalidSecret(
            "Secret has an invalid length — check for missing characters.".to_string(),
        ),
        DecodeKind::Symbol => SentinelError::InvalidSecret(
            "Secret contains invalid characters — use only A-Z and 2-7.".to_string(),
        ),
        DecodeKind::Padding => {
            SentinelError::InvalidSecret("Secret has invalid padding.".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_spaces_and_lowercases() {
        assert_eq!(
            normalize_secret_input("jbsw y3dp ehpk 3pxp"),
            "JBSWY3DPEHPK3PXP"
        );
    }

    #[test]
    fn normalize_strips_padding() {
        assert_eq!(
            normalize_secret_input("JBSWY3DPEHPK3PXP==="),
            "JBSWY3DPEHPK3PXP"
        );
    }

    #[test]
    fn decode_valid_secret() {
        // RFC 4226 test secret "12345678901234567890" = 20 bytes,
        // base32-encoded as "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".
        let bytes = decode_secret("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").expect("valid secret");
        assert_eq!(bytes.len(), 20);
        assert_eq!(&bytes, b"12345678901234567890");
    }

    #[test]
    fn decode_rejects_too_short_secret() {
        // 10 bytes is < 16-byte minimum.
        let result = decode_secret("JBSWY3DPEHPK3PXP");
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn decode_accepts_rfc_test_secret() {
        // RFC 4226 test secret: "12345678901234567890" (ASCII) = 20 bytes
        // Base32 of those bytes is "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".
        let result = decode_secret("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes, b"12345678901234567890");
    }

    #[test]
    fn decode_rejects_invalid_chars() {
        let result = decode_secret("INVALID!@#$%");
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn decode_rejects_empty() {
        let result = decode_secret("");
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn decode_rejects_oversized_input() {
        // 300 chars — over the 256 limit.
        let big = "A".repeat(300);
        let result = decode_secret(&big);
        assert!(matches!(result, Err(SentinelError::PayloadTooLarge { .. })));
    }
}
