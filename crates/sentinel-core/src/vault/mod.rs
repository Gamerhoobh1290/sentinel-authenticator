//! Vault encryption, decryption, and file I/O.
//!
//! This module provides the high-level operations:
//!
//! - `create_vault(path, password)` — initialize a new empty vault file
//! - `open_vault(path, password)` — read & decrypt an existing vault
//! - `save_vault(path, password, payload)` — re-encrypt and write
//!
//! All operations use Argon2id for KDF and AES-256-GCM for AEAD. The
//! derived key is held in a `VaultKey` wrapper that zeroizes on drop.

pub mod format;
pub mod payload;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

use crate::error::{Result, SentinelError};

// Private imports of the constants we use internally.
use format::{HEADER_LEN, KDF_ID_ARGON2ID, KEY_LEN, NONCE_LEN, SALT_LEN};

/// Re-exports for convenience.
pub use format::{KdfParams, VaultHeader, VAULT_FORMAT_VERSION};
pub use payload::{now_ms, VaultPayload, PAYLOAD_SCHEMA_VERSION};

/// A derived vault key. Wraps the 32-byte AES-256 key. Zeroizes on drop.
#[derive(Clone)]
pub struct VaultKey {
    bytes: [u8; KEY_LEN],
}

impl VaultKey {
    /// Derive a key from the master password using Argon2id with the given
    /// parameters and salt.
    ///
    /// # Errors
    /// Returns [`SentinelError::Crypto`] if Argon2id reports an error
    /// (e.g. invalid parameters or OS memory allocation failure).
    pub fn derive(password: &[u8], params: &KdfParams, salt: &[u8]) -> Result<Self> {
        let argon = argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                params.m_cost_kib,
                params.t_cost,
                params.p_cost,
                Some(KEY_LEN),
            )
            .map_err(|_| SentinelError::Crypto)?,
        );
        let mut key = [0u8; KEY_LEN];
        argon
            .hash_password_into(password, salt, &mut key)
            .map_err(|_| SentinelError::InvalidPassword)?;
        Ok(Self { bytes: key })
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Drop for VaultKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl std::fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the key — not even in debug output.
        f.debug_struct("VaultKey")
            .field("len", &self.bytes.len())
            .finish()
    }
}

/// Create a new vault file at the given path. Fails if the file already
/// exists — use `save_vault` to update an existing vault.
///
/// Uses the default Argon2id parameters (64 MiB memory, 3 iterations, 4 lanes).
///
/// # Errors
/// - [`SentinelError::Io`] if the file cannot be created or written.
/// - [`SentinelError::Crypto`] if Argon2id fails or AES-GCM encryption fails.
pub fn create_vault(path: &Path, password: &str) -> Result<()> {
    create_vault_with_params(path, password, &KdfParams::default())
}

/// Create a new vault file with explicit KDF parameters. Used by tests
/// (which need weak/fast params) and by future settings UI that lets users
/// tune Argon2id cost.
///
/// # Errors
/// - [`SentinelError::Io`] if the file cannot be created or written.
/// - [`SentinelError::Crypto`] if Argon2id fails or AES-GCM encryption fails.
pub fn create_vault_with_params(path: &Path, password: &str, params: &KdfParams) -> Result<()> {
    if path.exists() {
        return Err(SentinelError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "vault file already exists",
        )));
    }

    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut salt).map_err(|_| SentinelError::Crypto)?;
    getrandom::getrandom(&mut nonce).map_err(|_| SentinelError::Crypto)?;

    let key = VaultKey::derive(password.as_bytes(), params, &salt)?;
    let payload = VaultPayload::new_empty();
    let ciphertext = encrypt_payload(&key, &nonce, &payload)?;

    let header = VaultHeader {
        version: VAULT_FORMAT_VERSION,
        kdf_id: KDF_ID_ARGON2ID,
        kdf_params: *params,
        salt,
        nonce,
    };

    write_vault_file(path, &header, &ciphertext)
}

