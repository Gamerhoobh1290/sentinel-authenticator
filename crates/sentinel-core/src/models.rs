//! Data models shared between the core library and the Tauri frontend.
//!
//! These types deliberately do NOT include raw secret bytes — the secret
//! lives only inside the vault's encrypted blob and is materialised into
//! memory only when an OTP code needs to be generated.

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// OTP algorithm. Wire-compatible with the Google migration enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl OtpAlgorithm {
    /// Returns the block size in bytes for the underlying hash.
    #[must_use]
    pub const fn block_size(self) -> usize {
        match self {
            Self::Sha1 | Self::Sha256 => 64,
            Self::Sha512 => 128,
        }
    }

    /// Returns the output size in bytes for the underlying hash.
    #[must_use]
    pub const fn output_size(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }
}

impl std::fmt::Display for OtpAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sha1 => write!(f, "SHA1"),
            Self::Sha256 => write!(f, "SHA256"),
            Self::Sha512 => write!(f, "SHA512"),
        }
    }
}

/// OTP type — RFC 4226 (HOTP) or RFC 6238 (TOTP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtpType {
    Totp,
    Hotp,
}

impl std::fmt::Display for OtpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Totp => write!(f, "TOTP"),
            Self::Hotp => write!(f, "HOTP"),
        }
    }
}

/// Code length. RFC 4226 mandates 6-8 digits; we restrict to the
/// commonly-used 6 and 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Digits {
    Six,
    Eight,
}

impl Digits {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::Six => 6,
            Self::Eight => 8,
        }
    }

    /// Parse a digit count from a u32. Returns `None` for unsupported values.
    #[must_use]
    pub const fn from_u32(n: u32) -> Option<Self> {
        match n {
            6 => Some(Self::Six),
            8 => Some(Self::Eight),
            _ => None,
        }
    }
}

impl OtpAlgorithm {
    /// Parse an algorithm name from a string (case-insensitive).
    /// Accepts "sha1", "sha256", "sha512" (with or without dash, e.g. "sha-1").
    #[must_use]
    pub fn from_str_ci(s: &str) -> Option<Self> {
        let normalized = s.to_ascii_lowercase().replace(['-', '_'], "");
        match normalized.as_str() {
            "sha1" | "sha" => Some(Self::Sha1),
            "sha256" => Some(Self::Sha256),
            "sha512" => Some(Self::Sha512),
            _ => None,
        }
    }
}

/// A decrypted OTP account record. Held in memory only while the vault is
/// unlocked. Dropping this value zeroizes the secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    /// Unique internal ID (UUID v4). Never derived from account data.
    pub id: String,
    pub issuer: String,
    pub label: String,
    /// Raw secret bytes. Will be zeroized on drop via the Secret wrapper.
    pub secret: Secret,
    pub otp_type: OtpType,
    pub algorithm: OtpAlgorithm,
    pub digits: Digits,
    /// TOTP period in seconds (defaults to 30). Ignored for HOTP.
    pub period: u32,
    /// HOTP counter. Ignored for TOTP.
    pub counter: u64,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub sort_position: u32,
    pub icon_color: Option<String>,
    pub icon_text: Option<String>,
    pub created_at: u64, // ms since epoch
    pub updated_at: u64,
}

/// Newtype wrapper that zeroizes the inner bytes on drop.
/// Construct via `Secret::new(...)`. Read via `secret.as_bytes()`.
#[derive(Clone)]
pub struct Secret {
    inner: Vec<u8>,
}

impl PartialEq for Secret {
    fn eq(&self, other: &Self) -> bool {
        // Constant-time compare via the `subtle` crate to avoid timing leaks
        // when comparing secrets. (We expose this only for tests/in-memory
        // equality, never across the IPC boundary.)
        subtle::ConstantTimeEq::ct_eq(&self.inner[..], &other.inner[..]).into()
    }
}

impl Eq for Secret {}

impl Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a byte array. CBOR handles this efficiently.
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(Self::new(bytes))
    }
}

impl Secret {
    /// Take ownership of the given bytes; they will be zeroized when this
    /// `Secret` is dropped.
    #[must_use]
    #[allow(clippy::missing_const_for_fn, clippy::missing_errors_doc)]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { inner: bytes }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret value — not even in debug output.
        f.debug_struct("Secret")
            .field("len", &self.inner.len())
            .finish()
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

impl Default for AccountRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            issuer: String::new(),
            label: String::new(),
            secret: Secret::new(Vec::new()),
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
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// Sanitized view sent to the frontend. The secret is intentionally absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountView {
    pub id: String,
    pub issuer: String,
    pub label: String,
    pub otp_type: OtpType,
    pub algorithm: OtpAlgorithm,
    pub digits: Digits,
    pub period: u32,
    pub counter: u64,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub sort_position: u32,
    pub icon_color: Option<String>,
    pub icon_text: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl From<&AccountRecord> for AccountView {
    fn from(r: &AccountRecord) -> Self {
        Self {
            id: r.id.clone(),
            issuer: r.issuer.clone(),
            label: r.label.clone(),
            otp_type: r.otp_type,
            algorithm: r.algorithm,
            digits: r.digits,
            period: r.period,
            counter: r.counter,
            tags: r.tags.clone(),
            favorite: r.favorite,
            sort_position: r.sort_position,
            icon_color: r.icon_color.clone(),
            icon_text: r.icon_text.clone(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Result of generating a one-time code. Never persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeResult {
    pub account_id: String,
    pub code: String,
    /// TOTP only: seconds remaining in the current period.
    pub seconds_remaining: Option<u32>,
    /// TOTP only: the period (seconds).
    pub period: Option<u32>,
}
