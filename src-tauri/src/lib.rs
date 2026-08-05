//! Sentinel Authenticator — Rust backend.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

mod commands;

use std::sync::Mutex;

use commands::VaultState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

/// Sentinel application bootstrap.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .manage(Mutex::new(VaultState::new()))
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::app_meta,
            commands::vault_exists,
            commands::vault_create,
            commands::vault_unlock,
            commands::vault_lock,
            commands::vault_is_unlocked,
            commands::list_accounts,
            commands::generate_code,
            commands::increment_hotp_counter,
            commands::add_account_manual,
            commands::add_account_from_otpauth,
            commands::import_from_migration,
            commands::update_account,
            commands::delete_account,
            commands::delete_accounts,
            commands::change_password,
            commands::create_backup_file,
            commands::preview_backup_file,
            commands::restore_backup_file,
        ])
        .setup(|app| {
            eprintln!(
                "[sentinel] starting — version {} on {}",
                env!("CARGO_PKG_VERSION"),
                app.config().identifier
            );

            // System tray
            let open = MenuItem::with_id(app, "open", "Open Sentinel", true, None::<&str>)?;
            let lock = MenuItem::with_id(app, "lock", "Lock vault", true, None::<&str>)?;
            let add = MenuItem::with_id(app, "add", "Add account", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &lock, &add, &quit])?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Sentinel Authenticator")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "lock" => {
                        use tauri::State;
                        let state: State<'_, Mutex<VaultState>> = app.state();
                        let mut state = state.lock().expect("vault state mutex");
                        state.lock();
                    }
                    "add" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("sentinel://add-account", ());
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Sentinel Authenticator");
}
