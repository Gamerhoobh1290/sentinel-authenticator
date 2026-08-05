//! TOTP — Time-based One-Time Password (RFC 6238).
//!
//! TOTP is HOTP where the counter is derived from the current Unix time:
//!
//! ```text
//! counter = floor(unix_time_seconds / period)
//! ```
//!
//! RFC 6238 §4.2 specifies the default period as 30 seconds. We also support
//! custom periods for issuers that use them (e.g. 60s).
//!
//! Supported algorithms: SHA-1 (RFC default), SHA-256, SHA-512.
//! Supported digit counts: 6 and 8.
//!
//! # Test vectors
//!
//! RFC 6238 Appendix B publishes vectors for SHA-1, SHA-256, and SHA-512 at
//! five Unix timestamps. Those vectors are covered in the test module.
//!
//! # Time handling
//!
//! The caller supplies `now_unix` (seconds since the Unix epoch, UTC). Sentinel
//! does NOT silently apply any time offset — if the system clock is wrong, the
//! codes will be wrong, and the UI is responsible for surfacing that to the
//! user. We never adjust `now_unix` internally.

use crate::error::{Result, SentinelError};
use crate::models::{CodeResult, Digits, OtpAlgorithm};
use crate::otp::hotp::generate_hotp;

/// Generate a TOTP code.
///
/// `now_unix` is the current Unix timestamp in seconds (UTC).
/// `period` is the TOTP step in seconds (typically 30).
/// `secret` is the raw (already base32-decoded) key bytes.
/// `algorithm` selects HMAC-SHA1/256/512.
/// `digits` selects 6 or 8 digit output.
///
/// # Errors
/// Returns [`SentinelError::InvalidSecret`] if the secret is empty.
/// Returns [`SentinelError::Crypto`] if `period` is 0 (would divide by zero).
pub fn generate_totp(
    account_id: &str,
    secret: &[u8],
    now_unix: u64,
    period: u32,
    algorithm: OtpAlgorithm,
    digits: Digits,
) -> Result<CodeResult> {
    if secret.is_empty() {
        return Err(SentinelError::InvalidSecret("Secret is empty.".to_string()));
    }
    if period == 0 {
        return Err(SentinelError::Crypto);
    }

    // RFC 6238 §4.2: T = floor(unix_time / period)
    let counter = now_unix / u64::from(period);

    let mut result = generate_hotp(account_id, secret, counter, algorithm, digits)?;

    // Augment with TOTP-specific metadata: seconds remaining and period.
    // seconds_remaining = period - (now_unix % period) - 1
    // (the -1 is because we're already inside the current second)
    let elapsed_in_period = now_unix % u64::from(period);
    let remaining = u64::from(period)
        .saturating_sub(elapsed_in_period)
        .saturating_sub(1);
    result.seconds_remaining = Some(u32::try_from(remaining).unwrap_or(0));
    result.period = Some(period);

    Ok(result)
}

