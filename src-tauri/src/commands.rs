//! Tauri IPC command handlers.
//!
//! Each command is a thin wrapper around the relevant backend module.
//! Commands never return raw secrets except where the user has explicitly
//! re-authenticated (the reveal-secret command, added in M7).

use std::path::PathBuf;
use std::sync::Mutex;

use sentinel_core::backup::{create_backup, preview_backup, restore_backup, BackupPreviewEntry};
use sentinel_core::import::{parse_migration_uri, parse_otpauth_uri, validate_manual_account};
use sentinel_core::models::{AccountView, CodeResult, Digits, OtpAlgorithm, OtpType};
use sentinel_core::otp::generate_totp;
use sentinel_core::vault::{
    change_master_password, create_vault, open_vault, save_vault, VaultPayload,
};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

/// Trivial health-check command.
#[tauri::command]
pub fn ping() -> &'static str {
    "sentinel-ok"
}

/// Static application metadata returned to the frontend.
#[derive(Debug, Serialize)]
pub struct AppMeta {
    pub name: &'static str,
    pub version: &'static str,
    pub platform: String,
}

#[tauri::command]
pub fn app_meta() -> AppMeta {
    AppMeta {
        name: "sentinel-authenticator",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS.to_string(),
    }
}

// ─── Vault state ────────────────────────────────────────────────────

/// In-memory vault state. The decrypted payload lives here only while
/// the vault is unlocked. When locked, the payload is dropped (and the
/// contained Secrets are zeroized).
pub struct VaultState {
    pub payload: Option<VaultPayload>,
    pub password: Option<String>,
    pub vault_path: PathBuf,
}

impl VaultState {
    fn new() -> Self {
        let vault_path = default_vault_path();
        Self {
            payload: None,
            password: None,
            vault_path,
        }
    }

    fn is_unlocked(&self) -> bool {
        self.payload.is_some()
    }

    fn lock(&mut self) {
        // Dropping the payload zeroizes all contained Secrets.
        self.payload = None;
        self.password = None;
    }
}

fn default_vault_path() -> PathBuf {
    if let Some(app_data) = dirs::data_dir() {
        app_data.join("Sentinel").join("vault.bin")
    } else {
        PathBuf::from("vault.bin")
    }
}

/// Check if a vault file exists at the default path.
#[tauri::command]
pub fn vault_exists(state: State<'_, Mutex<VaultState>>) -> bool {
    let state = state.lock().expect("vault state mutex");
    state.vault_path.exists()
}

/// Create a new vault with the given master password.
#[tauri::command]
pub fn vault_create(password: String, state: State<'_, Mutex<VaultState>>) -> Result<(), String> {
    let mut state = state.lock().expect("vault state mutex");
    create_vault(&state.vault_path, &password).map_err(|e| e.to_string())?;
    let payload = open_vault(&state.vault_path, &password).map_err(|e| e.to_string())?;
    state.payload = Some(payload);
    state.password = Some(password);
    Ok(())
}

/// Unlock an existing vault.
#[tauri::command]
pub fn vault_unlock(password: String, state: State<'_, Mutex<VaultState>>) -> Result<(), String> {
    let mut state = state.lock().expect("vault state mutex");
    if !state.vault_path.exists() {
        return Err("Vault file does not exist.".to_string());
    }
    let payload = open_vault(&state.vault_path, &password).map_err(|e| e.to_string())?;
    state.payload = Some(payload);
    state.password = Some(password);
    Ok(())
}

/// Lock the vault (clear in-memory state).
#[tauri::command]
pub fn vault_lock(state: State<'_, Mutex<VaultState>>) {
    let mut state = state.lock().expect("vault state mutex");
    state.lock();
}

/// Check if the vault is currently unlocked.
#[tauri::command]
pub fn vault_is_unlocked(state: State<'_, Mutex<VaultState>>) -> bool {
    let state = state.lock().expect("vault state mutex");
    state.is_unlocked()
}

