//! Vault file format and Argon2id KDF parameters.
//!
//! ## File layout (v1)
//!
//! ```text
//! Offset  Size      Field
//! ------  --------  ----------------------------------------------------
//!   0     8 bytes   Magic: "SENTINEL1" (ASCII, no NUL)
//!   8     2 bytes   Format version (u16 little-endian) — currently 1
//!  10     1 byte    KDF algorithm ID (1 = Argon2id)
//!  11     4 bytes   Argon2id m_cost (u32 LE, in KiB)
//!  15     4 bytes   Argon2id t_cost (u32 LE, iterations)
//!  19     4 bytes   Argon2id p_cost (u32 LE, parallelism)
//!  23    16 bytes   Salt (random)
//!  39    12 bytes   AES-256-GCM nonce (random)
//!  51     N bytes   AES-256-GCM ciphertext (account_list_cbor + 16-byte tag)
//! ```
//!
//! ## Cryptography
//!
//! - **KDF**: Argon2id with m=64 MiB, t=3, p=4 (OWASP-recommended minimum
//!   as of 2024). Parameters are stored in the file header so future
//!   versions can tune them without breaking older vaults.
//! - **AEAD**: AES-256-GCM with a 12-byte random nonce per encryption.
//!   The GCM tag (16 bytes) is appended to the ciphertext by the `aes-gcm`
//!   crate and verified on decryption — this provides tamper detection.
//! - **Salt**: 16 bytes from the OS CSPRNG (`getrandom`). Unique per vault.
//! - **Nonce**: 12 bytes from the OS CSPRNG. Unique per encryption operation
//!   (a new nonce is generated every time the vault is saved).
//!
//! ## What is NOT in the file
//!
//! - No plaintext metadata (issuer names, account labels, etc.) — everything
//!   is inside the encrypted blob.
//! - No master-password verifier. Including one would let an attacker who
//!   obtains the vault file test password guesses offline at high speed
//!   without the Argon2id cost. The only way to verify a password is to
//!   attempt the full Argon2id + AES-GCM decryption, which is intentionally
//!   expensive.
//! - No version of the encrypted payload's schema inside the plaintext.
//!   The schema version is part of the CBOR payload itself, so it's
//!   encrypted along with everything else.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SentinelError};

/// Current vault file format version.
pub const VAULT_FORMAT_VERSION: u16 = 1;

/// Magic bytes that identify a Sentinel vault file.
///
/// We use exactly 8 bytes: "SENTINL1". The "1" suffix indicates version 1
/// of the magic itself; the version field at offset 8 carries the actual
/// file format version.
pub const VAULT_MAGIC: &[u8; 8] = b"SENTINL1";

/// KDF algorithm IDs stored in the vault header.
pub const KDF_ID_ARGON2ID: u8 = 1;

/// Default Argon2id parameters. Tunable per-installation; stored in the
/// file header so older vaults keep their original parameters.
///
/// - `m_cost = 65_536` KiB = 64 MiB memory
/// - `t_cost = 3` iterations
/// - `p_cost = 4` parallel lanes
///
/// These match OWASP's 2024 recommendation for interactive password hashing.
/// On a typical 2024-era laptop, deriving a key takes ~150–300ms — fast
/// enough for an interactive unlock, slow enough to make brute force painful.
pub const DEFAULT_ARGON2_M_COST_KIB: u32 = 65_536;
pub const DEFAULT_ARGON2_T_COST: u32 = 3;
pub const DEFAULT_ARGON2_P_COST: u32 = 4;

/// Salt length in bytes. 16 bytes (128 bits) is more than sufficient for
/// a per-vault salt — the salt's only job is to ensure that two vaults
/// with the same master password produce different keys.
pub const SALT_LEN: usize = 16;

/// AES-256-GCM nonce length. 12 bytes (96 bits) is the standard GCM nonce
/// size and the only size the NIST spec defines for GCM.
pub const NONCE_LEN: usize = 12;

/// AES-256 key length in bytes.
pub const KEY_LEN: usize = 32;

/// Header length in bytes (everything before the ciphertext).
pub const HEADER_LEN: usize = 8 + 2 + 1 + 4 + 4 + 4 + SALT_LEN + NONCE_LEN; // = 51

/// KDF parameters used to derive the vault key from the master password.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_cost_kib: DEFAULT_ARGON2_M_COST_KIB,
            t_cost: DEFAULT_ARGON2_T_COST,
            p_cost: DEFAULT_ARGON2_P_COST,
        }
    }
}

