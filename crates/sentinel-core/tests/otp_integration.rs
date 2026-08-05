//! Integration tests for the OTP engine.
//!
//! These exercise the public API end-to-end: starting from a base32-encoded
//! secret (the format a user would paste or scan), decode it and generate
//! HOTP/TOTP codes that match the official RFC test vectors.

use sentinel_core::models::{Digits, OtpAlgorithm};
use sentinel_core::otp::{base32, generate_hotp, generate_totp};

// RFC 4226 test secret "12345678901234567890" base32-encoded.
// Cross-checked with: https://datatracker.ietf.org/doc/html/rfc4226#appendix-D
const RFC_SECRET_BASE32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

#[test]
fn end_to_end_hotp_from_base32_matches_rfc_4226() {
    let secret = base32::decode_secret(RFC_SECRET_BASE32).expect("RFC secret must decode");
    assert_eq!(&secret, b"12345678901234567890");

    let expected = [
        "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583", "399871",
        "520489",
    ];

    for (counter, expected_code) in expected.iter().enumerate() {
        let result = generate_hotp(
            "rfc-test",
            &secret,
            counter as u64,
            OtpAlgorithm::Sha1,
            Digits::Six,
        )
        .expect("HOTP generation must succeed");
        assert_eq!(
            result.code, *expected_code,
            "RFC 4226 HOTP mismatch at counter {counter}"
        );
        assert_eq!(result.account_id, "rfc-test");
        assert!(result.seconds_remaining.is_none());
        assert!(result.period.is_none());
    }
}

#[test]
fn end_to_end_totp_from_base32_matches_rfc_6238() {
    // SHA-1 secret from RFC 6238 Appendix B
    let secret = base32::decode_secret(RFC_SECRET_BASE32).expect("RFC secret must decode");

    // (unix_time, expected_8_digit_code)
    let vectors: &[(u64, &str)] = &[
        (59, "94287082"),
        (1_111_111_109, "07081804"),
        (1_111_111_111, "14050471"),
        (1_234_567_890, "89005924"),
        (2_000_000_000, "69279037"),
        (20_000_000_000, "65353130"),
    ];

    for &(t, expected) in vectors {
        let result = generate_totp(
            "rfc-test",
            &secret,
            t,
            30,
            OtpAlgorithm::Sha1,
            Digits::Eight,
        )
        .expect("TOTP generation must succeed");
        assert_eq!(result.code, expected, "RFC 6238 TOTP mismatch at t={t}");
        assert_eq!(result.period, Some(30));
        assert!(result.seconds_remaining.is_some());
    }
}

#[test]
fn user_input_normalization_then_decode_then_generate() {
    // Simulate a user pasting "jbsw y3dp ehpk 3pxp" with spaces and lowercase.
    // This is the canonical example secret from RFC 4226's worked examples.
    // We require >= 16 bytes after decode, so use a longer test secret.
    let user_input = "jbswy3dp ehpk3pxp jbswy3dp ehpk3pxp";
    let secret = base32::decode_secret(user_input).expect("normalized secret must decode");
    assert_eq!(secret.len(), 20);

    // Should be able to generate a code without error.
    let result = generate_totp(
        "user-input-test",
        &secret,
        1_700_000_000,
        30,
        OtpAlgorithm::Sha1,
        Digits::Six,
    )
    .expect("TOTP generation must succeed");
    assert_eq!(result.code.len(), 6);
    assert_eq!(result.account_id, "user-input-test");
}

#[test]
fn codes_dont_match_with_wrong_secret() {
    let secret_a = base32::decode_secret(RFC_SECRET_BASE32).unwrap();
    let secret_b = base32::decode_secret("JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP").unwrap();
    assert_ne!(secret_a, secret_b);

    let code_a = generate_totp(
        "a",
        &secret_a,
        1_700_000_000,
        30,
        OtpAlgorithm::Sha1,
        Digits::Six,
    )
    .unwrap();
    let code_b = generate_totp(
        "b",
        &secret_b,
        1_700_000_000,
        30,
        OtpAlgorithm::Sha1,
        Digits::Six,
    )
    .unwrap();
    assert_ne!(code_a.code, code_b.code);
}

#[test]
fn totp_code_changes_when_period_crosses_boundary() {
    let secret = base32::decode_secret(RFC_SECRET_BASE32).unwrap();
    let at_boundary_minus_1 =
        generate_totp("t", &secret, 29, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
    let at_boundary = generate_totp("t", &secret, 30, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
    let at_boundary_plus_1 =
        generate_totp("t", &secret, 31, 30, OtpAlgorithm::Sha1, Digits::Six).unwrap();
    assert_ne!(at_boundary_minus_1.code, at_boundary.code);
    assert_eq!(at_boundary.code, at_boundary_plus_1.code);
}
