//! Encrypted backup format (v1).
//!
//! Backups are independent of the vault's master password. They use a
//! separate backup password with its own Argon2id key derivation.
//!
//! ## File layout (v1)
//!
//! ```text
//! Offset  Size      Field
//! ------  --------  ----------------------------------------------------
//!   0     8 bytes   Magic: "SENTBK01"
//!   8     2 bytes   Version (u16 LE) — currently 1
//!  10     1 byte    KDF ID (1 = Argon2id)
//!  11     4 bytes   Argon2id m_cost (u32 LE)
//!  15     4 bytes   Argon2id t_cost (u32 LE)
//!  19     4 bytes   Argon2id p_cost (u32 LE)
//!  23    16 bytes   Salt (random)
//!  39    12 bytes   AES-256-GCM nonce (random)
//!  51     N bytes   AES-256-GCM ciphertext (backup_payload_cbor + tag)
//! ```

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{Result, SentinelError};
use crate::models::AccountRecord;
use crate::vault::{
    now_ms, KdfParams, VaultKey, VaultPayload, PAYLOAD_SCHEMA_VERSION, VAULT_FORMAT_VERSION,
};

pub const BACKUP_FORMAT_VERSION: u16 = 1;
pub const BACKUP_MAGIC: &[u8; 8] = b"SENTBK01";
pub const BACKUP_HEADER_LEN: usize = 51;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPayload {
    pub schema_version: u16,
    pub created_at: u64,
    pub source_vault_version: u16,
    pub accounts: Vec<AccountRecord>,
}

impl BackupPayload {
    #[must_use]
    pub fn from_vault(vault: &VaultPayload) -> Self {
        Self {
            schema_version: PAYLOAD_SCHEMA_VERSION,
            created_at: now_ms(),
            source_vault_version: VAULT_FORMAT_VERSION,
            accounts: vault.accounts.clone(),
        }
    }

    #[must_use]
    pub fn into_vault_payload(self) -> VaultPayload {
        VaultPayload {
            schema_version: PAYLOAD_SCHEMA_VERSION,
            created_at: self.created_at,
            modified_at: now_ms(),
            accounts: self.accounts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackupHeader {
    version: u16,
    kdf_params: KdfParams,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
}

impl BackupHeader {
    fn to_bytes(&self) -> [u8; BACKUP_HEADER_LEN] {
        let mut buf = [0u8; BACKUP_HEADER_LEN];
        buf[0..8].copy_from_slice(BACKUP_MAGIC);
        buf[8..10].copy_from_slice(&self.version.to_le_bytes());
        buf[10] = 1;
        buf[11..15].copy_from_slice(&self.kdf_params.m_cost_kib.to_le_bytes());
        buf[15..19].copy_from_slice(&self.kdf_params.t_cost.to_le_bytes());
        buf[19..23].copy_from_slice(&self.kdf_params.p_cost.to_le_bytes());
        buf[23..39].copy_from_slice(&self.salt);
        buf[39..51].copy_from_slice(&self.nonce);
        buf
    }

    fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < BACKUP_HEADER_LEN {
            return Err(SentinelError::CorruptBackup);
        }
        if &buf[0..8] != BACKUP_MAGIC {
            return Err(SentinelError::CorruptBackup);
        }
        let version = u16::from_le_bytes([buf[8], buf[9]]);
        if version != BACKUP_FORMAT_VERSION {
            return Err(SentinelError::UnsupportedBackupVersion(
                BACKUP_FORMAT_VERSION,
                version,
            ));
        }
        if buf[10] != 1 {
            return Err(SentinelError::CorruptBackup);
        }
        let m_cost_kib = u32::from_le_bytes([buf[11], buf[12], buf[13], buf[14]]);
        let t_cost = u32::from_le_bytes([buf[15], buf[16], buf[17], buf[18]]);
        let p_cost = u32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]);
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&buf[23..39]);
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&buf[39..51]);
        Ok(Self {
            version,
            kdf_params: KdfParams {
                m_cost_kib,
                t_cost,
                p_cost,
            },
            salt,
            nonce,
        })
    }
}