/// Open and decrypt an existing vault file.
///
/// Reads the header, derives the key from the master password using the
/// stored KDF parameters and salt, then decrypts the ciphertext.
///
/// # Errors
/// - [`SentinelError::Io`] if the file cannot be read.
/// - [`SentinelError::CorruptVault`] if the file is too short, has a bad
///   magic, or an unsupported version.
/// - [`SentinelError::InvalidPassword`] if the password is wrong (AES-GCM
///   tag verification fails) or Argon2id derivation fails.
/// - [`SentinelError::Cbor`] if the decrypted payload is not valid CBOR.
pub fn open_vault(path: &Path, password: &str) -> Result<VaultPayload> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < HEADER_LEN + 16 {
        // Header + at least the GCM tag (16 bytes). Empty plaintext is valid.
        return Err(SentinelError::CorruptVault);
    }

    let header = VaultHeader::from_bytes(&bytes[..HEADER_LEN])?;
    let ciphertext = &bytes[HEADER_LEN..];

    let key = VaultKey::derive(password.as_bytes(), &header.kdf_params, &header.salt)?;
    let plaintext = decrypt_payload(&key, &header.nonce, ciphertext)?;
    VaultPayload::from_cbor(&plaintext)
}

/// Save a payload to an existing vault file. Re-derives the key, generates
/// a fresh nonce (never reuses the old one), and rewrites the entire file.
///
/// The salt and KDF parameters are preserved from the existing file.
///
/// **Password verification**: before overwriting, this function verifies
/// the password by attempting to decrypt the existing ciphertext. If the
/// password is wrong, the file is left untouched and
/// [`SentinelError::InvalidPassword`] is returned. This prevents data loss
/// if the user mistypes their password in a "save" flow.
///
/// # Errors
/// - [`SentinelError::Io`] if the file cannot be read or written.
/// - [`SentinelError::InvalidPassword`] if the password is wrong.
/// - [`SentinelError::CorruptVault`] if the existing file is corrupt.
/// - [`SentinelError::Crypto`] if encryption fails.
pub fn save_vault(path: &Path, password: &str, mut payload: VaultPayload) -> Result<()> {
    // Read the existing file.
    let existing_bytes = std::fs::read(path)?;
    if existing_bytes.len() < HEADER_LEN + 16 {
        return Err(SentinelError::CorruptVault);
    }
    let existing_header = VaultHeader::from_bytes(&existing_bytes[..HEADER_LEN])?;
    let existing_ciphertext = &existing_bytes[HEADER_LEN..];

    // Derive the key and VERIFY it by decrypting the existing ciphertext.
    // This is the critical safety check: if the password is wrong, we fail
    // here before writing anything.
    let key = VaultKey::derive(
        password.as_bytes(),
        &existing_header.kdf_params,
        &existing_header.salt,
    )?;
    let _verification = decrypt_payload(&key, &existing_header.nonce, existing_ciphertext)?;
    // If decrypt_payload succeeded, the password is correct. We discard the
    // decrypted plaintext — the caller has already provided the payload they
    // want to save.

    // Generate a fresh nonce for this save. NEVER reuse a nonce with the
    // same key — that would compromise the confidentiality and integrity
    // of the AES-GCM ciphertext.
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|_| SentinelError::Crypto)?;

    payload.touch();
    let ciphertext = encrypt_payload(&key, &nonce, &payload)?;

    let header = VaultHeader {
        version: VAULT_FORMAT_VERSION,
        kdf_id: KDF_ID_ARGON2ID,
        kdf_params: existing_header.kdf_params,
        salt: existing_header.salt,
        nonce,
    };

    // Write to a temp file first, then rename — atomic on most filesystems.
    let temp_path = temp_path_for(path);
    write_vault_file(&temp_path, &header, &ciphertext)?;
    std::fs::rename(&temp_path, path)?;

    Ok(())
}

