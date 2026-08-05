/**
 * Sentinel Authenticator — IPC bridge.
 *
 * Wraps Tauri's `invoke()` with typed functions. Each function maps to
 * a `#[tauri::command]` in `src-tauri/src/commands.rs`.
 *
 * In test environments (Vitest), this module is mocked via
 * `src/lib/ipc.test.ts` — the real Tauri IPC is not available.
 */

import type { AccountView, CodeResult } from "@/types";
import type { OtpAlgorithm, OtpType } from "@/types";

export interface ManualAccountInput {
  issuer: string;
  label: string;
  secret: string;
  otpType: OtpType;
  algorithm: OtpAlgorithm;
  digits: 6 | 8;
  period: number;
  counter: number;
  iconColor?: string;
  iconText?: string;
}

export interface UpdateAccountInput {
  id: string;
  issuer?: string;
  label?: string;
  favorite?: boolean;
  tags?: string[];
  iconColor?: string | null;
  iconText?: string | null;
}

// Lazy-load the Tauri invoke function so this module can be imported
// in non-Tauri environments (Vitest) without throwing.
let _invoke:
  ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (_invoke === null) {
    try {
      const api = await import("@tauri-apps/api/core");
      _invoke = api.invoke;
    } catch {
      throw new Error(
        `IPC call '${cmd}' failed: Tauri API not available (running outside Tauri?)`,
      );
    }
  }
  const fn = _invoke;
  if (!fn) throw new Error("IPC not initialized");
  return fn(cmd, args) as Promise<T>;
}

export const ipc = {
  ping: () => invoke<string>("ping"),

  appMeta: () =>
    invoke<{ name: string; version: string; platform: string }>("app_meta"),

  vaultExists: () => invoke<boolean>("vault_exists"),

  vaultCreate: (password: string) => invoke<void>("vault_create", { password }),

  vaultUnlock: (password: string) => invoke<void>("vault_unlock", { password }),

  vaultLock: () => invoke<void>("vault_lock"),

  vaultIsUnlocked: () => invoke<boolean>("vault_is_unlocked"),

  listAccounts: () => invoke<AccountView[]>("list_accounts"),

  generateCode: (accountId: string) =>
    invoke<CodeResult>("generate_code", { accountId }),

  incrementHotpCounter: (accountId: string) =>
    invoke<void>("increment_hotp_counter", { accountId }),

  addAccountManual: (input: ManualAccountInput) =>
    invoke<AccountView>("add_account_manual", {
      input: {
        issuer: input.issuer,
        label: input.label,
        secret: input.secret,
        otp_type: input.otpType,
        algorithm: input.algorithm,
        digits: input.digits,
        period: input.period,
        counter: input.counter,
        icon_color: input.iconColor,
        icon_text: input.iconText,
      },
    }),

  addAccountFromOtpauth: (uri: string) =>
    invoke<AccountView>("add_account_from_otpauth", { uri }),

  importFromMigration: (uri: string) =>
    invoke<AccountView[]>("import_from_migration", { uri }),

  updateAccount: (input: UpdateAccountInput) =>
    invoke<void>("update_account", {
      input: {
        id: input.id,
        issuer: input.issuer,
        label: input.label,
        favorite: input.favorite,
        tags: input.tags,
        icon_color: input.iconColor,
        icon_text: input.iconText,
      },
    }),

  deleteAccount: (accountId: string) => invoke<void>("delete_account", { accountId }),

  deleteAccounts: (accountIds: string[]) =>
    invoke<number>("delete_accounts", { accountIds }),

  changePassword: (oldPassword: string, newPassword: string) =>
    invoke<void>("change_password", { oldPassword, newPassword }),

  createBackup: (path: string, backupPassword: string) =>
    invoke<void>("create_backup_file", { path, backupPassword }),

  previewBackup: (path: string, backupPassword: string) =>
    invoke<BackupPreviewEntry[]>("preview_backup_file", { path, backupPassword }),

  restoreBackup: (path: string, backupPassword: string) =>
    invoke<number>("restore_backup_file", { path, backupPassword }),
};

export interface BackupPreviewEntry {
  issuer: string;
  label: string;
  otpType: "totp" | "hotp";
}