/// Create an encrypted backup file.
///
/// # Errors
/// - [`SentinelError::Io`] if the file cannot be written.
/// - [`SentinelError::Crypto`] if encryption fails.
pub fn create_backup(path: &Path, backup_password: &str, vault: &VaultPayload) -> Result<()> {
    let params = KdfParams::default();
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut salt).map_err(|_| SentinelError::Crypto)?;
    getrandom::getrandom(&mut nonce).map_err(|_| SentinelError::Crypto)?;

    let key = VaultKey::derive(backup_password.as_bytes(), &params, &salt)?;
    let payload = BackupPayload::from_vault(vault);
    let plaintext = cbor_encode(&payload)?;
    let ciphertext = encrypt(&key, &nonce, &plaintext)?;

    let header = BackupHeader {
        version: BACKUP_FORMAT_VERSION,
        kdf_params: params,
        salt,
        nonce,
    };
    let mut bytes = Vec::with_capacity(BACKUP_HEADER_LEN + ciphertext.len());
    bytes.extend_from_slice(&header.to_bytes());
    bytes.extend_from_slice(&ciphertext);
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Restore a backup file. Returns the decrypted backup payload.
///
/// # Errors
/// - [`SentinelError::Io`] if the file cannot be read.
/// - [`SentinelError::CorruptBackup`] if the file is corrupt.
/// - [`SentinelError::InvalidBackupPassword`] if the password is wrong.
pub fn restore_backup(path: &Path, backup_password: &str) -> Result<BackupPayload> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < BACKUP_HEADER_LEN + 16 {
        return Err(SentinelError::CorruptBackup);
    }
    let header = BackupHeader::from_bytes(&bytes[..BACKUP_HEADER_LEN])?;
    let ciphertext = &bytes[BACKUP_HEADER_LEN..];
    let key = VaultKey::derive(backup_password.as_bytes(), &header.kdf_params, &header.salt)?;
    let plaintext = decrypt(&key, &header.nonce, ciphertext)?;
    cbor_decode(&plaintext)
}

