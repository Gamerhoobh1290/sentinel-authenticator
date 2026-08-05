# Sentinel Authenticator — Architecture

This document describes the high-level architecture of Sentinel. For cryptographic design specifically, see [`SECURITY.md`](./SECURITY.md).

## System overview

```
┌──────────────────────────────────────────────────────────────┐
│                    Tauri 2 Native Window                      │
│  ┌────────────────────────┐   ┌────────────────────────────┐ │
│  │   React + TS + Vite     │   │   Rust Backend              │ │
│  │   Tailwind v3.4         │   │   (sentinel-authenticator)  │ │
│  │   Zustand (UI state)    │◄──┤   ↑ thin Tauri shell        │ │
│  │                          │   │   │ depends on               │ │
│  │   - Lock screen          │   │   ▼                          │ │
│  │   - Account list         │   │   sentinel-core              │ │
│  │   - Add/import flows     │   │   (cross-platform library)   │ │
│  │   - Settings             │   │                              │ │
│  │   - Backup/restore UI    │   │   - vault (encrypted file)   │ │
│  │                          │   │   - otp (TOTP/HOTP engine)   │ │
│  │   QR scanning via        │   │   - import (otpauth + Google)│ │
│  │   getUserMedia + zxing   │   │   - backup (versioned AEAD)  │ │
│  │   (image bytes handed to │   │   - redact (log scrubbing)   │ │
│  │    Rust for decoding if  │   │   - models (sanitized views) │ │
│  │    needed)               │   │                              │ │
│  └────────────────────────┘   └────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
         │                                          │
         ▼                                          ▼
   WebView2 (Windows)                    %APPDATA%\Sentinel\
   - Strict CSP                            - vault.bin   (encrypted)
   - No remote origins allowed             - settings.json (non-sensitive)
   - No eval, no inline scripts            - backups\*.sentinelbak
```

## Repository layout

```
.
├── crates/
│   └── sentinel-core/          # Cross-platform Rust library
│       ├── Cargo.toml
│       ├── proto/               # Google migration protobuf schema (M5)
│       └── src/
│           ├── lib.rs
│           ├── error.rs         # SentinelError type
│           ├── models.rs        # AccountRecord, Secret, AccountView
│           ├── otp/             # HOTP + TOTP engine (M2)
│           ├── vault/           # AES-256-GCM + Argon2id (M3)
│           ├── import/          # otpauth:// + Google migration (M4/M5)
│           ├── backup/          # Versioned AEAD backup format (M8)
│           └── redact.rs        # Log redaction layer
│
├── src-tauri/                   # Tauri desktop shell
│   ├── Cargo.toml               # depends on sentinel-core via path
│   ├── tauri.conf.json          # window config, NSIS+MSI bundle, CSP
│   ├── capabilities/
│   │   └── default.json         # least-privilege Tauri capability
│   ├── icons/                   # generated app icons (all sizes)
│   └── src/
│       ├── main.rs              # entry point
│       ├── lib.rs               # Tauri::Builder + plugin registration
│       └── commands.rs          # #[tauri::command] handlers
│
├── src/                         # React + TypeScript frontend
│   ├── main.tsx                 # React entry
│   ├── App.tsx                  # root component
│   ├── components/
│   │   ├── ui/                  # design-system primitives (Button, Card, …)
│   │   ├── layout/              # AppShell (sidebar + main area)
│   │   ├── auth/                # LockScreen
│   │   ├── accounts/            # MainView (M6 will fill this in)
│   │   ├── import/              # Import flows (M4/M5)
│   │   ├── backup/              # Backup/restore UI (M8)
│   │   └── settings/            # Settings UI
│   ├── hooks/                   # useTheme, useReducedMotion, …
│   ├── lib/                     # cn() class merge, ipc helpers
│   ├── store/                   # Zustand stores (settings, vault state)
│   ├── styles/                  # globals.css (Tailwind + tokens)
│   ├── test/                    # Vitest setup
│   └── types/                   # shared TS types
│
├── scripts/
│   └── generate_icons.py        # builds all app icon sizes from SVG
│
├── .github/workflows/
│   ├── ci.yml                   # runs on every push: tests + lint
│   └── release-windows.yml      # builds Windows installer on windows-latest
│
├── docs/                        # see README for the doc index
├── Cargo.toml                   # workspace root
├── package.json                 # frontend
└── README.md
```

## Why two crates?

`sentinel-core` is the security-sensitive core (cryptography, OTP, vault, import, backup). It has **no Tauri dependency** so it can be unit-tested in CI on any operating system — Linux, macOS, or Windows — without needing platform UI libraries (WebView2 on Windows, `webkit2gtk` on Linux).

`sentinel-authenticator` (the Tauri app) is a thin shell that depends on `sentinel-core` via a path dependency and exposes selected functions as `#[tauri::command]` IPC handlers. Its build is gated to Windows because Tauri's native deps make Linux cross-compilation unreliable.

This split keeps the security-critical code testable everywhere while still producing a native Windows binary.

## Frontend state

The React app uses two Zustand stores:

- **`settingsStore`** — non-sensitive UI preferences (theme, density, auto-lock delay, clipboard clear delay, etc.). Persisted to `localStorage` intentionally — no secrets ever live here. The ESLint config forbids `localStorage` access anywhere else in the codebase via the `no-restricted-syntax` rule.
- **`vaultStore`** — the vault lifecycle state machine (`uninitialised | locked | unlocking | unlocked | locking`). Never holds decrypted account data. The decrypted `Vec<AccountRecord>` lives in Rust memory and is zeroized on lock.

## IPC boundary

The frontend calls Rust via Tauri's `invoke()` API. The contract is:

1. **Sanitized views**: the frontend only ever receives `AccountView` (no `secret` field). Raw secret bytes never cross the IPC boundary except when the user explicitly re-authenticates to reveal one.
2. **Codes on demand**: OTP codes are generated in Rust from the in-memory decrypted secret and returned to the frontend. The frontend never holds the secret long enough to generate a code itself.
3. **Lock clears state**: when the vault locks, all in-memory decrypted state in Rust is zeroized, and the frontend's `vaultStore` flips to `locked`. The frontend re-renders the lock screen.

## Window + lifecycle

- Single main window (`label: "main"`), 960×640 default, 720×520 minimum, resizable.
- Tray icon (M7) provides Open / Lock / Add account / Quit.
- Auto-lock (M7) uses an inactivity timer + Windows session-lock notification (`WTS_SESSION_LOCK` via the `windows` crate).
- "Start with Windows" (M7) uses `tauri-plugin-autostart` to write an `HKCU\...\Run` shortcut — no admin rights required.

## Build pipeline

- **Dev**: `npm run tauri:dev` starts Vite on `:1420` and Tauri's dev watcher.
- **Production**: `npm run build` produces the frontend bundle in `dist/`, then `cargo tauri build` packages it with the Rust binary and produces NSIS + MSI installers in `src-tauri/target/.../release/bundle/`.
- **CI**: every push runs Rust tests + frontend tests on Linux (cheap). Tag pushes (`v*`) and manual triggers run the full Windows build on `windows-latest` and upload the installer as a workflow artifact.
