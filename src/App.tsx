/**
 * Sentinel Authenticator — root component.
 *
 * Wires together the lock screen, main account interface, add/import
 * dialogs, and keyboard shortcuts.
 */

import { useState, useEffect, useCallback } from "react";
import { AppShell, type NavSection } from "@/components/layout/AppShell";
import { LockScreen } from "@/components/auth/LockScreen";
import { AccountListView } from "@/components/accounts/AccountListView";
import { AddAccountDialog } from "@/components/accounts/AddAccountDialog";
import { QrImportDialog } from "@/components/import/QrImportDialog";
import { SettingsView } from "@/components/settings/SettingsView";
import { BackupView } from "@/components/backup/BackupView";
import { TextField, Card, Button } from "@/components/ui";
import { useSettings } from "@/store/settingsStore";
import { useVault } from "@/store/vaultStore";
import { useAccounts } from "@/store/accountStore";
import { useTheme, useReducedMotion } from "@/hooks/useTheme";
import { useAutoLock } from "@/hooks/useAutoLock";
import { ipc } from "@/lib/ipc";

export default function App() {
  const theme = useSettings((s) => s.theme);
  const density = useSettings((s) => s.density);
  const reducedMotion = useSettings((s) => s.reducedMotion);
  useTheme(theme);
  useReducedMotion(reducedMotion);
  useAutoLock();

  const vaultState = useVault((s) => s.state);
  const setVaultState = useVault((s) => s.setState);
  const setError = useVault((s) => s.setError);
  const errorMessage = useVault((s) => s.lastError);

  const [section, setSection] = useState<NavSection>("accounts");
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [bootstrapped, setBootstrapped] = useState(false);

  const loadAccounts = useAccounts((s) => s.loadAccounts);
  const setSearch = useAccounts((s) => s.setSearch);

  // Bootstrap: check if vault exists on first launch
  useEffect(() => {
    void (async () => {
      try {
        const exists = await ipc.vaultExists();
        setVaultState(exists ? "locked" : "uninitialised");
      } catch {
        // IPC not available (dev/test) — default to locked
        setVaultState("locked");
      }
      setBootstrapped(true);
    })();
  }, [setVaultState]);

  // Sync search query from the shell to the account store
  useEffect(() => {
    setSearch(searchQuery);
  }, [searchQuery, setSearch]);

  const handleLock = useCallback(() => {
    void ipc.vaultLock().catch(() => {});
    setVaultState("locked");
  }, [setVaultState]);

  // Keyboard shortcuts
  useEffect(() => {
    if (vaultState !== "unlocked") return;

    const handleKey = (e: KeyboardEvent) => {
      // Don't interfere with typing in inputs
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT"
      ) {
        if (e.key === "Escape") target.blur();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
      } else if ((e.ctrlKey || e.metaKey) && e.key === "n") {
        e.preventDefault();
        setShowAddDialog(true);
      } else if ((e.ctrlKey || e.metaKey) && e.key === "i") {
        e.preventDefault();
        setShowImportDialog(true);
      } else if ((e.ctrlKey || e.metaKey) && e.key === "l") {
        e.preventDefault();
        handleLock();
      }
    };

    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [vaultState, handleLock]);

  const handleUnlockSubmit = useCallback(
    async (password: string) => {
      setVaultState("unlocking");
      setError(null);
      try {
        if (vaultState === "uninitialised") {
          await ipc.vaultCreate(password);
        } else {
          await ipc.vaultUnlock(password);
        }
        setVaultState("unlocked");
        await loadAccounts();
      } catch (e) {
        setError(String(e).replace(/^Error:\s*/, ""));
        setVaultState("locked");
      }
    },
    [vaultState, setVaultState, setError, loadAccounts],
  );

  const handleAddAccount = useCallback(() => {
    setShowAddDialog(true);
  }, []);

  if (!bootstrapped) {
    return (
      <div className="flex h-full items-center justify-center bg-bg">
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-accent border-r-transparent" />
      </div>
    );
  }

  if (vaultState !== "unlocked") {
    return (
      <LockScreen
        mode={vaultState === "uninitialised" ? "create" : "unlock"}
        onSubmit={handleUnlockSubmit}
        loading={vaultState === "unlocking"}
        errorMessage={errorMessage}
      />
    );
  }

  return (
    <>
      <AppShell
        active={section}
        onNavigate={setSection}
        onLock={handleLock}
        onAddAccount={handleAddAccount}
        compact={density === "compact"}
        searchSlot={
          <TextField
            type="search"
            placeholder="Search accounts… (Ctrl+K)"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search accounts"
            className="max-w-md"
            leading={<SearchIcon />}
          />
        }
      >
        {section === "accounts" && <AccountListView />}
        {section === "favorites" && <FavoritesView />}
        {section === "import" && (
          <ImportSection onOpenImport={() => setShowImportDialog(true)} />
        )}
        {section === "backup" && <BackupView />}
        {section === "settings" && <SettingsView />}
      </AppShell>

      <AddAccountDialog
        open={showAddDialog}
        onClose={() => setShowAddDialog(false)}
        onAdded={() => void loadAccounts()}
      />
      <QrImportDialog
        open={showImportDialog}
        onClose={() => setShowImportDialog(false)}
        onImported={() => void loadAccounts()}
      />
    </>
  );
}