/// Preview a backup's contents (no secrets exposed).
///
/// # Errors
/// Same as [`restore_backup`].
pub fn preview_backup(path: &Path, backup_password: &str) -> Result<Vec<BackupPreviewEntry>> {
    let payload = restore_backup(path, backup_password)?;
    Ok(payload
        .accounts
        .iter()
        .map(|a| BackupPreviewEntry {
            issuer: a.issuer.clone(),
            label: a.label.clone(),
            otp_type: a.otp_type,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPreviewEntry {
    pub issuer: String,
    pub label: String,
    pub otp_type: crate::models::OtpType,
}

fn cbor_encode(payload: &BackupPayload) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(payload, &mut buf)
        .map_err(|e| SentinelError::Cbor(e.to_string()))?;
    Ok(buf)
}

fn cbor_decode(bytes: &[u8]) -> Result<BackupPayload> {
    ciborium::de::from_reader(bytes).map_err(|e| SentinelError::Cbor(e.to_string()))
}

fn encrypt(key: &VaultKey, nonce: &[u8; NONCE_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| SentinelError::Crypto)?;
    cipher
        .encrypt(nonce.into(), plaintext)
        .map_err(|_| SentinelError::Crypto)
}

fn decrypt(key: &VaultKey, nonce: &[u8; NONCE_LEN], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| SentinelError::Crypto)?;
    cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| SentinelError::InvalidBackupPassword)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Digits, OtpAlgorithm, OtpType, Secret};
    use crate::vault::KdfParams;
    use tempfile::tempdir;

    fn test_vault() -> VaultPayload {
        let mut v = VaultPayload::new_empty();
        let now = now_ms();
        v.accounts.push(AccountRecord {
            id: "acc-1".to_string(),
            issuer: "GitHub".to_string(),
            label: "alice@example.com".to_string(),
            secret: Secret::new(b"12345678901234567890".to_vec()),
            otp_type: OtpType::Totp,
            algorithm: OtpAlgorithm::Sha1,
            digits: Digits::Six,
            period: 30,
            counter: 0,
            tags: vec![],
            favorite: false,
            sort_position: 0,
            icon_color: None,
            icon_text: None,
            created_at: now,
            updated_at: now,
        });
        v
    }

    fn weak_params() -> KdfParams {
        KdfParams {
            m_cost_kib: 8,
            t_cost: 1,
            p_cost: 1,
        }
    }

    fn create_test_backup(path: &Path, password: &str, vault: &VaultPayload) -> Result<()> {
        let params = weak_params();
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::getrandom(&mut salt).map_err(|_| SentinelError::Crypto)?;
        getrandom::getrandom(&mut nonce).map_err(|_| SentinelError::Crypto)?;
        let key = VaultKey::derive(password.as_bytes(), &params, &salt)?;
        let payload = BackupPayload::from_vault(vault);
        let plaintext = cbor_encode(&payload)?;
        let ciphertext = encrypt(&key, &nonce, &plaintext)?;
        let header = BackupHeader {
            version: BACKUP_FORMAT_VERSION,
            kdf_params: params,
            salt,
            nonce,
        };
        let mut bytes = Vec::with_capacity(BACKUP_HEADER_LEN + ciphertext.len());
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&ciphertext);
        std::fs::write(path, bytes)?;
        Ok(())
    }

    #[test]
    fn backup_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        let vault = test_vault();
        create_test_backup(&path, "backup-pw", &vault).unwrap();
        let restored = restore_backup(&path, "backup-pw").unwrap();
        assert_eq!(restored.accounts.len(), 1);
        assert_eq!(restored.accounts[0].id, "acc-1");
        assert_eq!(restored.accounts[0].issuer, "GitHub");
        assert_eq!(
            restored.accounts[0].secret.as_bytes(),
            b"12345678901234567890"
        );
    }

    #[test]
    fn wrong_backup_password_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        let vault = test_vault();
        create_test_backup(&path, "correct", &vault).unwrap();
        let result = restore_backup(&path, "WRONG");
        assert!(matches!(result, Err(SentinelError::InvalidBackupPassword)));
    }

    #[test]
    fn tampered_backup_detected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        let vault = test_vault();
        create_test_backup(&path, "pw", &vault).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        std::fs::write(&path, bytes).unwrap();
        let result = restore_backup(&path, "pw");
        assert!(matches!(result, Err(SentinelError::InvalidBackupPassword)));
    }

    #[test]
    fn corrupt_header_detected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        std::fs::write(
            &path,
            b"NOTBACK0xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        )
        .unwrap();
        let result = restore_backup(&path, "pw");
        assert!(matches!(result, Err(SentinelError::CorruptBackup)));
    }

    #[test]
    fn truncated_backup_detected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        let vault = test_vault();
        create_test_backup(&path, "pw", &vault).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.truncate(20);
        std::fs::write(&path, bytes).unwrap();
        let result = restore_backup(&path, "pw");
        assert!(matches!(result, Err(SentinelError::CorruptBackup)));
    }

    #[test]
    fn backup_independent_from_vault_password() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        let vault = test_vault();
        create_test_backup(&path, "backup-pw", &vault).unwrap();
        let restored = restore_backup(&path, "backup-pw").unwrap();
        assert_eq!(restored.accounts.len(), 1);
    }

    #[test]
    fn preview_backup_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup.sentinelbak");
        let vault = test_vault();
        create_test_backup(&path, "pw", &vault).unwrap();
        let preview = preview_backup(&path, "pw").unwrap();
        assert_eq!(preview.len(), 1);
        assert_eq!(preview[0].issuer, "GitHub");
        assert_eq!(preview[0].label, "alice@example.com");
    }

    #[test]
    fn backup_to_vault_payload_round_trip() {
        let vault = test_vault();
        let backup = BackupPayload::from_vault(&vault);
        let restored_vault = backup.into_vault_payload();
        assert_eq!(restored_vault.accounts.len(), vault.accounts.len());
        assert_eq!(restored_vault.accounts[0].id, vault.accounts[0].id);
        assert_eq!(
            restored_vault.accounts[0].secret.as_bytes(),
            vault.accounts[0].secret.as_bytes()
        );
    }
}
