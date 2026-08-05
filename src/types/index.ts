/**
 * Sentinel Authenticator — shared types
 *
 * These mirror the Rust-side structs in `src-tauri/src/models.rs`.
 * Frontend code ONLY receives sanitized versions of these — raw secrets
 * never travel across the IPC boundary except when the user explicitly
 * reveals a secret after re-authentication.
 */

export type OtpType = "totp" | "hotp";

export type OtpAlgorithm = "sha1" | "sha256" | "sha512";

export type Theme = "dark" | "light" | "system";

export type Density = "comfortable" | "compact";

export type ClipboardClearDelay = 10_000 | 30_000 | 60_000 | "never";

export type AutoLockDelay =
  60_000 | 300_000 | 900_000 | 1_800_000 | 3_600_000 | "never";

/** Sanitized account record — what the frontend receives for display. */
export interface AccountView {
  id: string;
  issuer: string;
  label: string;
  otpType: OtpType;
  algorithm: OtpAlgorithm;
  digits: 6 | 8;
  period?: number; // seconds (TOTP only)
  counter?: number; // HOTP only
  tags: string[];
  favorite: boolean;
  sortPosition: number;
  iconColor?: string;
  iconText?: string;
  createdAt: number; // ms since epoch
  updatedAt: number;
}

/** Current code result — generated on demand, never stored. */
export interface CodeResult {
  accountId: string;
  code: string;
  /** Seconds remaining in the current period (TOTP only). */
  secondsRemaining?: number;
  /** Period in seconds (TOTP only). */
  period?: number;
}

/** Vault lifecycle states. */
export type VaultState =
  | "uninitialised" // No vault exists yet — user must create one
  | "locked" // Vault exists, master password required
  | "unlocking" // Verifying password
  | "unlocked" // Decrypted vault in memory
  | "locking"; // Clearing state

/** Non-sensitive application settings. Persisted to settings.json. */
export interface AppSettings {
  theme: Theme;
  density: Density;
  reducedMotion: boolean;
  accentColor?: string;
  autoLockDelay: AutoLockDelay;
  lockWhenMinimized: boolean;
  lockWhenWindowsLocks: boolean;
  clipboardClearDelay: ClipboardClearDelay;
  codeFormatting: "grouped" | "plain";
  startWithWindows: boolean;
  startMinimized: boolean;
  minimizeToTray: boolean;
  closeToTray: boolean;
  alwaysOnTop: boolean;
  confirmDestructiveActions: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  density: "comfortable",
  reducedMotion: false,
  autoLockDelay: 300_000,
  lockWhenMinimized: true,
  lockWhenWindowsLocks: true,
  clipboardClearDelay: 30_000,
  codeFormatting: "grouped",
  startWithWindows: false,
  startMinimized: false,
  minimizeToTray: true,
  closeToTray: true,
  alwaysOnTop: false,
  confirmDestructiveActions: true,
};