function FavoritesView() {
  const toggleFavoriteFilter = useAccounts((s) => s.toggleFavoriteFilter);
  const showFavoritesOnly = useAccounts((s) => s.showFavoritesOnly);

  useEffect(() => {
    if (!showFavoritesOnly) toggleFavoriteFilter();
    return () => {
      if (showFavoritesOnly) toggleFavoriteFilter();
    };
  }, [showFavoritesOnly, toggleFavoriteFilter]);

  return <AccountListView />;
}

function ImportSection({ onOpenImport }: { onOpenImport: () => void }) {
  return (
    <div className="p-6">
      <div className="mx-auto max-w-2xl space-y-4">
        <Card padding="lg">
          <h2 className="text-lg font-semibold text-fg">Import accounts</h2>
          <p className="mt-1 text-sm text-fg-muted">
            Import accounts from Google Authenticator or from individual QR codes.
          </p>
          <div className="mt-4 flex gap-2">
            <Button variant="primary" onClick={onOpenImport}>
              Scan QR code
            </Button>
          </div>
        </Card>

        <Card padding="lg">
          <h3 className="text-sm font-semibold text-fg">Supported formats</h3>
          <ul className="mt-2 space-y-1.5 text-sm text-fg-muted">
            <li>
              • <strong>otpauth://totp/...</strong> — standard TOTP QR codes
            </li>
            <li>
              • <strong>otpauth://hotp/...</strong> — standard HOTP QR codes
            </li>
            <li>
              • <strong>otpauth-migration://</strong> — Google Authenticator transfer
              payloads
            </li>
            <li>• Multi-batch Google transfers (multiple QR codes)</li>
            <li>• Camera scanning or image file upload (PNG, JPEG, WebP)</li>
          </ul>
        </Card>

        <Card padding="lg">
          <h3 className="text-sm font-semibold text-fg">
            How to export from Google Authenticator
          </h3>
          <ol className="mt-2 space-y-1.5 text-sm text-fg-muted">
            <li>1. Open Google Authenticator on your phone</li>
            <li>
              2. Tap the menu → <strong>Transfer accounts</strong>
            </li>
            <li>
              3. Tap <strong>Export accounts</strong>
            </li>
            <li>4. Show the generated QR code(s) to your PC camera</li>
            <li>5. Scan each QR code in Sentinel's import dialog</li>
          </ol>
          <p className="mt-3 rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            ⚠️ Keep your original authenticator until you have verified that imported
            codes work correctly.
          </p>
        </Card>
      </div>
    </div>
  );
}

function SearchIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  );
}
