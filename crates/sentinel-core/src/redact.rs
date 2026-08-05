//! Log redaction layer.
//!
//! Strip secrets, base32-looking strings, and other sensitive tokens from
//! any string before it leaves the process. Tests cover the common cases.
//! Used by the future tracing/log layer in M9.

use regex::Regex;
use std::sync::OnceLock;

/// Redact known sensitive substrings from a log/message line.
///
/// Replaces:
///  - `otpauth-migration://...` URIs
///  - `otpauth://...` URIs (the URI may include the secret in the query)
///  - long base32-looking tokens (≥16 chars in [A-Z2-7])
///  - `secret=...` query params
///  - `password=...` query params
///  - `data=...` query params (Google migration)
#[must_use]
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();

    // URL-style params first (before we munge the rest)
    out = param_re()
        .replace_all(&out, "${key}=<redacted>")
        .to_string();

    // Full otpauth-migration URIs
    out = migration_uri_re()
        .replace_all(&out, "otpauth-migration://<redacted>")
        .to_string();

    // Full otpauth URIs (the URI may carry the secret in the query string)
    out = otpauth_uri_re()
        .replace_all(&out, "otpauth://<redacted>")
        .to_string();

    // Long base32-looking tokens
    out = base32_re()
        .replace_all(&out, "<redacted-base32>")
        .to_string();

    out
}

fn param_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Matches: secret=VALUE | password=VALUE | data=VALUE | key=VALUE | token=VALUE | passcode=VALUE
        // VALUE is anything that is not whitespace, ampersand, quote, or angle bracket.
        Regex::new(r#"(?i)(?P<key>secret|password|data|key|token|passcode)=([^\s&'"<>]+)"#)
            .expect("hardcoded regex must compile")
    })
}

fn migration_uri_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"otpauth-migration://\S+").expect("hardcoded regex must compile"))
}

fn otpauth_uri_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"otpauth://\S+").expect("hardcoded regex must compile"))
}

fn base32_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    // 16+ chars of [A-Z2-7] — typical minimum for an OTP secret.
    R.get_or_init(|| Regex::new(r"\b[A-Z2-7]{16,}\b").expect("hardcoded regex must compile"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_base32_token() {
        let s = "Decoded secret: JBSWY3DPEHPK3PXPGEZDGNBVGY3TQOJQ";
        let r = redact(s);
        assert!(!r.contains("JBSWY3DPEHPK3PXP"));
        assert!(r.contains("<redacted-base32>"));
    }

    #[test]
    fn redacts_secret_param() {
        let s = "GET /?secret=JBSWY3DPEHPK3PXP&foo=bar";
        let r = redact(s);
        assert!(!r.contains("JBSWY3DPEHPK3PXP"));
        assert!(r.contains("secret=<redacted>"));
    }

    #[test]
    fn redacts_otpauth_uri() {
        let s = "Scanned: otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        let r = redact(s);
        assert!(!r.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(r.contains("otpauth://<redacted>"));
    }

    #[test]
    fn redacts_migration_uri() {
        let s = "otpauth-migration://offline?data=CiYKCkhlbGxvId6tvu8SDKVzZWNyZXQoATACEAEYASAAKglwcm9kdWN0ZXM=";
        let r = redact(s);
        assert!(r.contains("otpauth-migration://<redacted>"));
        assert!(!r.contains("data="));
    }

    #[test]
    fn leaves_non_sensitive_text_intact() {
        let s = "User clicked the Save button at 2025-01-01 12:00:00";
        let r = redact(s);
        assert_eq!(r, s);
    }
}
