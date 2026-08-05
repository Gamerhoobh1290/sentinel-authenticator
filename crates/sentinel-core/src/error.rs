//! Sentinel error types.
//!
//! Errors are deliberately opaque — they never include secrets, raw bytes,
//! or stack-trace material. Error messages are safe to surface to the user
//! and to logs.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SentinelError {
    #[error("Invalid master password")]
    InvalidPassword,

    #[error("Vault file is corrupted or tampered with")]
    CorruptVault,

    #[error("Vault file format is unsupported (expected version {0}, found {1})")]
    UnsupportedVaultVersion(u16, u16),

    #[error("Vault has not been initialised — create one first")]
    VaultNotInitialised,

    #[error("Vault is locked — unlock required")]
    VaultLocked,

    #[error("Backup file is corrupted or tampered with")]
    CorruptBackup,

    #[error("Invalid backup password")]
    InvalidBackupPassword,

    #[error("Backup file format is unsupported (expected version {0}, found {1})")]
    UnsupportedBackupVersion(u16, u16),

    #[error("Invalid secret: {0}")]
    InvalidSecret(String),

    #[error("Invalid otpauth URI: {0}")]
    InvalidOtpauthUri(String),

    #[error("Unsupported QR payload: {0}")]
    UnsupportedQrPayload(String),

    #[error("Malformed Google migration payload: {0}")]
    MalformedMigration(String),

    #[error("Google migration batch is incomplete — scanned {scanned} of {total} QR codes")]
    IncompleteMigrationBatch { scanned: usize, total: usize },

    #[error("Payload too large (max {max} bytes, got {got} bytes)")]
    PayloadTooLarge { max: usize, got: usize },

    #[error("Path traversal detected in: {0}")]
    PathTraversal(String),

    #[error("Account already exists in the vault")]
    DuplicateAccount,

    #[error("Cryptography error")]
    Crypto,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Encoding error: {0}")]
    Encoding(#[from] data_encoding::DecodeError),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("CBOR (de)serialization error: {0}")]
    Cbor(String),

    #[error("Protobuf decode error: {0}")]
    Prost(#[from] prost::DecodeError),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

pub type Result<T> = std::result::Result<T, SentinelError>;
