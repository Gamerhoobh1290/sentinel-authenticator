# Sentinel Authenticator — Threat Model

## What Sentinel protects against

### 1. Someone obtaining a copy of the encrypted vault

**Protected**: The vault file (`vault.bin`) is encrypted with AES-256-GCM using a key derived from the master password via Argon2id (64 MiB / 3 iterations / 4 lanes). An attacker who obtains the file cannot decrypt it without the master password. The Argon2id parameters make brute-force guessing expensive (~150-300ms per attempt on modern hardware).

**Limitation**: If the master password is weak (short, common, reused), an attacker with the vault file can try common passwords offline. Choose a strong, unique master password.

### 2. Another local Windows user

**Protected**: The vault file is stored in `%APPDATA%\Sentinel\vault.bin`, which is under the user's profile directory. Windows ACLs prevent other non-admin users from reading it.

**Limitation**: A Windows administrator can read any user's files. Sentinel does not protect against malicious administrators.

### 3. Accidental disclosure through logs or screenshots

**Protected**: 
- The Rust redaction layer strips `otpauth://` URIs, `otpauth-migration://` URIs, base32 tokens (16+ chars), and `secret=`/`password=`/`data=` query parameters from any log output.
- The `Secret` type never prints its value in `Debug` output — only the length.
- Frontend `console.log` of secrets is prevented by the ESLint rule banning `localStorage`/`sessionStorage`/`document.cookie` access.
- No OTP codes are stored — they are generated on demand and discarded.

**Limitation**: The user could still manually screenshot the app while codes are visible. Sentinel cannot prevent OS-level screenshot capture.

### 4. Clipboard monitoring

**Protected**: Copied codes are automatically cleared after 10/30/60 seconds (default 30s). Before clearing, the clipboard is re-read — only cleared if it still contains the value we placed (doesn't clobber the user's other copies).

**Limitation**: During the 30-second window, a clipboard-monitoring malware process could capture the code. This is an inherent trade-off — the user needs to paste the code somewhere. Keep the auto-clear delay short.

### 5. Malicious QR codes

**Protected**: All QR payload fields are validated before use:
- `otpauth://` URIs: scheme, type, label, secret (Base32), algorithm, digits, period, counter are all validated
- `otpauth-migration://` payloads: protobuf decoded, each account validated (secret length 16-256 bytes, known algorithm/digits/type enums, MD5 rejected, HOTP counter required and non-negative)
- Maximum URI length: 2048 bytes
- Maximum field lengths: 256 characters
- Maximum data parameter: 16 KB

**Limitation**: A malicious QR code could still contain a valid-looking but fake account. The user should verify imported accounts against their original authenticator.

### 6. Malware running under the same Windows account

**Partially protected**: 
- Secrets are encrypted at rest
- Decrypted state lives only in Rust memory while unlocked
- Auto-lock clears decrypted state after inactivity
- No secrets in logs or browser storage

**NOT protected**: Malware running under the same user account can:
- Read the decrypted vault from the app's memory while it's unlocked
- Capture screenshots of visible codes
- Intercept clipboard contents (even with auto-clear, there's a 30s window)
- Log keystrokes (capturing the master password)
- Hook the WebView2 rendering pipeline

**Honest assessment**: A compromised PC can potentially capture unlocked codes or the master password. Sentinel mitigates but does not eliminate this risk. For high-security scenarios, use a separate hardware security key (FIDO2/WebAuthn) rather than a software authenticator on the same device.

### 7. Keyloggers and screen-capture malware

**NOT protected**: See section 6. Sentinel cannot detect or prevent keyloggers or screen-capture malware running under the same user account.

### 8. Using the authenticator on the same device as the account login

**Risk**: If an attacker compromises the device, they can capture both the OTP code (from Sentinel) and the account credentials (from the login page), defeating 2FA.

**Mitigation**: Use Sentinel on a separate device from where you log in to accounts, OR ensure the device is free of malware. Sentinel is designed for users who prefer a desktop authenticator over a phone-based one, but the same-device risk is inherent to all software authenticators.

### 9. Loss of the master password

**NOT recoverable**: By design, there is no password recovery mechanism. The master password is never stored, and no verifier is included in the vault file (which would make offline cracking easier). If you lose the master password, the vault cannot be decrypted.

**Mitigation**: Create encrypted backups regularly, and store the backup file and backup password in separate secure locations. If you lose the master password but have a backup, you can restore the backup with a new master password.

### 10. Loss or corruption of the local vault

**Protected**: 
- Atomic file writes (temp file + rename) prevent corruption on crash
- GCM tag provides tamper detection — corrupted files are rejected
- Encrypted backups provide a recovery path

**Limitation**: If the vault file is deleted or the disk fails without a backup, accounts are lost. Regular backups are essential.

### 11. Dependency or supply-chain risks

**Mitigated**:
- All dependencies are pinned in `Cargo.lock` and `package-lock.json`
- Security-critical dependencies are from the RustCrypto organisation (audited, actively maintained)
- `cargo audit` and `npm audit` run in CI
- No network permissions in the Tauri capability configuration — even if a dependency were compromised, it couldn't exfiltrate data

**Limitation**: A compromised dependency could still access in-memory data while the vault is unlocked. This is an inherent risk of using third-party libraries. Sentinel minimizes this by using a small number of well-established dependencies.

### 12. Incorrect system time

**Protected**: OTP codes are generated using UTC-based time. If the system clock is wrong, codes will be incorrect, but this does not compromise security — it just means the user can't log in until they fix their clock.

**Detection**: Sentinel does not silently apply a time offset. If codes repeatedly fail, the user should check their system clock. A future version may add a clock-skew warning.

## What Sentinel does NOT protect against

- **Physical access to an unlocked machine**: If the vault is unlocked and the attacker has physical access, they can see all codes.
- **Coercion ("rubber-hose cryptanalysis")**: Sentinel cannot prevent someone from forcing you to unlock the vault.
- **Zero-day exploits in WebView2 or Windows**: Sentinel relies on the platform's security.
- **Firmware-level malware**: Bootkits/rootkits can bypass all software security.
- **Side-channel attacks on the CPU**: Spectre/Meltdown-class vulnerabilities could theoretically leak in-memory secrets.

## Summary

Sentinel provides strong protection for OTP secrets **at rest** (encrypted with AES-256-GCM + Argon2id) and reasonable protection **in use** (auto-lock, clipboard clearing, zeroization, no logging). It does **not** protect against a compromised PC where malware runs under the same user account — that is an inherent limitation of all software authenticators. For maximum security, use a hardware security key.
