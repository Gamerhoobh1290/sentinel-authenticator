# Sentinel Authenticator — Manual QA Checklist

This is a repeatable manual QA checklist for verifying Sentinel Authenticator before each release. Run through every item on a clean Windows 10 or Windows 11 machine.

## Prerequisites

- Windows 10 (build 19041+) or Windows 11
- WebView2 Runtime installed (preinstalled on Windows 11)
- A phone with Google Authenticator installed and at least 2 test accounts
- Test accounts that you can safely lose (do NOT use your real 2FA accounts for testing)

## Checklist

### 1. First launch

- [ ] Download and run `Sentinel-Authenticator-Setup.exe`
- [ ] Windows SmartScreen shows a warning (expected — installer is unsigned)
- [ ] Click "More info" → "Run anyway"
- [ ] Installer completes without errors
- [ ] Sentinel launches and shows the lock screen in "Create" mode
- [ ] Application title bar shows "Sentinel Authenticator"

### 2. Master password creation

- [ ] Enter a password shorter than 8 characters → "Create vault" button is disabled
- [ ] Enter mismatched passwords → error message appears
- [ ] Enter matching passwords ≥ 8 characters → "Create vault" button enables
- [ ] Click "Create vault" → vault is created, main interface appears
- [ ] Verify `%APPDATA%\Sentinel\vault.bin` exists on disk

### 3. Lock and unlock

- [ ] Click the "Lock" button in the sidebar → lock screen appears
- [ ] Enter wrong password → "Incorrect password" error
- [ ] Enter correct password → vault unlocks, accounts list appears
- [ ] Press Ctrl+L → vault locks
- [ ] Press Ctrl+L again (from lock screen) → no effect

### 4. Manual account creation

- [ ] Click "Add" or press Ctrl+N → Add Account dialog opens
- [ ] Fill in: Issuer="Test", Account="user@test.com", Secret="JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP"
- [ ] Click "Add account" → dialog closes, account appears in list
- [ ] Verify a 6-digit code is displayed and counting down

### 5. Normal QR import

- [ ] Click "Import" in sidebar → Import section appears
- [ ] Click "Scan QR code" → QR dialog opens
- [ ] Use the "Upload image" tab → upload a `otpauth://` QR image
- [ ] Account is imported and appears in the list

### 6. Google transfer QR import

- [ ] On your phone, open Google Authenticator
- [ ] Menu → Transfer accounts → Export accounts
- [ ] Select test accounts, tap Next
- [ ] In Sentinel, scan the QR code(s)
- [ ] Accounts appear in the import preview
- [ ] Click "Done" → imported accounts appear in the list

### 7. Multi-QR Google transfer

- [ ] Export 5+ accounts from Google Authenticator (generates 2+ QR codes)
- [ ] Scan the first QR → "1 of 2 scanned" progress message
- [ ] Scan the second QR → all accounts imported
- [ ] Verify all accounts appear with correct issuers

### 8. Duplicate handling

- [ ] Import the same account twice → both entries appear (duplicate)
- [ ] (Future: duplicate detection should prompt to skip or merge)

### 9. Code accuracy

- [ ] Compare Sentinel's code for an account with Google Authenticator's code for the same account
- [ ] Codes must match exactly

### 10. Code transition at 30-second boundary

- [ ] Watch a TOTP code at the 30-second mark
- [ ] Code must change exactly when the countdown ring reaches 0
- [ ] No flickering or duplicate codes

### 11. Search and organisation

- [ ] Type in the search box → list filters instantly by issuer/account
- [ ] Click "Favorites" toggle → only favorite accounts shown
- [ ] Click a star icon on an account → it becomes a favorite
- [ ] Change sort to "By issuer" → list re-sorts alphabetically

### 12. Clipboard clearing

- [ ] Click a code → code is copied, "Copied!" feedback appears
- [ ] Paste immediately → code appears
- [ ] Wait 30 seconds → paste → clipboard is cleared (or shows previous content)
- [ ] Copy something else within 30s → Sentinel does NOT clobber it

### 13. Automatic locking

- [ ] Set auto-lock to 1 minute in Settings
- [ ] Leave the app idle for 65 seconds → vault locks automatically
- [ ] Set auto-lock to "Never" → vault does not auto-lock

### 14. Theme switching

- [ ] Settings → Theme → Light → UI switches to light mode
- [ ] Settings → Theme → Dark → UI switches to dark mode
- [ ] Settings → Theme → System → follows OS theme

### 15. Keyboard navigation

- [ ] Press Ctrl+K → search field focuses
- [ ] Press Ctrl+N → Add Account dialog opens
- [ ] Press Ctrl+I → Import dialog opens
- [ ] Press Ctrl+L → vault locks
- [ ] Press Escape in a dialog → dialog closes
- [ ] Tab through the account list → focus indicators are visible

### 16. Camera permission denial

- [ ] Deny camera access in Windows Settings
- [ ] Open QR import → Camera tab → "Start camera"
- [ ] Clear error message appears: "Could not access the camera"
- [ ] "Upload image" tab still works as fallback

### 17. Backup creation

- [ ] Go to Backup section → "Create backup"
- [ ] Choose a file location, enter backup password
- [ ] Click "Create backup" → success message
- [ ] Verify `.sentinelbak` file exists at the chosen location

### 18. Backup restoration

- [ ] Go to Backup → "Restore backup"
- [ ] Choose the backup file, enter backup password
- [ ] Click "Preview" → list of accounts appears (no secrets)
- [ ] Click "Restore" → accounts are merged into vault

### 19. Incorrect backup password

- [ ] Try to restore with wrong password → "Invalid backup password" error
- [ ] Original vault is not affected

### 20. Application restart

- [ ] Close Sentinel completely
- [ ] Reopen → lock screen appears (vault is locked)
- [ ] Enter master password → vault unlocks, accounts are still there

### 21. System tray behaviour

- [ ] Minimize Sentinel → tray icon appears
- [ ] Right-click tray icon → menu shows Open/Lock/Add/Quit
- [ ] Click "Open" → window restores
- [ ] Click "Lock" → vault locks
- [ ] Click "Quit" → application exits

### 22. Windows installer installation and uninstallation

- [ ] Run the installer → installs to `%LOCALAPPDATA%\Sentinel`
- [ ] Start menu entry appears
- [ ] Desktop shortcut appears (if selected)
- [ ] Uninstall from Settings → Apps → Sentinel → Uninstall
- [ ] Verify `%APPDATA%\Sentinel\vault.bin` still exists (user data not removed by uninstall)
- [ ] Manually delete `%APPDATA%\Sentinel\` to fully clean up

## Sign-off

- Tester name: _______________
- Date: _______________
- Version: _______________
- All items passed: ☐ Yes ☐ No (list failures below)

### Failures

(Describe any failures here)
