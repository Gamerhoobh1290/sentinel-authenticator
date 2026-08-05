//! Sentinel core library.
//!
//! Independent of Tauri so the security-sensitive cryptography, OTP, vault,
//! import, and backup logic can be unit-tested in CI without requiring
//! platform UI dependencies (`WebView2` on Windows, `webkit2gtk` on Linux).
//!
//! The Tauri app in `/src-tauri` depends on this crate via a path dependency
//! and exposes selected functions as `#[tauri::command]` handlers.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_inception)]

pub mod backup;
pub mod error;
pub mod import;
pub mod models;
pub mod otp;
pub mod proto;
pub mod redact;
pub mod vault;
