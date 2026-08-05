//! Manual account creation validation.
//!
//! Validates user-supplied input for the "Add account manually" form.
//! All fields are untrusted — we normalize and validate defensively.

use crate::error::{Result, SentinelError};
use crate::models::{AccountRecord, Digits, OtpAlgorithm, OtpType, Secret};
use crate::otp::base32;
use crate::vault::now_ms;

/// Maximum allowed length for issuer or account label strings.
const MAX_LABEL_LEN: usize = 256;

/// Maximum allowed TOTP period (seconds).
const MAX_PERIOD: u32 = 600;

/// Input for manual account creation. All strings are untrusted user input.
#[derive(Debug, Clone)]
pub struct ManualAccountInput {
    pub issuer: String,
    pub label: String,
    pub secret: String, // Base32, will be normalized + decoded
    pub otp_type: OtpType,
    pub algorithm: OtpAlgorithm,
    pub digits: Digits,
    pub period: u32,  // TOTP only
    pub counter: u64, // HOTP only
    pub icon_color: Option<String>,
    pub icon_text: Option<String>,
}

/// Validate a manual account creation input and convert it into a new
/// `AccountRecord` with a fresh UUID and current timestamps.
///
/// # Errors
/// - [`SentinelError::InvalidSecret`] if the secret is not valid Base32.
/// - [`SentinelError::InvalidOtpauthUri`] (reused) if other fields are invalid.
/// - [`SentinelError::PayloadTooLarge`] if any field exceeds length limits.
pub fn validate_manual_account(input: ManualAccountInput, id: String) -> Result<AccountRecord> {
    // Normalize + validate the issuer
    let issuer = input.issuer.trim().to_string();
    if issuer.len() > MAX_LABEL_LEN {
        return Err(SentinelError::InvalidOtpauthUri(format!(
            "Issuer is too long (max {MAX_LABEL_LEN} characters)."
        )));
    }

    // Normalize + validate the account label
    let label = input.label.trim().to_string();
    if label.is_empty() {
        return Err(SentinelError::InvalidOtpauthUri(
            "Account label is required.".to_string(),
        ));
    }
    if label.len() > MAX_LABEL_LEN {
        return Err(SentinelError::InvalidOtpauthUri(format!(
            "Account label is too long (max {MAX_LABEL_LEN} characters)."
        )));
    }

    // Decode the Base32 secret (normalize handles spaces, lowercase, padding)
    let secret_bytes = base32::decode_secret(&input.secret)?;
    let secret = Secret::new(secret_bytes);

    // Validate period for TOTP
    if input.otp_type == OtpType::Totp && (input.period == 0 || input.period > MAX_PERIOD) {
        return Err(SentinelError::InvalidOtpauthUri(format!(
            "Period must be between 1 and {MAX_PERIOD} seconds."
        )));
    }

    // Validate icon fields if present
    if let Some(ref color) = input.icon_color {
        if color.len() > 32 {
            return Err(SentinelError::InvalidOtpauthUri(
                "Icon color is too long.".to_string(),
            ));
        }
        // Basic hex color validation: #RRGGBB
        if !color.starts_with('#') || color.len() != 7 {
            return Err(SentinelError::InvalidOtpauthUri(
                "Icon color must be in #RRGGBB format.".to_string(),
            ));
        }
        if !color[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SentinelError::InvalidOtpauthUri(
                "Icon color must be valid hexadecimal.".to_string(),
            ));
        }
    }
    if let Some(ref text) = input.icon_text {
        if text.chars().count() > 3 {
            return Err(SentinelError::InvalidOtpauthUri(
                "Icon text must be at most 3 characters.".to_string(),
            ));
        }
    }

    let now = now_ms();
    Ok(AccountRecord {
        id,
        issuer,
        label,
        secret,
        otp_type: input.otp_type,
        algorithm: input.algorithm,
        digits: input.digits,
        period: input.period,
        counter: input.counter,
        tags: Vec::new(),
        favorite: false,
        sort_position: 0,
        icon_color: input.icon_color,
        icon_text: input.icon_text,
        created_at: now,
        updated_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_SECRET_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    fn valid_input() -> ManualAccountInput {
        ManualAccountInput {
            issuer: "GitHub".to_string(),
            label: "alice@example.com".to_string(),
            secret: RFC_SECRET_B32.to_string(),
            otp_type: OtpType::Totp,
            algorithm: OtpAlgorithm::Sha1,
            digits: Digits::Six,
            period: 30,
            counter: 0,
            icon_color: None,
            icon_text: None,
        }
    }

    #[test]
    fn valid_manual_account_succeeds() {
        let account = validate_manual_account(valid_input(), "id-1".to_string())
            .expect("valid input must succeed");
        assert_eq!(account.id, "id-1");
        assert_eq!(account.issuer, "GitHub");
        assert_eq!(account.label, "alice@example.com");
        assert_eq!(account.secret.as_bytes(), b"12345678901234567890");
        assert_eq!(account.otp_type, OtpType::Totp);
        assert_eq!(account.period, 30);
    }

    #[test]
    fn normalizes_secret_whitespace_and_case() {
        let mut input = valid_input();
        input.secret = "gezd gnbv gy3t qojq gezd gnbv gy3t qojq".to_string();
        let account = validate_manual_account(input, "id".to_string()).expect("must succeed");
        assert_eq!(account.secret.as_bytes(), b"12345678901234567890");
    }

    #[test]
    fn rejects_empty_label() {
        let mut input = valid_input();
        input.label = "   ".to_string();
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_invalid_secret() {
        let mut input = valid_input();
        input.secret = "NOT_BASE32!".to_string();
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidSecret(_))));
    }

    #[test]
    fn rejects_zero_period() {
        let mut input = valid_input();
        input.period = 0;
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_huge_period() {
        let mut input = valid_input();
        input.period = 999_999;
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_oversized_issuer() {
        let mut input = valid_input();
        input.issuer = "x".repeat(MAX_LABEL_LEN + 1);
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_oversized_label() {
        let mut input = valid_input();
        input.label = "x".repeat(MAX_LABEL_LEN + 1);
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn rejects_bad_icon_color() {
        let mut input = valid_input();
        input.icon_color = Some("not-a-color".to_string());
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn accepts_valid_icon_color() {
        let mut input = valid_input();
        input.icon_color = Some("#ff5733".to_string());
        input.icon_text = Some("GH".to_string());
        let account = validate_manual_account(input, "id".to_string()).expect("must succeed");
        assert_eq!(account.icon_color.as_deref(), Some("#ff5733"));
        assert_eq!(account.icon_text.as_deref(), Some("GH"));
    }

    #[test]
    fn rejects_long_icon_text() {
        let mut input = valid_input();
        input.icon_text = Some("ABCD".to_string()); // 4 chars, max is 3
        let result = validate_manual_account(input, "id".to_string());
        assert!(matches!(result, Err(SentinelError::InvalidOtpauthUri(_))));
    }

    #[test]
    fn hotp_account_works() {
        let mut input = valid_input();
        input.otp_type = OtpType::Hotp;
        input.counter = 42;
        let account = validate_manual_account(input, "id".to_string()).expect("must succeed");
        assert_eq!(account.otp_type, OtpType::Hotp);
        assert_eq!(account.counter, 42);
    }

    #[test]
    fn trims_whitespace_in_issuer_and_label() {
        let mut input = valid_input();
        input.issuer = "  GitHub  ".to_string();
        input.label = "  alice@example.com  ".to_string();
        let account = validate_manual_account(input, "id".to_string()).expect("must succeed");
        assert_eq!(account.issuer, "GitHub");
        assert_eq!(account.label, "alice@example.com");
    }
}
