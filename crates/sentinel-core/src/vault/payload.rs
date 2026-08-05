//! Encrypted vault payload (the plaintext that lives inside the AES-GCM
//! ciphertext).
//!
//! The payload is CBOR-encoded for compactness. It contains:
//!
//! ```text
//! PayloadV1 {
//!     schema_version: u16,    // payload schema version (independent of file format version)
//!     created_at: u64,        // ms since epoch when the vault was first created
//!     modified_at: u64,       // ms since epoch of the last save
//!     accounts: [AccountRecord],
//! }
//! ```
//!
//! The `schema_version` lets us migrate the payload format in the future
//! (e.g. adding new fields) without changing the file format. Migrations
//! run after decryption, on the in-memory plaintext.

use ciborium::de::from_reader;
use ciborium::ser::into_writer;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, SentinelError};
use crate::models::AccountRecord;

/// Current payload schema version. Bump when the CBOR structure changes.
pub const PAYLOAD_SCHEMA_VERSION: u16 = 1;

/// The decrypted vault payload. Lives in memory only while the vault is
/// unlocked. The whole struct is dropped (and the contained Secrets
/// zeroized) when the vault locks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultPayload {
    pub schema_version: u16,
    pub created_at: u64,
    pub modified_at: u64,
    pub accounts: Vec<AccountRecord>,
}

impl VaultPayload {
    /// Create a new empty payload with the current schema version and
    /// timestamps set to now.
    #[must_use]
    pub fn new_empty() -> Self {
        let now = now_ms();
        Self {
            schema_version: PAYLOAD_SCHEMA_VERSION,
            created_at: now,
            modified_at: now,
            accounts: Vec::new(),
        }
    }

    /// Serialize to CBOR bytes.
    ///
    /// # Errors
    /// Returns [`SentinelError::Cbor`] if serialization fails (should not
    /// happen for valid `AccountRecord` values).
    pub fn to_cbor(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        into_writer(self, &mut buf).map_err(|e| SentinelError::Cbor(e.to_string()))?;
        Ok(buf)
    }

    /// Deserialize from CBOR bytes.
    ///
    /// # Errors
    /// Returns [`SentinelError::Cbor`] if the bytes are not valid CBOR or
    /// do not deserialize into a `VaultPayload`.
    /// Returns [`SentinelError::CorruptVault`] if the schema version is
    /// not supported.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let payload: Self = from_reader(bytes).map_err(|e| SentinelError::Cbor(e.to_string()))?;
        if payload.schema_version != PAYLOAD_SCHEMA_VERSION {
            // Future: migrate older schemas here. For now we only support v1.
            return Err(SentinelError::CorruptVault);
        }
        Ok(payload)
    }

    /// Mark the payload as modified (bumps `modified_at` to now).
    pub fn touch(&mut self) {
        self.modified_at = now_ms();
    }
}

/// Current time in milliseconds since the Unix epoch.
#[must_use]
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Digits, OtpAlgorithm, OtpType, Secret};

    fn dummy_account(id: &str, issuer: &str) -> AccountRecord {
        let now = now_ms();
        AccountRecord {
            id: id.to_string(),
            issuer: issuer.to_string(),
            label: format!("user@{issuer}.example.com"),
            secret: Secret::new(b"12345678901234567890".to_vec()),
            otp_type: OtpType::Totp,
            algorithm: OtpAlgorithm::Sha1,
            digits: Digits::Six,
            period: 30,
            counter: 0,
            tags: Vec::new(),
            favorite: false,
            sort_position: 0,
            icon_color: None,
            icon_text: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn payload_round_trip_empty() {
        let p = VaultPayload::new_empty();
        let bytes = p.to_cbor().expect("serialize");
        let p2 = VaultPayload::from_cbor(&bytes).expect("deserialize");
        assert_eq!(p2.schema_version, PAYLOAD_SCHEMA_VERSION);
        assert!(p2.accounts.is_empty());
        assert_eq!(p2.created_at, p.created_at);
    }

    #[test]
    fn payload_round_trip_with_accounts() {
        let mut p = VaultPayload::new_empty();
        p.accounts.push(dummy_account("acc-1", "GitHub"));
        p.accounts.push(dummy_account("acc-2", "GitLab"));

        let bytes = p.to_cbor().expect("serialize");
        let p2 = VaultPayload::from_cbor(&bytes).expect("deserialize");

        assert_eq!(p2.accounts.len(), 2);
        assert_eq!(p2.accounts[0].id, "acc-1");
        assert_eq!(p2.accounts[0].issuer, "GitHub");
        assert_eq!(p2.accounts[0].label, "user@GitHub.example.com");
        assert_eq!(p2.accounts[1].id, "acc-2");
        assert_eq!(p2.accounts[1].issuer, "GitLab");
    }

    #[test]
    fn payload_rejects_unknown_schema_version() {
        // Manually craft a payload with schema_version=999
        let p = VaultPayload {
            schema_version: 999,
            created_at: 0,
            modified_at: 0,
            accounts: vec![],
        };
        let bytes = p.to_cbor().expect("serialize");
        let result = VaultPayload::from_cbor(&bytes);
        assert!(matches!(result, Err(SentinelError::CorruptVault)));
    }

    #[test]
    fn payload_rejects_garbage_cbor() {
        let garbage = b"this is not cbor";
        let result = VaultPayload::from_cbor(garbage);
        assert!(matches!(result, Err(SentinelError::Cbor(_))));
    }

    #[test]
    fn touch_bumps_modified_at() {
        let mut p = VaultPayload::new_empty();
        let original = p.modified_at;
        std::thread::sleep(std::time::Duration::from_millis(2));
        p.touch();
        assert!(p.modified_at > original);
    }
}
