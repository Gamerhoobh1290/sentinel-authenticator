# Sentinel Authenticator

> A local-first, fully offline desktop two-factor authentication (2FA) app for Windows.

[![CI](https://github.com/sentinel/sentinel-authenticator/actions/workflows/ci.yml/badge.svg)](./.github/workflows/ci.yml)
[![Windows build](https://github.com/sentinel/sentinel-authenticator/actions/workflows/release-windows.yml/badge.svg)](./.github/workflows/release-windows.yml)

**Status: early development — milestone 1 (scaffold + design system).** Many features described below are planned; see the [Roadmap](#roadmap) section.

---

## What Sentinel does

Sentinel Authenticator is a Windows desktop application that generates TOTP and HOTP one-time codes locally on your computer. It works entirely offline after installation — no account registration, no cloud backend, no telemetry, no advertising, and no automatic secret synchronization. All authenticator secrets are encrypted at rest with AES-256-GCM and unlocked only with your master password.

It is designed as a privacy-respecting alternative to cloud-connected authenticator apps for users who keep their 2FA secrets on a single trusted PC.

## Supported OTP formats

- **TOTP** — RFC 6238 (time-based)
- **HOTP** — RFC 4226 (counter-based)
- **Algorithms** — SHA-1 (RFC default), SHA-256, SHA-512
- **Code length** — 6-digit and 8-digit
- **Period** — standard 30 seconds, or any custom period provided by an imported account
- **HOTP counters** — with explicit confirmation before incrementing
- **Base32 secrets** — RFC 4648, with input normalization (whitespace and lowercase tolerated)
- **`otpauth://` URIs** — both `totp/` and `hotp/` variants
- **Google Authenticator migration** — `otpauth-migration://offline?data=...` payloads, including multi-batch transfers

## Installation

> ⚠️ **Unsigned installer notice.** The Sentinel installer is currently **not digitally signed**. Windows SmartScreen will display a warning the first time you run it. To proceed, click **More info → Run anyway**. See [`docs/RELEASE.md`](./docs/RELEASE.md) for verification and code-signing guidance.

### Download a prebuilt installer

1. Go to the **Actions** tab → **Build Windows installer** workflow.
2. Pick the most recent successful run.
3. Download the **`Sentinel-Authenticator-Setup-exe`** artifact.
4. Unzip and run `Sentinel-Authenticator-Setup.exe`.

### Build from source (Windows)

Prerequisites:

- Windows 10 or Windows 11
- [Rust](https://rustup.rs/) (stable, MSVC toolchain)
- [Node.js](https://nodejs.org/) 20 or later
- [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (preinstalled on Windows 11; Windows 10 users may need to install it)
- [NSIS](https://nsis.sourceforge.io/) (optional — Tauri's bundled copy usually works)

```powershell
git clone https://github.com/sentinel/sentinel-authenticator.git
cd sentinel-authenticator
npm install
cargo tauri build --target x86_64-pc-windows-msvc
```

The installer will appear at:

```
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Sentinel-Authenticator-Setup.exe
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/Sentinel-Authenticator-Setup.msi
```

## Development

### Prerequisites

- Rust stable via [rustup](https://rustup.rs/)
- Node.js 20+ and npm
- For the Tauri desktop shell on Linux dev: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` (Linux dev only — the Windows build does not need these)
- The [`tauri-cli`](https://tauri.app/) npm package (installed as a dev dependency)

### Run

```bash
npm install           # install frontend dependencies
npm run tauri:dev     # launch the desktop app in dev mode (hot reload)
```

### Test

```bash
# Rust core (OTP, vault, import, backup) — runs on any OS
cargo test -p sentinel-core --all-targets
cargo clippy -p sentinel-core --all-targets -- -D warnings
cargo fmt --all --check

# Frontend (React components, type safety)
npm run test
npm run typecheck
npm run lint
npm run format:check

# Production build (frontend bundle)
npm run build
```

### Build the Windows installer

See the **Build from source (Windows)** section above, or trigger the GitHub Actions workflow from the Actions tab.

## Security summary

- **Cryptography**: AES-256-GCM (AEAD) for vault encryption; Argon2id for master-password key derivation; OS CSPRNG for salts, nonces, and the vault key; constant-time secret comparisons via the `subtle` crate; key zeroization via `zeroize`. No custom cryptography.
- **No plaintext at rest**: secrets are never written to SQLite, JSON, localStorage, sessionStorage, or configuration files. They live only inside the encrypted vault file at `%APPDATA%\Sentinel\vault.bin`.
- **No remote network**: the Tauri capability configuration grants no network permissions. The Content Security Policy restricts all remote origins.
- **Auto-lock**: configurable inactivity timer; lock-on-minimize; lock-on-Windows-session-lock (via `WTS_SESSION_LOCK` notifications).
- **Clipboard protection**: copied codes auto-clear after 10/30/60 seconds (default 30s); only cleared if the clipboard still contains the value placed by Sentinel; raw secrets require explicit re-authentication.
- **Master password**: required on first launch after restart, when changing security settings, when exporting a backup, when revealing a raw secret, and when disabling important protections. Never stored.

See [`docs/SECURITY.md`](./docs/SECURITY.md) for the full security design and [`docs/THREAT_MODEL.md`](./docs/THREAT_MODEL.md) for honest coverage of what Sentinel can and cannot protect against.

## Backup warning

Sentinel can create an encrypted backup file (`.sentinelbak`) protected by a **backup password independent of your master password**. Store the backup file and the backup password in separate secure locations. If you lose both your master password and your backup, your accounts cannot be recovered — by design.

See [`docs/BACKUP_FORMAT.md`](./docs/BACKUP_FORMAT.md) for the backup file format and safe-storage guidance.

## Known limitations

- **Unsigned installer**: Windows SmartScreen will warn on first run.
- **No Windows Hello in v1**: master password only. Windows Hello will be added in a future release if it can be implemented securely using supported Windows APIs.
- **Google cloud-synced authenticator accounts cannot be downloaded** through a supported public API. Import through transfer QR codes, original QR codes, or manual setup keys.
- **Compromised PC**: if malware runs under the same Windows account, it can potentially capture unlocked codes via screen capture or memory scraping. Sentinel mitigates (auto-lock, no plaintext at rest, clipboard clearing) but does not claim to be invulnerable.
- **No Android or iOS version**: this release is focused on Windows desktop only.
- **Camera access**: depends on WebView2's `getUserMedia` support. If the OS denies camera permission, image-file import is the fallback.

## ⚠️ Keep your original authenticator until verified

After importing accounts into Sentinel, **do not delete them from your original authenticator** until you have verified that the imported codes work correctly for at least one full TOTP period cycle (30 seconds minimum, ideally across a period boundary at the top of the minute). Some issuers use non-standard parameters that may need manual correction.

## Roadmap

This project follows a milestone-based development plan:

- [x] **M1** — Project scaffold, design system, basic window
- [ ] **M2** — OTP generation engine with RFC tests
- [ ] **M3** — Encrypted vault and lock/unlock workflow
- [ ] **M4** — Manual account creation and normal QR import
- [ ] **M5** — Google Authenticator migration import
- [ ] **M6** — Main account interface, search and organisation
- [ ] **M7** — Clipboard protection, auto-lock, system tray
- [ ] **M8** — Encrypted backup and restore
- [ ] **M9** — Accessibility, performance, error-state pass
- [ ] **M10** — Security review, dependency audit, threat-model review
- [ ] **M11** — Full automated and manual testing
- [ ] **M12** — Windows packaging and final documentation

## Documentation

- [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) — system architecture and module layout
- [`docs/SECURITY.md`](./docs/SECURITY.md) — cryptographic design and security decisions
- [`docs/THREAT_MODEL.md`](./docs/THREAT_MODEL.md) — what Sentinel protects against, and what it cannot
- [`docs/GOOGLE_IMPORT.md`](./docs/GOOGLE_IMPORT.md) — Google Authenticator transfer QR import flow
- [`docs/BACKUP_FORMAT.md`](./docs/BACKUP_FORMAT.md) — encrypted backup file format specification
- [`docs/MANUAL_QA.md`](./docs/MANUAL_QA.md) — repeatable manual QA checklist
- [`docs/RELEASE.md`](./docs/RELEASE.md) — release process, code signing, SmartScreen guidance

## License

MIT. See [`LICENSE`](./LICENSE).