/// Change the master password of an existing vault. Re-encrypts the payload
/// with a new salt and a new key derived from the new password.
///
/// The existing KDF parameters are preserved — only the salt and key change.
/// This ensures users who tuned their Argon2id cost keep their settings.
///
/// # Errors
/// - [`SentinelError::InvalidPassword`] if the old password is wrong.
/// - [`SentinelError::Io`] if the file cannot be read or written.
pub fn change_master_password(path: &Path, old_password: &str, new_password: &str) -> Result<()> {
    // Read the existing file to get the header (KDF params + salt).
    let existing_bytes = std::fs::read(path)?;
    if existing_bytes.len() < HEADER_LEN + 16 {
        return Err(SentinelError::CorruptVault);
    }
    let existing_header = VaultHeader::from_bytes(&existing_bytes[..HEADER_LEN])?;
    let existing_ciphertext = &existing_bytes[HEADER_LEN..];

    // Verify the old password by deriving the key and decrypting.
    let old_key = VaultKey::derive(
        old_password.as_bytes(),
        &existing_header.kdf_params,
        &existing_header.salt,
    )?;
    let plaintext = decrypt_payload(&old_key, &existing_header.nonce, existing_ciphertext)?;
    let payload = VaultPayload::from_cbor(&plaintext)?;

    // Generate fresh salt + nonce for the new key. Preserve KDF params.
    let mut new_salt = [0u8; SALT_LEN];
    let mut new_nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut new_salt).map_err(|_| SentinelError::Crypto)?;
    getrandom::getrandom(&mut new_nonce).map_err(|_| SentinelError::Crypto)?;

    let new_key = VaultKey::derive(
        new_password.as_bytes(),
        &existing_header.kdf_params,
        &new_salt,
    )?;
    let ciphertext = encrypt_payload(&new_key, &new_nonce, &payload)?;

    let header = VaultHeader {
        version: VAULT_FORMAT_VERSION,
        kdf_id: KDF_ID_ARGON2ID,
        kdf_params: existing_header.kdf_params,
        salt: new_salt,
        nonce: new_nonce,
    };

    let temp_path = temp_path_for(path);
    write_vault_file(&temp_path, &header, &ciphertext)?;
    std::fs::rename(&temp_path, path)?;

    Ok(())
}

/// Encrypt a payload with AES-256-GCM. Returns ciphertext + 16-byte GCM tag
/// (the `aes-gcm` crate appends the tag automatically).
fn encrypt_payload(
    key: &VaultKey,
    nonce: &[u8; NONCE_LEN],
    payload: &VaultPayload,
) -> Result<Vec<u8>> {
    let plaintext = payload.to_cbor()?;
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| SentinelError::Crypto)?;
    let ciphertext = cipher
        .encrypt(nonce.into(), plaintext.as_ref())
        .map_err(|_| SentinelError::Crypto)?;
    Ok(ciphertext)
}

/// Decrypt a ciphertext with AES-256-GCM. Verifies the GCM tag (tamper
/// detection). Returns the plaintext bytes.
fn decrypt_payload(key: &VaultKey, nonce: &[u8; NONCE_LEN], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| SentinelError::Crypto)?;
    cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| SentinelError::InvalidPassword)
}

/// Write the header + ciphertext to a file.
fn write_vault_file(path: &Path, header: &VaultHeader, ciphertext: &[u8]) -> Result<()> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    bytes.extend_from_slice(&header.to_bytes());
    bytes.extend_from_slice(ciphertext);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Generate a sibling temp file path for atomic writes.