/// List all accounts in the vault (sanitized — no secrets).
#[tauri::command]
pub fn list_accounts(state: State<'_, Mutex<VaultState>>) -> Result<Vec<AccountView>, String> {
    let state = state.lock().expect("vault state mutex");
    let payload = state.payload.as_ref().ok_or("Vault is locked.")?;
    Ok(payload.accounts.iter().map(AccountView::from).collect())
}

/// Generate the current TOTP code for an account.
#[tauri::command]
pub fn generate_code(
    account_id: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<CodeResult, String> {
    let state = state.lock().expect("vault state mutex");
    let payload = state.payload.as_ref().ok_or("Vault is locked.")?;
    let account = payload
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found.")?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    match account.otp_type {
        OtpType::Totp => generate_totp(
            &account.id,
            account.secret.as_bytes(),
            now,
            account.period,
            account.algorithm,
            account.digits,
        )
        .map_err(|e| e.to_string()),
        OtpType::Hotp => {
            // For HOTP, generate with the current counter (don't increment here —
            // increment requires explicit confirmation per spec).
            sentinel_core::otp::generate_hotp(
                &account.id,
                account.secret.as_bytes(),
                account.counter,
                account.algorithm,
                account.digits,
            )
            .map_err(|e| e.to_string())
        }
    }
}

/// Increment the HOTP counter for an account (requires explicit confirmation).
#[tauri::command]
pub fn increment_hotp_counter(
    account_id: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<(), String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;
    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;

    let account = payload
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found.")?;

    if account.otp_type != OtpType::Hotp {
        return Err("Account is not HOTP.".to_string());
    }

    account.counter += 1;
    account.updated_at = sentinel_core::vault::now_ms();

    // Save the updated payload
    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Account CRUD ───────────────────────────────────────────────────

/// Input for manual account creation.
#[derive(Debug, Deserialize)]
pub struct ManualAccountInputDto {
    pub issuer: String,
    pub label: String,
    pub secret: String,
    pub otp_type: OtpType,
    pub algorithm: OtpAlgorithm,
    pub digits: Digits,
    pub period: u32,
    pub counter: u64,
    pub icon_color: Option<String>,
    pub icon_text: Option<String>,
}

/// Add a new account manually (from the add-account form).
#[tauri::command]
pub fn add_account_manual(
    input: ManualAccountInputDto,
    state: State<'_, Mutex<VaultState>>,
) -> Result<AccountView, String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;

    let input = sentinel_core::import::manual::ManualAccountInput {
        issuer: input.issuer,
        label: input.label,
        secret: input.secret,
        otp_type: input.otp_type,
        algorithm: input.algorithm,
        digits: input.digits,
        period: input.period,
        counter: input.counter,
        icon_color: input.icon_color,
        icon_text: input.icon_text,
    };

    let id = uuid_v4();
    let account = validate_manual_account(input, id).map_err(|e| e.to_string())?;

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;
    payload.accounts.push(account.clone());
    payload.touch();

    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(AccountView::from(&account))
}

/// Add a new account from an otpauth:// URI.
#[tauri::command]
pub fn add_account_from_otpauth(
    uri: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<AccountView, String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;

    let parsed = parse_otpauth_uri(&uri).map_err(|e| e.to_string())?;
    let id = uuid_v4();
    let account = parsed.into_account_record(id);

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;
    payload.accounts.push(account.clone());
    payload.touch();

    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(AccountView::from(&account))
}

/// Import accounts from a Google migration URI.
#[tauri::command]
pub fn import_from_migration(
    uri: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<Vec<AccountView>, String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;

    let batch = parse_migration_uri(&uri).map_err(|e| e.to_string())?;
    let mut added = Vec::new();

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;

    for parsed in batch.accounts {
        let id = uuid_v4();
        let account = parsed.into_account_record(id);
        added.push(AccountView::from(&account));
        payload.accounts.push(account);
    }

    payload.touch();
    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(added)
}

