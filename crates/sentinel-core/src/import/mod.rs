//! Account import and manual creation.
//!
//! Submodules:
//!  - `otpauth` — parses `otpauth://` URIs (the standard QR-code payload
//!    for TOTP/HOTP accounts).
//!  - `manual` — validates user-supplied manual account creation input.
//!  - `migration` — decodes Google Authenticator `otpauth-migration://`
//!    transfer payloads, including multi-batch transfers.

pub mod manual;
pub mod migration;
pub mod otpauth;

pub use manual::validate_manual_account;
pub use migration::{merge_batches, parse_migration_uri, MigrationBatch};
pub use otpauth::{parse_otpauth_uri, ParsedOtpauth};
