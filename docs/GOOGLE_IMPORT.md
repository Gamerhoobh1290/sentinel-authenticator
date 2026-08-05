# Google Authenticator Migration Import

## Overview

Sentinel can import accounts from Google Authenticator using the "Transfer accounts" flow. This lets you move accounts from your phone to your PC without retyping each secret manually.

## How it works

Google Authenticator's "Transfer accounts" → "Export accounts" feature generates QR codes containing an `otpauth-migration://offline?data=<base64>` URI. The `data` parameter is a base64-encoded protobuf payload containing one or more OTP accounts.

### Protobuf schema

The payload format is defined in [`proto/migration_payload.proto`](../crates/sentinel-core/proto/migration_payload.proto):

```protobuf
message MigrationPayload {
  repeated OtpParameters otp_parameters = 1;
  optional int32 batch_size  = 2;  // total QR codes in the batch
  optional int32 batch_index = 3;  // 0-based index of this QR
  optional int32 batch_id    = 4;  // stable ID shared across the batch
}

message OtpParameters {
  bytes secret = 1;
  string name = 2;
  string issuer = 3;
  Algorithm algorithm = 4;   // SHA1, SHA256, SHA512, MD5 (rejected)
  DigitCount digits = 5;     // 6 or 8
  OtpType type = 6;          // HOTP or TOTP
  optional int64 counter = 7;
}
```

## Step-by-step import

1. Open Google Authenticator on your phone
2. Tap the menu (⋮) → **Transfer accounts**
3. Tap **Export accounts**
4. Select the accounts to export
5. Google Authenticator displays one or more QR codes
6. In Sentinel, click **Import** → **Scan QR code**
7. Point your camera at each QR code
8. Sentinel detects and imports the accounts

### Multi-batch transfers

If you export many accounts, Google Authenticator splits them across multiple QR codes. Each QR contains:
- `batch_size`: the total number of QR codes in the transfer
- `batch_index`: which QR this is (0-based)
- `batch_id`: a stable ID shared across all QRs in the same batch

Sentinel validates that all batches are scanned before completing the import. If you miss a QR, you'll see an "Incomplete batch" error.

## Supported fields

| Field | Supported | Notes |
|-------|-----------|-------|
| Secret | ✅ | Raw bytes (not base32 — already decoded) |
| Account name | ✅ | |
| Issuer | ✅ | |
| Algorithm | ✅ | SHA1, SHA256, SHA512. MD5 is rejected. |
| Digits | ✅ | 6 or 8 |
| OTP type | ✅ | HOTP and TOTP |
| HOTP counter | ✅ | Required for HOTP accounts |
| Batch info | ✅ | Multi-batch transfers supported |

## Security

- **No QR images are saved** — images are processed in-memory and discarded
- **No raw migration payloads are logged** — the redaction layer strips `otpauth-migration://` URIs
- **All fields are validated** — malformed payloads are rejected with clear errors
- **Maximum payload size**: 16 KB (DoS guard)
- **Maximum field lengths**: 256 characters for names, 256 bytes for secrets

## Limitations

### Google cloud-synced accounts

Google Authenticator can sync accounts to your Google Account (introduced in 2023). **These cloud-synced accounts cannot be downloaded through a supported public API.** Sentinel can only import accounts that are present on the phone and can be exported via the QR-code transfer flow.

If your accounts are cloud-synced but not present on any phone, you will need to:
1. Set up Google Authenticator on a new phone
2. Restore from the Google Account backup
3. Use the "Transfer accounts" export flow from that phone

### Period

Google migration payloads do not carry a `period` field — all TOTP accounts are assumed to use the standard 30-second period. If an account uses a non-standard period, you will need to edit it manually after import.

## Alternative import methods

If the Google transfer flow doesn't work, you can also:
- **Scan individual `otpauth://` QR codes** — the per-account QR codes that services provide during 2FA setup
- **Enter the secret manually** — use the "Add account" form and paste the Base32 secret