/// Parsed vault header. The ciphertext is held separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultHeader {
    pub version: u16,
    pub kdf_id: u8,
    pub kdf_params: KdfParams,
    pub salt: [u8; SALT_LEN],
    pub nonce: [u8; NONCE_LEN],
}

impl VaultHeader {
    /// Serialize the header to a 51-byte buffer.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..8].copy_from_slice(VAULT_MAGIC);
        buf[8..10].copy_from_slice(&self.version.to_le_bytes());
        buf[10] = self.kdf_id;
        buf[11..15].copy_from_slice(&self.kdf_params.m_cost_kib.to_le_bytes());
        buf[15..19].copy_from_slice(&self.kdf_params.t_cost.to_le_bytes());
        buf[19..23].copy_from_slice(&self.kdf_params.p_cost.to_le_bytes());
        buf[23..23 + SALT_LEN].copy_from_slice(&self.salt);
        buf[23 + SALT_LEN..23 + SALT_LEN + NONCE_LEN].copy_from_slice(&self.nonce);
        buf
    }

    /// Parse a header from a 51-byte buffer.
    ///
    /// # Errors
    /// Returns [`SentinelError::CorruptVault`] if the magic is wrong.
    /// Returns [`SentinelError::UnsupportedVaultVersion`] if the version is
    /// not 1.
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < HEADER_LEN {
            return Err(SentinelError::CorruptVault);
        }
        if &buf[0..8] != VAULT_MAGIC {
            return Err(SentinelError::CorruptVault);
        }
        let version = u16::from_le_bytes([buf[8], buf[9]]);
        if version != VAULT_FORMAT_VERSION {
            return Err(SentinelError::UnsupportedVaultVersion(
                VAULT_FORMAT_VERSION,
                version,
            ));
        }
        let kdf_id = buf[10];
        if kdf_id != KDF_ID_ARGON2ID {
            return Err(SentinelError::CorruptVault);
        }
        let m_cost_kib = u32::from_le_bytes([buf[11], buf[12], buf[13], buf[14]]);
        let t_cost = u32::from_le_bytes([buf[15], buf[16], buf[17], buf[18]]);
        let p_cost = u32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]);
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&buf[23..23 + SALT_LEN]);
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&buf[23 + SALT_LEN..23 + SALT_LEN + NONCE_LEN]);

        Ok(Self {
            version,
            kdf_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SentinelError;

    #[test]
    fn header_round_trip() {
        let header = VaultHeader {
            version: 1,
            kdf_id: KDF_ID_ARGON2ID,
            kdf_params: KdfParams::default(),
            salt: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            nonce: [0xa1; NONCE_LEN],
        };
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), HEADER_LEN);
        let parsed = VaultHeader::from_bytes(&bytes).expect("round trip");
        assert_eq!(parsed, header);
    }

    #[test]
    fn header_rejects_wrong_magic() {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..8].copy_from_slice(b"NOTSENT1");
        let result = VaultHeader::from_bytes(&bytes);
        assert!(matches!(result, Err(SentinelError::CorruptVault)));
    }

    #[test]
    fn header_rejects_unsupported_version() {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..8].copy_from_slice(VAULT_MAGIC);
        bytes[8..10].copy_from_slice(&99u16.to_le_bytes()); // unsupported version
        bytes[10] = KDF_ID_ARGON2ID;
        let result = VaultHeader::from_bytes(&bytes);
        assert!(matches!(
            result,
            Err(SentinelError::UnsupportedVaultVersion(1, 99))
        ));
    }

    #[test]
    fn header_rejects_unknown_kdf() {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..8].copy_from_slice(VAULT_MAGIC);
        bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
        bytes[10] = 99; // unknown KDF
        let result = VaultHeader::from_bytes(&bytes);
        assert!(matches!(result, Err(SentinelError::CorruptVault)));
    }

    #[test]
    fn header_rejects_short_buffer() {
        let bytes = [0u8; 10];
        let result = VaultHeader::from_bytes(&bytes);
        assert!(matches!(result, Err(SentinelError::CorruptVault)));
    }

    #[test]
    fn magic_is_exactly_8_bytes() {
        assert_eq!(VAULT_MAGIC.len(), 8);
    }

    #[test]
    fn header_len_matches_spec() {
        // 8 (magic) + 2 (version) + 1 (kdf id) + 4+4+4 (params) + 16 (salt) + 12 (nonce)
        assert_eq!(HEADER_LEN, 51);
    }
}