/// Update an existing account's non-secret fields.
#[derive(Debug, Deserialize)]
pub struct UpdateAccountInput {
    pub id: String,
    pub issuer: Option<String>,
    pub label: Option<String>,
    pub favorite: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub icon_color: Option<Option<String>>,
    pub icon_text: Option<Option<String>>,
}

#[tauri::command]
pub fn update_account(
    input: UpdateAccountInput,
    state: State<'_, Mutex<VaultState>>,
) -> Result<(), String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;

    let account = payload
        .accounts
        .iter_mut()
        .find(|a| a.id == input.id)
        .ok_or("Account not found.")?;

    if let Some(issuer) = input.issuer {
        account.issuer = issuer;
    }
    if let Some(label) = input.label {
        account.label = label;
    }
    if let Some(favorite) = input.favorite {
        account.favorite = favorite;
    }
    if let Some(tags) = input.tags {
        account.tags = tags;
    }
    if let Some(icon_color) = input.icon_color {
        account.icon_color = icon_color;
    }
    if let Some(icon_text) = input.icon_text {
        account.icon_text = icon_text;
    }
    account.updated_at = sentinel_core::vault::now_ms();

    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete an account by ID.
#[tauri::command]
pub fn delete_account(
    account_id: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<(), String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;

    let original_len = payload.accounts.len();
    payload.accounts.retain(|a| a.id != account_id);
    if payload.accounts.len() == original_len {
        return Err("Account not found.".to_string());
    }
    payload.touch();

    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete multiple accounts by ID (bulk operation).
#[tauri::command]
pub fn delete_accounts(
    account_ids: Vec<String>,
    state: State<'_, Mutex<VaultState>>,
) -> Result<usize, String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;

    let original_len = payload.accounts.len();
    payload.accounts.retain(|a| !account_ids.contains(&a.id));
    let deleted = original_len - payload.accounts.len();
    payload.touch();

    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;

    Ok(deleted)
}

/// Change the master password.
#[tauri::command]
pub fn change_password(
    old_password: String,
    new_password: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<(), String> {
    let mut state = state.lock().expect("vault state mutex");
    change_master_password(&state.vault_path, &old_password, &new_password)
        .map_err(|e| e.to_string())?;
    state.password = Some(new_password);
    Ok(())
}

/// Create an encrypted backup at the given path.
#[tauri::command]
pub fn create_backup_file(
    path: String,
    backup_password: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<(), String> {
    let state = state.lock().expect("vault state mutex");
    let payload = state.payload.as_ref().ok_or("Vault is locked.")?;
    create_backup(std::path::Path::new(&path), &backup_password, payload).map_err(|e| e.to_string())
}

/// Preview a backup file's contents (no secrets).
#[tauri::command]
pub fn preview_backup_file(
    path: String,
    backup_password: String,
) -> Result<Vec<BackupPreviewEntry>, String> {
    preview_backup(std::path::Path::new(&path), &backup_password).map_err(|e| e.to_string())
}

/// Restore a backup file, merging accounts into the current vault.
#[tauri::command]
pub fn restore_backup_file(
    path: String,
    backup_password: String,
    state: State<'_, Mutex<VaultState>>,
) -> Result<usize, String> {
    let mut state = state.lock().expect("vault state mutex");
    let password = state.password.clone().ok_or("Vault is locked.")?;
    let backup_payload =
        restore_backup(std::path::Path::new(&path), &backup_password).map_err(|e| e.to_string())?;

    let payload = state.payload.as_mut().ok_or("Vault is locked.")?;
    let added = backup_payload.accounts.len();
    payload.accounts.extend(backup_payload.accounts);
    payload.touch();

    let payload_clone = payload.clone();
    save_vault(&state.vault_path, &password, payload_clone).map_err(|e| e.to_string())?;
    Ok(added)
}

/// Simple UUID v4 generator (uses OS CSPRNG).
fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    // Set version (4) and variant bits per RFC 4122
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}
