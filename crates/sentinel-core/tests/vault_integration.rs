//! Integration tests for the vault module.
//!
//! These exercise the full create → open → modify → save → reopen cycle
//! using real (temp) files, verifying that:
//!  - Round-trip encryption preserves account data
//!  - Wrong passwords are rejected
//!  - Tampered files are detected
//!  - Master password changes work end-to-end

use sentinel_core::models::{AccountRecord, Digits, OtpAlgorithm, OtpType, Secret};
use sentinel_core::vault::{
    change_master_password, create_vault_with_params, now_ms, open_vault, save_vault, KdfParams,
};
use tempfile::tempdir;

#[allow(clippy::missing_const_for_fn)]
fn test_params() -> KdfParams {
    KdfParams {
        m_cost_kib: 8,
        t_cost: 1,
        p_cost: 1,
    }
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
        tags: vec!["work".to_string()],
        favorite: true,
        sort_position: 0,
        icon_color: Some("#ff0000".to_string()),
        icon_text: Some("G".to_string()),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn full_vault_lifecycle() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("lifecycle.bin");

    // 1. Create
    create_vault_with_params(&path, "pw1", &test_params()).expect("create");

    // 2. Open (empty)
    let mut payload = open_vault(&path, "pw1").expect("open empty");
    assert!(payload.accounts.is_empty());

    // 3. Add accounts
    payload.accounts.push(dummy_account("a1", "GitHub"));
    payload.accounts.push(dummy_account("a2", "GitLab"));
    payload.accounts.push(dummy_account("a3", "AWS"));

    // 4. Save
    save_vault(&path, "pw1", payload).expect("save");

    // 5. Reopen and verify all fields survived the round trip
    let payload2 = open_vault(&path, "pw1").expect("reopen");
    assert_eq!(payload2.accounts.len(), 3);

    let github = &payload2.accounts[0];
    assert_eq!(github.id, "a1");
    assert_eq!(github.issuer, "GitHub");
    assert_eq!(github.label, "user@GitHub.example.com");
    assert_eq!(github.otp_type, OtpType::Totp);
    assert_eq!(github.algorithm, OtpAlgorithm::Sha1);
    assert_eq!(github.digits, Digits::Six);
    assert_eq!(github.period, 30);
    assert_eq!(github.tags, vec!["work".to_string()]);
    assert!(github.favorite);
    assert_eq!(github.icon_color.as_deref(), Some("#ff0000"));
    assert_eq!(github.icon_text.as_deref(), Some("G"));
    assert_eq!(github.secret.as_bytes(), b"12345678901234567890");
}

#[test]
fn wrong_password_rejected_at_every_stage() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("wrongpw.bin");

    create_vault_with_params(&path, "correct", &test_params()).expect("create");

    // Open with wrong password
    let result = open_vault(&path, "WRONG");
    assert!(result.is_err());

    // Save with wrong password (must not corrupt the vault)
    let payload = open_vault(&path, "correct").expect("open");
    let save_result = save_vault(&path, "WRONG", payload);
    assert!(save_result.is_err());

    // Original vault still works
    let payload = open_vault(&path, "correct").expect("original intact");
    assert!(payload.accounts.is_empty());

    // Change password with wrong old password
    let change_result = change_master_password(&path, "WRONG", "new");
    assert!(change_result.is_err());

    // Original password still works
    let payload = open_vault(&path, "correct").expect("password unchanged");
    assert!(payload.accounts.is_empty());
}

#[test]
fn master_password_change_preserves_accounts() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("changepw.bin");

    create_vault_with_params(&path, "old", &test_params()).expect("create");
    let mut payload = open_vault(&path, "old").expect("open");
    payload.accounts.push(dummy_account("a1", "GitHub"));
    payload.accounts.push(dummy_account("a2", "GitLab"));
    save_vault(&path, "old", payload).expect("save");

    change_master_password(&path, "old", "new").expect("change");

    // Old password no longer works
    assert!(open_vault(&path, "old").is_err());

    // New password works and accounts are preserved
    let payload = open_vault(&path, "new").expect("open with new");
    assert_eq!(payload.accounts.len(), 2);
    assert_eq!(payload.accounts[0].id, "a1");
    assert_eq!(payload.accounts[1].id, "a2");
}

#[test]
fn tampered_vault_detected() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("tampered.bin");

    create_vault_with_params(&path, "pw", &test_params()).expect("create");

    // Flip a byte in the ciphertext
    let mut bytes = std::fs::read(&path).expect("read");
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&path, bytes).expect("write");

    // Open must fail (GCM tag verification fails)
    let result = open_vault(&path, "pw");
    assert!(result.is_err());
}

#[test]
fn multiple_saves_each_get_fresh_nonce() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("nonces.bin");

    create_vault_with_params(&path, "pw", &test_params()).expect("create");

    let payload = open_vault(&path, "pw").expect("open");
    let bytes1 = std::fs::read(&path).expect("read");
    save_vault(&path, "pw", payload).expect("save 1");
    let bytes2 = std::fs::read(&path).expect("read 2");
    save_vault(&path, "pw", open_vault(&path, "pw").expect("open 2")).expect("save 2");
    let bytes3 = std::fs::read(&path).expect("read 3");

    // Nonce is at offset 39..51 (after magic + version + kdf_id + params + salt)
    let n1 = &bytes1[39..51];
    let n2 = &bytes2[39..51];
    let n3 = &bytes3[39..51];

    assert_ne!(n1, n2, "save 1 must use a fresh nonce");
    assert_ne!(n2, n3, "save 2 must use a fresh nonce");
    assert_ne!(n1, n3, "nonces must all differ");
}