/// Compute the time step counter for a given timestamp and period.
/// Exposed so tests and the import layer can verify behaviour at boundaries.
#[must_use]
pub fn time_step_counter(now_unix: u64, period: u32) -> u64 {
    now_unix / u64::from(period)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── RFC 6238 Appendix B test vectors ───────────────────────────────
    //
    // The RFC uses different secrets per algorithm:
    //   SHA-1:   "12345678901234567890"                   (20 bytes)
    //   SHA-256: "12345678901234567890123456789012"        (32 bytes)
    //   SHA-512: "1234567890123456789012345678901234567890123456789012345678901234" (64 bytes)
    //
    // All five test timestamps and expected 8-digit codes per algorithm:
    //
    // Time (sec)      | SHA-1    | SHA-256  | SHA-512
    // ----------------|----------|----------|----------
    // 59              | 94287082 | 46119246 | 90693936
    // 1111111109      | 07081804 | 68084774 | 25091201
    // 1111111111      | 14050471 | 67062674 | 99943326
    // 1234567890      | 89005924 | 91819424 | 93441116
    // 2000000000      | 69279037 | 90698825 | 38618901
    // 20000000000     | 65353130 | 77737706 | 47863826

    const SHA1_SECRET: &[u8] = b"12345678901234567890";
    const SHA256_SECRET: &[u8] = b"12345678901234567890123456789012";
    const SHA512_SECRET: &[u8] =
        b"1234567890123456789012345678901234567890123456789012345678901234";

    const RFC_6238_VECTORS: &[(u64, &str, &str, &str)] = &[
        // (time, sha1, sha256, sha512)
        (59, "94287082", "46119246", "90693936"),
        (1_111_111_109, "07081804", "68084774", "25091201"),
        (1_111_111_111, "14050471", "67062674", "99943326"),
        (1_234_567_890, "89005924", "91819424", "93441116"),
        (2_000_000_000, "69279037", "90698825", "38618901"),
        (20_000_000_000, "65353130", "77737706", "47863826"),
    ];

    #[test]
    fn rfc_6238_sha1_vectors_pass() {
        for &(t, expected, _, _) in RFC_6238_VECTORS {
            let r = generate_totp(
                "test",
                SHA1_SECRET,
                t,
                30,
                OtpAlgorithm::Sha1,
                Digits::Eight,
            )
            .expect("TOTP must succeed for RFC test vector");
            assert_eq!(
                r.code, expected,
                "SHA-1 TOTP mismatch at t={t}: got {}, expected {expected}",
                r.code
            );
        }
    }

    #[test]
    fn rfc_6238_sha256_vectors_pass() {
        for &(t, _, expected, _) in RFC_6238_VECTORS {
            let r = generate_totp(
                "test",
                SHA256_SECRET,
                t,
                30,
                OtpAlgorithm::Sha256,
                Digits::Eight,
            )
            .expect("TOTP must succeed for RFC test vector");
            assert_eq!(
                r.code, expected,
                "SHA-256 TOTP mismatch at t={t}: got {}, expected {expected}",
                r.code
            );
        }
    }

    #[test]
    fn rfc_6238_sha512_vectors_pass() {
        for &(t, _, _, expected) in RFC_6238_VECTORS {
            let r = generate_totp(
                "test",
                SHA512_SECRET,
                t,
                30,
                OtpAlgorithm::Sha512,
                Digits::Eight,
            )
            .expect("TOTP must succeed for RFC test vector");
            assert_eq!(
                r.code, expected,
                "SHA-512 TOTP mismatch at t={t}: got {}, expected {expected}",
                r.code
            );
        }
    }

    #[test]
    fn rfc_6238_six_digit_vectors_pass() {
        // The RFC publishes 8-digit vectors. Sentinel also supports 6-digit
        // output, which is the truncation to the last 6 digits. Cross-check
        // against the well-known 6-digit values for SHA-1 (commonly cited):
        //   t=59          -> 287082
        //   t=1111111109  -> 081804
        //   t=1111111111  -> 050471
        //   t=1234567890  -> 005924
        //   t=2000000000  -> 279037
        //   t=20000000000 -> 353130
        const SIX_DIGIT_SHA1: &[(u64, &str)] = &[
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ];
        for &(t, expected) in SIX_DIGIT_SHA1 {
            let r = generate_totp("test", SHA1_SECRET, t, 30, OtpAlgorithm::Sha1, Digits::Six)
                .expect("TOTP must succeed");
            assert_eq!(
                r.code, expected,
                "6-digit SHA-1 TOTP mismatch at t={t}: got {}, expected {expected}",
                r.code
            );
        }
    }

    // ─── Period boundary tests ─────────────────────────────────────────

    #[test]
    fn period_boundary_codes_change_at_period_edge() {
        // At t=29 the counter is 0; at t=30 it flips to 1. Codes must differ.
        let before =
            generate_totp("test", SHA1_SECRET, 29, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        let at =
            generate_totp("test", SHA1_SECRET, 30, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        let after =
            generate_totp("test", SHA1_SECRET, 31, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();

        assert_ne!(before.code, at.code, "code must change at the 30s boundary");
        assert_eq!(
            at.code, after.code,
            "code must stay the same within the same period"
        );
    }

    #[test]
    fn seconds_remaining_is_correct_at_period_start() {
        // At t=30 (start of period 1), 30 seconds remain (minus the current second = 29).
        let r =
            generate_totp("test", SHA1_SECRET, 30, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        assert_eq!(r.seconds_remaining, Some(29));
        assert_eq!(r.period, Some(30));
    }

    #[test]
    fn seconds_remaining_is_correct_mid_period() {
        let r =
            generate_totp("test", SHA1_SECRET, 45, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        // 45 % 30 = 15 elapsed; remaining = 30 - 15 - 1 = 14
        assert_eq!(r.seconds_remaining, Some(14));
    }

    #[test]
    fn seconds_remaining_is_zero_at_period_end() {
        // At t=29 (last second of period 0), remaining = 30 - 29 - 1 = 0
        let r =
            generate_totp("test", SHA1_SECRET, 29, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        assert_eq!(r.seconds_remaining, Some(0));
    }

    #[test]
    fn custom_period_60s_works() {
        // t=59 -> counter 0; t=60 -> counter 1
        let a =
            generate_totp("test", SHA1_SECRET, 59, 60, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        let b =
            generate_totp("test", SHA1_SECRET, 60, 60, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        assert_ne!(a.code, b.code);
        assert_eq!(a.period, Some(60));
        assert_eq!(b.period, Some(60));
    }

    #[test]
    fn rejects_empty_secret() {
        let result = generate_totp("test", b"", 0, 30, OtpAlgorithm::Sha1, Digits::Six);
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn rejects_zero_period() {
        let result = generate_totp("test", SHA1_SECRET, 0, 0, OtpAlgorithm::Sha1, Digits::Six);
        assert!(matches!(result, Err(SentinelError::Crypto)));
    }

    #[test]
    fn time_step_counter_matches_rfc() {
        // RFC 6238 §4.2: T = floor(unix_time / step)
        assert_eq!(time_step_counter(59, 30), 1);
        assert_eq!(time_step_counter(60, 30), 2);
        assert_eq!(time_step_counter(1_111_111_109, 30), 37_037_036);
    }
}
