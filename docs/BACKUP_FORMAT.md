# Sentinel Authenticator — Backup File Format

## Overview

Sentinel backup files (`.sentinelbak`) use a versioned, encrypted format independent of the vault's master password. The backup password is separate — losing your vault master password does not compromise your backups, and vice versa.

## File layout (v1)

```
Offset  Size      Field
------  --------  ----------------------------------------------------
   0     8 bytes   Magic: "SENTBK01" (ASCII)
   8     2 bytes   Version (u16 LE) — currently 1
  10     1 byte    KDF algorithm ID (1 = Argon2id)
  11     4 bytes   Argon2id m_cost (u32 LE, in KiB)
  15     4 bytes   Argon2id t_cost (u32 LE, iterations)
  19     4 bytes   Argon2id p_cost (u32 LE, parallelism)
  23    16 bytes   Salt (random, unique per backup)
  39    12 bytes   AES-256-GCM nonce (random, fresh per backup)
  51     N bytes   AES-256-GCM ciphertext (payload_cbor + 16-byte tag)
```

**Total header size**: 51 bytes.

## Encrypted payload (CBOR)

The plaintext inside the AES-256-GCM ciphertext is a CBOR-encoded `BackupPayload`:

```rust
struct BackupPayload {
    schema_version: u16,        // payload schema version (currently 1)
    created_at: u64,             // ms since epoch when backup was created
    source_vault_version: u16,   // vault format version of the source
    accounts: Vec<AccountRecord>, // the actual account data
}
```

Each `AccountRecord` contains:
- `id`: unique UUID v4
- `issuer`, `label`: account metadata
- `secret`: raw secret bytes (serialized as a CBOR byte array)
- `otp_type`: TOTP or HOTP
- `algorithm`: SHA1, SHA256, or SHA512
- `digits`: 6 or 8
- `period`: TOTP period in seconds (ignored for HOTP)
- `counter`: HOTP counter (ignored for TOTP)
- `tags`, `favorite`, `sort_position`, `icon_color`, `icon_text`
- `created_at`, `updated_at`: timestamps

## Cryptography

### Key derivation

- **Algorithm**: Argon2id (RFC 9106)
- **Parameters**: Same as vault (64 MiB / 3 iterations / 4 lanes by default)
- **Salt**: 16 bytes from OS CSPRNG, unique per backup
- **Output**: 32-byte AES-256 key

The backup password is independent from the vault master password. You can use the same password if you want, but it's more secure to use a different one — that way, compromising one doesn't compromise the other.

### Encryption

- **Algorithm**: AES-256-GCM (NIST SP 800-38D)
- **Nonce**: 12 bytes from OS CSPRNG, fresh per backup
- **Tag**: 16 bytes (appended to ciphertext)
- **Tamper detection**: GCM tag verification on restore

## What is NOT in the backup

- **No master-password verifier** — including one would make offline password guessing easier
- **No vault file path or machine identifier** — backups are portable
- **No plaintext metadata** — everything is inside the encrypted blob

## Versioning

The backup format has two version fields:

1. **File format version** (offset 8): the byte-level file layout. Currently 1. If this changes, old backups can still be read by checking the version and dispatching to the appropriate parser.

2. **Payload schema version** (inside the encrypted CBOR): the structure of the `BackupPayload`. Currently 1. If this changes, migrations can be run after decryption on the in-memory plaintext.

Both versions allow forward and backward compatibility.

## Safe storage guidance

- **Store the backup file and backup password in separate locations.** If an attacker gets both, they can decrypt all your accounts.
- **Use a strong backup password.** The same Argon2id cost applies as the vault — a weak password can be brute-forced.
- **Store backups on encrypted media** (e.g., an encrypted USB drive, a password manager's secure notes).
- **Create backups regularly** — after adding new accounts.
- **Test restoring from backup** periodically to make sure it works.

## Restore behavior

When restoring a backup, Sentinel:
1. Reads the file and parses the header
2. Derives the key from the backup password using the stored Argon2id parameters
3. Decrypts the ciphertext (GCM tag verification catches tampering or wrong passwords)
4. Decodes the CBOR payload
5. **Merges** the accounts into the current vault (does not replace)
6. Saves the updated vault

Duplicate detection (same issuer + label) is handled by the user in the restore preview dialog.