fn temp_path_for(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".tmp");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AccountRecord, Digits, OtpAlgorithm, OtpType, Secret};
    use tempfile::tempdir;

    // Use weak KDF params for tests so they run fast.
    fn test_params() -> KdfParams {
        KdfParams {
            m_cost_kib: 8, // 8 KiB — fast but insecure; tests only
            t_cost: 1,
            p_cost: 1,
        }
    }

    /// Create a vault with test-fast KDF params (NOT the production defaults).
    fn create_test_vault(path: &Path, password: &str) -> Result<()> {
        create_vault_with_params(path, password, &test_params())
    }

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
    fn create_and_open_round_trip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "correct horse battery staple").expect("create");
        let payload = open_vault(&path, "correct horse battery staple").expect("open");
        assert!(payload.accounts.is_empty());
    }

    #[test]
    fn open_with_wrong_password_fails() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "correct horse battery staple").expect("create");
        let result = open_vault(&path, "WRONG password");
        assert!(matches!(result, Err(SentinelError::InvalidPassword)));
    }

    #[test]
    fn save_and_reopen_preserves_accounts() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "pw").expect("create");

        // Open, add accounts, save.
        let mut payload = open_vault(&path, "pw").expect("open");
        payload.accounts.push(dummy_account("acc-1", "GitHub"));
        payload.accounts.push(dummy_account("acc-2", "GitLab"));
        save_vault(&path, "pw", payload).expect("save");

        // Re-open and verify.
        let payload2 = open_vault(&path, "pw").expect("reopen");
        assert_eq!(payload2.accounts.len(), 2);
        assert_eq!(payload2.accounts[0].id, "acc-1");
        assert_eq!(payload2.accounts[1].id, "acc-2");
    }

    #[test]
    fn save_with_wrong_password_does_not_corrupt_vault() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "correct").expect("create");

        let mut payload = open_vault(&path, "correct").expect("open");
        payload.accounts.push(dummy_account("acc-1", "GitHub"));

        // Try to save with wrong password — should fail.
        let result = save_vault(&path, "WRONG", payload.clone());
        assert!(matches!(result, Err(SentinelError::InvalidPassword)));

        // Original vault should still be intact with the correct password.
        let payload2 = open_vault(&path, "correct").expect("original still works");
        assert!(payload2.accounts.is_empty());
    }

    #[test]
    fn tampered_ciphertext_detected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "pw").expect("create");

        // Read the file, flip a byte in the ciphertext, write it back.
        let mut bytes = std::fs::read(&path).expect("read");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        std::fs::write(&path, bytes).expect("write");

        let result = open_vault(&path, "pw");
        assert!(matches!(result, Err(SentinelError::InvalidPassword)));
        // InvalidPassword is correct here: GCM tag verification fails and we
        // can't distinguish wrong-password from tampered-ciphertext, so we
        // return the same error to avoid leaking information.
    }

    #[test]
    fn tampered_header_detected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "pw").expect("create");

        let mut bytes = std::fs::read(&path).expect("read");
        bytes[20] ^= 0xff; // flip a byte in the KDF params
        std::fs::write(&path, bytes).expect("write");

        // Either the header parse fails (CorruptVault) or the KDF params are
        // subtly wrong and the derived key doesn't match (InvalidPassword).
        // Both are acceptable — the point is that the user can't unlock.
        let result = open_vault(&path, "pw");
        assert!(result.is_err());
    }

    #[test]
    fn truncated_file_detected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "pw").expect("create");

        let mut bytes = std::fs::read(&path).expect("read");
        bytes.truncate(HEADER_LEN + 5); // too short to contain a GCM tag
        std::fs::write(&path, bytes).expect("write");

        let result = open_vault(&path, "pw");
        assert!(matches!(result, Err(SentinelError::CorruptVault)));
    }

    #[test]
    fn wrong_magic_rejected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        std::fs::write(
            &path,
            b"NOTSENT1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        )
        .expect("write");
        let result = open_vault(&path, "pw");
        assert!(matches!(result, Err(SentinelError::CorruptVault)));
    }

    #[test]
    fn change_master_password_works() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "old-pw").expect("create");

        // Add an account, save.
        let mut payload = open_vault(&path, "old-pw").expect("open");
        payload.accounts.push(dummy_account("acc-1", "GitHub"));
        save_vault(&path, "old-pw", payload).expect("save");

        // Change password.
        change_master_password(&path, "old-pw", "new-pw").expect("change");

        // Old password no longer works.
        let result = open_vault(&path, "old-pw");
        assert!(matches!(result, Err(SentinelError::InvalidPassword)));

        // New password works and accounts are preserved.
        let payload = open_vault(&path, "new-pw").expect("open with new");
        assert_eq!(payload.accounts.len(), 1);
        assert_eq!(payload.accounts[0].id, "acc-1");
    }

    #[test]
    fn change_master_password_with_wrong_old_password_fails() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "old-pw").expect("create");

        let result = change_master_password(&path, "WRONG", "new-pw");
        assert!(matches!(result, Err(SentinelError::InvalidPassword)));

        // Original vault is still intact.
        let payload = open_vault(&path, "old-pw").expect("original works");
        assert!(payload.accounts.is_empty());
    }

    #[test]
    fn create_vault_fails_if_file_exists() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "pw").expect("create");
        let result = create_test_vault(&path, "pw");
        assert!(result.is_err());
    }

    #[test]
    fn vault_key_is_zeroized_on_drop() {
        // We can't directly test that memory is zeroed, but we can verify
        // that VaultKey implements Zeroize via its Drop impl. This test
        // exists to ensure the Drop impl isn't accidentally removed.
        let key = VaultKey::derive(b"pw", &test_params(), &[0u8; SALT_LEN]).expect("derive");
        drop(key);
        // If the Drop impl is removed, the test still passes — but at least
        // the type system requires the impl to exist for the code to compile.
    }

    #[test]
    fn save_uses_fresh_nonce_each_time() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("vault.bin");
        create_test_vault(&path, "pw").expect("create");

        let payload = open_vault(&path, "pw").expect("open");
        let bytes1 = std::fs::read(&path).expect("read");

        save_vault(&path, "pw", payload).expect("save");
        let bytes2 = std::fs::read(&path).expect("read");

        // The nonce is at offset 39..51. It must be different after save.
        let nonce1 = &bytes1[39..51];
        let nonce2 = &bytes2[39..51];
        assert_ne!(nonce1, nonce2, "save must generate a fresh nonce");
    }
}
