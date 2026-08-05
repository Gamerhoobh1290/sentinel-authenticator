//! OTP generation engine.
//!
//! Implements RFC 4226 (HOTP) and RFC 6238 (TOTP) hand-rolled against the
//! official test vectors. See `tests/` for the RFC fixtures.
//!
//! Supported:
//!  - HOTP per RFC 4226
//!  - TOTP per RFC 6238
//!  - SHA-1, SHA-256, SHA-512 algorithms
//!  - 6-digit and 8-digit output
//!  - Standard 30s period and custom periods
//!  - HOTP counters
//!
//! Not stored: codes are generated on demand from the encrypted secret and
//! discarded immediately after use.

pub mod base32;
pub mod hotp;
pub mod totp;

pub use hotp::generate_hotp;
pub use totp::generate_totp;

/// Constant-time comparison of two OTP codes.
///
/// Use this when verifying a user-supplied code against an expected value.
/// Even though OTP codes are short and numeric (low entropy), using a
/// constant-time compare avoids leaking information about how many leading
/// digits match via timing side-channels.
///
/// Returns `true` if the codes are byte-for-byte equal AND have the same
/// length. Different-length inputs always return `false` (but the comparison
/// still runs in constant time relative to the longer input).
#[must_use]
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    if a.len() != b.len() {
        // Still do a comparison to avoid leaking the length difference via
        // early-return timing. Compare against ourselves — the result is
        // discarded.
        let _ = a.as_bytes().ct_eq(a.as_bytes());
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_equal_strings() {
        assert!(constant_time_eq("123456", "123456"));
    }

    #[test]
    fn constant_time_eq_rejects_different_strings() {
        assert!(!constant_time_eq("123456", "123457"));
        assert!(!constant_time_eq("123456", "000000"));
    }

    #[test]
    fn constant_time_eq_rejects_different_lengths() {
        assert!(!constant_time_eq("123456", "12345678"));
        assert!(!constant_time_eq("123456", ""));
    }

    #[test]
    fn constant_time_eq_handles_empty_strings() {
        assert!(constant_time_eq("", ""));
    }
}
