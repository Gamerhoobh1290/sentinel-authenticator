# Sentinel Authenticator — Security Design

## Cryptography

### Master password key derivation

**Algorithm**: Argon2id (RFC 9106)

**Parameters** (OWASP 2024 recommendation for interactive password hashing):
- Memory: 65,536 KiB (64 MiB)
- Iterations: 3
- Parallelism: 4 lanes
- Output: 32 bytes (AES-256 key)

These parameters are stored in the vault file header so older vaults retain their original parameters even if future versions change the defaults. On a 2024-era laptop, key derivation takes ~150-300ms — fast enough for interactive unlock, slow enough to make brute force expensive.

**Why Argon2id?** Argon2id combines resistance to side-channel attacks (i-mode) with resistance to GPU/ASIC cracking (d-mode). It won the Password Hashing Competition (2015) and is recommended by OWASP and RFC 9106.

### Vault encryption

**Algorithm**: AES-256-GCM (NIST SP 800-38D)

- **Key**: 32 bytes derived from master password via Argon2id
- **Nonce**: 12 bytes (96 bits) from OS CSPRNG, fresh per encryption
- **Tag**: 16 bytes (128 bits), appended to ciphertext by `aes-gcm` crate
- **Salt**: 16 bytes from OS CSPRNG, unique per vault

AES-256-GCM provides **authenticated encryption** — the GCM tag verifies both confidentiality and integrity. Any tampering with the ciphertext or header is detected on decryption.

**Why AES-256-GCM?** It's NIST-approved, FIPS-validated, hardware-accelerated on modern x86 processors (AES-NI), and widely deployed. The `aes-gcm` RustCrypto crate is actively maintained and has been audited.

### Nonce management

A **fresh random nonce** is generated for every save operation. Nonce reuse with the same key would catastrophically compromise AES-GCM (both confidentiality and integrity). Sentinel never reuses nonces — each `save_vault` call generates a new 12-byte random nonce.

### Key zeroization

- `VaultKey` (the derived AES key) implements `Drop` with `zeroize()` — the key bytes are overwritten with zeros when the value goes out of scope.
- `Secret` (the raw OTP secret bytes) also implements `Drop` with `zeroize()`.
- The decrypted `VaultPayload` is dropped when the vault locks, which cascades zeroization to all contained `Secret` values.

### Random number generation

All random values (salts, nonces, UUIDs) are generated via `getrandom` (RustCrypto), which uses the OS CSPRNG:
- Windows: `BCryptGenRandom`
- Linux: `getrandom` syscall or `/dev/urandom`

Sentinel never uses a userspace PRNG for security-sensitive values.

### Constant-time comparisons

Secret comparisons use `subtle::ConstantTimeEq` to prevent timing side-channels. This is used in:
- `Secret::eq` (when comparing two secrets in memory)
- `otp::constant_time_eq` (when verifying a user-supplied code)

## What is NOT stored

- **No plaintext secrets** in SQLite, JSON, localStorage, sessionStorage, or config files
- **No master-password verifier** in the vault file — including one would allow offline password guessing without the Argon2id cost. The only way to verify a password is the full Argon2id + AES-GCM decryption.
- **No plaintext metadata** — issuer names, account labels, and all other account data live inside the encrypted blob
- **No OTP codes** are stored — codes are generated on demand from the encrypted secret and discarded immediately

## Network and privacy

- **No network permissions** granted in the Tauri capability configuration
- **Strict CSP**: `default-src 'self'; script-src 'self'; connect-src 'self' ipc: http://ipc.localhost` — no remote origins
- **No telemetry, analytics, or advertising**
- **No cloud synchronization** — all data stays on the local machine
- **No account registration** — the app is fully offline

## Tauri capability configuration

The capability file (`src-tauri/capabilities/default.json`) grants only the minimum permissions needed:

- Window management (show, hide, minimize, close, set-focus, set-always-on-top)
- Clipboard (write-text, read-text, clear) — for code copying + auto-clear
- Dialog (open, save, message, ask) — for file pickers and confirmations
- Filesystem (read-text-file only — no arbitrary write)
- OS info (platform, version)
- Process (exit)

**Not granted**: shell execution, arbitrary filesystem write, HTTP fetch, WebSocket, raw network access.

## Windows Hello (deferred)

Windows Hello is **not implemented in v1**. It will be added in a future release only if it can be done securely using `Windows.Security.Credentials.UserConsentVerifier`, which:
- Does not store or expose biometric data to the application
- Returns only a boolean consent result
- Requires the master password as the recovery method

The master password will always remain the primary and reliable unlock method.

## Lock behavior

- **Auto-lock**: configurable inactivity timer (1/5/15/30/60 minutes or never)
- **Lock when minimized**: uses `visibilitychange` event
- **Lock when Windows locks**: will use `WTS_SESSION_CHANGE` / `WTS_SESSION_LOCK` via the `windows` crate (to be wired in a future update)
- **On lock**: all decrypted state is zeroized, clipboard clear timer is cancelled

## Clipboard protection

- Codes are copied as numeric strings only (never raw secrets unless explicitly revealed)
- **Auto-clear** after 10/30/60 seconds (default 30s)
- Before clearing, the clipboard is re-read; **only cleared if it still contains the value we placed** — prevents clobbering something the user copied in the meantime
- Raw secret reveal requires explicit re-authentication and a warning dialog

## Dependency choices

All security-critical dependencies are from the **RustCrypto** organisation (actively maintained, audited, de-facto standard):

| Crate | Purpose | Status |
|-------|---------|--------|
| `aes-gcm` | AES-256-GCM AEAD | Actively maintained |
| `argon2` | Argon2id KDF | Actively maintained |
| `subtle` | Constant-time comparison | Actively maintained |
| `getrandom` | OS CSPRNG | Actively maintained |
| `zeroize` | Key zeroization | Actively maintained |
| `hmac` | HMAC for OTP | Actively maintained |
| `sha1`, `sha2` | SHA-1/256/512 for OTP | Actively maintained |
| `prost` | Protobuf decoding (Google migration) | Tokio team, actively maintained |

No custom cryptography is used anywhere in the codebase.
