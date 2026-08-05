# Sentinel Authenticator — Release Process

## Build the Windows installer

### Prerequisites

- Windows 10 or Windows 11
- [Rust](https://rustup.rs/) stable with MSVC toolchain
- [Node.js](https://nodejs.org/) 20+
- [protoc](https://github.com/protocolbuffers/protobuf/releases) v25+ (or let CI install it)
- [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)

### Build locally

```powershell
git clone https://github.com/sentinel/sentinel-authenticator.git
cd sentinel-authenticator
npm install
cargo tauri build --target x86_64-pc-windows-msvc
```

The installer will be at:

```
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Sentinel-Authenticator-Setup.exe
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/Sentinel-Authenticator-Setup.msi
```

### Build via GitHub Actions

1. Push a tag: `git tag v0.1.0 && git push origin v0.1.0`
2. The `release-windows.yml` workflow runs on `windows-latest`
3. It builds the NSIS + MSI installers
4. Artifacts are uploaded to the Actions run
5. A draft GitHub Release is created (if the tag starts with `v`)

Download the artifact ZIP from the Actions tab → extract `Sentinel-Authenticator-Setup.exe`.

## Code signing

### Current status: UNSIGNED

The installer is **not digitally signed**. This means:

- **Windows SmartScreen** will show "Windows protected your PC" on first run
- Users must click "More info" → "Run anyway"
- This is expected for an open-source project without a paid code-signing certificate

### Adding code signing (future)

To sign the installer, you need:

1. A code-signing certificate from a trusted CA (DigiCert, Sectigo, etc.)
   - EV (Extended Validation) certificates bypass SmartScreen immediately
   - OV (Organization Validation) certificates build reputation over time
2. Export the certificate as a `.pfx` file
3. Base64-encode it: `base64 cert.pfx > cert.b64`
4. Add GitHub repository secrets:
   - `WINDOWS_CERTIFICATE_PFX_BASE64` — the base64-encoded .pfx
   - `WINDOWS_CERTIFICATE_PASSWORD` — the .pfx password
5. The `release-windows.yml` workflow will automatically sign the installer

### SmartScreen reputation

Even with an OV certificate, SmartScreen will initially warn users until the certificate builds reputation (hundreds of downloads). EV certificates bypass this entirely but are more expensive.

## Release checklist

Before tagging a release:

1. [ ] All automated tests pass (`cargo test`, `npm run test`, `cargo clippy`, `npm run lint`)
2. [ ] Manual QA checklist completed (see `docs/MANUAL_QA.md`)
3. [ ] `README.md` version updated
4. [ ] `tauri.conf.json` version updated
5. [ ] `Cargo.toml` versions updated
6. [ ] `package.json` version updated
7. [ ] Changelog updated (if maintained)
8. [ ] No secrets or test data committed
9. [ ] Commit and tag: `git tag v0.X.Y`
10. [ ] Push tag: `git push origin v0.X.Y`
11. [ ] Wait for GitHub Actions to complete
12. [ ] Review the draft GitHub Release
13. [ ] Publish the release

## Installer output locations

After a successful build:

```
src-tauri/target/x86_64-pc-windows-msvc/release/
├── sentinel-authenticator.exe          # the raw executable
└── bundle/
    ├── nsis/
    │   └── Sentinel-Authenticator-Setup.exe   # NSIS installer (recommended)
    └── msi/
        └── Sentinel-Authenticator-Setup.msi    # MSI installer
```

## Uninstallation

- Uninstalling via Windows Settings removes the application binary
- **User data** (`%APPDATA%\Sentinel\vault.bin`) is **not removed** by uninstall — this prevents accidental data loss
- To fully clean up: manually delete `%APPDATA%\Sentinel\` after uninstalling

## Version numbering

Sentinel uses [Semantic Versioning](https://semver.org/):

- `MAJOR`: breaking changes (e.g., vault format v2 that can't read v1)
- `MINOR`: new features (e.g., Windows Hello support)
- `PATCH`: bug fixes

Tags: `v0.1.0`, `v0.1.1`, `v0.2.0`, etc.
