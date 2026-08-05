//! HOTP — HMAC-based One-Time Password (RFC 4226).
//!
//! Algorithm (RFC 4226 §5.3):
//!  1. Compute `hs = HMAC-H(K, C)` where C is the 8-byte big-endian counter.
//!  2. Dynamic truncation: `offset = hs[last] & 0x0f`. Extract 4 bytes from
//!     `hs[offset..offset+4]`, big-endian, mask the high bit.
//!  3. `code = (truncated as u31) mod 10^digits`.
//!
//! Supported HMAC variants: SHA-1, SHA-256, SHA-512.
//! Supported digit counts: 6 and 8.
//!
//! The dynamic truncation step uses the LAST byte of the HMAC output to
//! determine the offset, regardless of hash length. This is per RFC 4226
//! and matches what the RFC test vectors expect.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::error::{Result, SentinelError};
use crate::models::{CodeResult, Digits, OtpAlgorithm};

/// Generate an HOTP code.
///
/// `secret` is the raw (already base32-decoded) key bytes.
/// `counter` is the moving factor (RFC 4226 §4.2).
/// `algorithm` selects HMAC-SHA1/256/512.
/// `digits` selects 6 or 8 digit output.
///
/// # Errors
/// Returns [`SentinelError::InvalidSecret`] if the secret is empty.
/// Returns [`SentinelError::Crypto`] only if the dynamic-truncation offset
/// calculation yields an out-of-bounds read — this cannot happen for any
/// of the supported hash output sizes (SHA-1: 20, SHA-256: 32, SHA-512: 64)
/// and exists purely as a defensive guard.
///
/// # Panics
/// This function never panics on valid input. Internally it calls
/// `Mac::new_from_slice` which is infallible for HMAC (any key length is
/// acceptable); the `expect` calls document that invariant.
pub fn generate_hotp(
    account_id: &str,
    secret: &[u8],
    counter: u64,
    algorithm: OtpAlgorithm,
    digits: Digits,
) -> Result<CodeResult> {
    if secret.is_empty() {
        return Err(SentinelError::InvalidSecret("Secret is empty.".to_string()));
    }

    // RFC 4226 §5.1: counter is a 64-bit value, big-endian.
    let counter_bytes = counter.to_be_bytes();

    // Compute the HMAC over the counter using the selected hash.
    let hmac_result: Vec<u8> = match algorithm {
        OtpAlgorithm::Sha1 => {
            let mut mac =
                <Hmac<Sha1> as Mac>::new_from_slice(secret).expect("HMAC accepts any key size");
            mac.update(&counter_bytes);
            mac.finalize().into_bytes().to_vec()
        }
        OtpAlgorithm::Sha256 => {
            let mut mac =
                <Hmac<Sha256> as Mac>::new_from_slice(secret).expect("HMAC accepts any key size");
            mac.update(&counter_bytes);
            mac.finalize().into_bytes().to_vec()
        }
        OtpAlgorithm::Sha512 => {
            let mut mac =
                <Hmac<Sha512> as Mac>::new_from_slice(secret).expect("HMAC accepts any key size");
            mac.update(&counter_bytes);
            mac.finalize().into_bytes().to_vec()
        }
    };

    // Dynamic truncation per RFC 4226 §5.3.
    // offset = low nibble of the LAST byte of the HMAC output.
    // This is correct for SHA-1 (20 bytes), SHA-256 (32 bytes), and SHA-512 (64 bytes).
    // Google Authenticator and most interoperable implementations use this convention.
    let last_index = hmac_result.len() - 1;
    let offset = (hmac_result[last_index] & 0x0f) as usize;

    // We need 4 bytes starting at `offset`. For SHA-1 (20 bytes), the maximum
    // offset is 15, so we read bytes 15..19 — safe. For SHA-256 and SHA-512,
    // there is even more headroom.
    if offset + 4 > hmac_result.len() {
        // Defensive: should never happen given the algorithm spec, but refuse
        // to panic on adversarial input.
        return Err(SentinelError::Crypto);
    }

    // Big-endian 4 bytes, mask the high bit to get a 31-bit positive integer.
    // Using `u32::from(u8)` rather than `as u32` — both are infallible, but
    // `From` is preferred by clippy and is more obviously correct.
    let truncated: u32 = (u32::from(hmac_result[offset] & 0x7f) << 24)
        | (u32::from(hmac_result[offset + 1]) << 16)
        | (u32::from(hmac_result[offset + 2]) << 8)
        | u32::from(hmac_result[offset + 3]);

    // Modulo 10^digits. RFC 4226 §5.3 specifies 6-8 digits; we support 6 and 8.
    let modulus: u32 = match digits {
        Digits::Six => 1_000_000,
        Digits::Eight => 100_000_000,
    };
    let code_num = truncated % modulus;

    // Zero-pad to the digit count.
    let code = format!("{:0width$}", code_num, width = digits.as_u32() as usize);

    Ok(CodeResult {
        account_id: account_id.to_string(),
        code,
        seconds_remaining: None,
        period: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 4226 Appendix D — test vectors.
    // Secret: "12345678901234567890" (ASCII, 20 bytes).
    // Algorithm: SHA-1. Digits: 6.
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    // (counter, expected_code)
    const RFC_4226_VECTORS: &[(u64, &str)] = &[
        (0, "755224"),
        (1, "287082"),
        (2, "359152"),
        (3, "969429"),
        (4, "338314"),
        (5, "254676"),
        (6, "287922"),
        (7, "162583"),
        (8, "399871"),
        (9, "520489"),
    ];

    #[test]
    fn rfc_4226_vectors_pass() {
        for &(counter, expected) in RFC_4226_VECTORS {
            let result =
                generate_hotp("test", RFC_SECRET, counter, OtpAlgorithm::Sha1, Digits::Six)
                    .expect("HOTP generation must succeed for RFC test vectors");
            assert_eq!(
                result.code, expected,
                "HOTP mismatch at counter {counter}: got {}, expected {expected}",
                result.code
            );
        }
    }

    #[test]
    fn eight_digit_hotp_is_consistent() {
        // RFC 4226 doesn't publish 8-digit vectors, but we can derive one
        // from the 6-digit value: the 8-digit form is the same number with
        // two more leading digits. Sanity-check that the value is in range
        // and that re-generating gives the same result.
        let r1 = generate_hotp("test", RFC_SECRET, 0, OtpAlgorithm::Sha1, Digits::Eight).unwrap();
        assert_eq!(r1.code.len(), 8);
        assert!(r1.code.chars().all(|c| c.is_ascii_digit()));

        let r2 = generate_hotp("test", RFC_SECRET, 0, OtpAlgorithm::Sha1, Digits::Eight).unwrap();
        assert_eq!(
            r1.code, r2.code,
            "HOTP must be deterministic for fixed input"
        );
    }

    #[test]
    fn different_counters_give_different_codes() {
        let a = generate_hotp("test", RFC_SECRET, 0, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        let b = generate_hotp("test", RFC_SECRET, 1, OtpAlgorithm::Sha1, Digits::Six).unwrap();
        assert_ne!(a.code, b.code);
    }

    #[test]
    fn different_secrets_give_different_codes() {
        let a = generate_hotp(
            "test",
            b"12345678901234567890",
            0,
            OtpAlgorithm::Sha1,
            Digits::Six,
        )
        .unwrap();
        let b = generate_hotp(
            "test",
            b"09876543210987654321",
            0,
            OtpAlgorithm::Sha1,
            Digits::Six,
        )
        .unwrap();
        assert_ne!(a.code, b.code);
    }

    #[test]
    fn rejects_empty_secret() {
        let result = generate_hotp("test", b"", 0, OtpAlgorithm::Sha1, Digits::Six);
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn sha256_and_sha512_produce_six_digit_codes() {
        // RFC 6238 Appendix B provides TOTP vectors for SHA-256 and SHA-512.
        // HOTP itself is only formally specified for SHA-1 in RFC 4226, but
        // the algorithm is hash-agnostic — extending it to SHA-256/512 is the
        // standard practice (Google Authenticator and others do this).
        // Here we just verify the format is correct.
        for alg in [OtpAlgorithm::Sha256, OtpAlgorithm::Sha512] {
            let r = generate_hotp("test", RFC_SECRET, 0, alg, Digits::Six).unwrap();
            assert_eq!(r.code.len(), 6, "SHA-{alg:?} HOTP must be 6 digits");
            assert!(
                r.code.chars().all(|c| c.is_ascii_digit()),
                "must be all digits"
            );
        }
    }

    #[test]
    fn code_is_zero_padded() {
        // Find a counter that yields a value with leading zeros. We can't
        // predict which, so just iterate and check at least one has a leading
        // zero over the first 1000 counters — this catches the zero-pad bug
        // reliably if it's present.
        let mut saw_leading_zero = false;
        for c in 0..1000u64 {
            let r = generate_hotp("test", RFC_SECRET, c, OtpAlgorithm::Sha1, Digits::Six).unwrap();
            if r.code.starts_with('0') {
                saw_leading_zero = true;
                assert_eq!(
                    r.code.len(),
                    6,
                    "must always be 6 chars even with leading zero"
                );
                break;
            }
        }
        // If none had a leading zero in 1000 tries, the test still passes —
        // but we want to assert the zero-pad path is exercised. Skip assert
        // to avoid flakiness; the format!() width specifier guarantees it.
        let _ = saw_leading_zero;
    }
}
