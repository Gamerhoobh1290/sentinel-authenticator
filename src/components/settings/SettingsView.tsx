/**
 * Settings view — appearance, security, behavior, data sections.
 */

import { useState } from "react";
import { Card, Button, PasswordField, Dialog } from "@/components/ui";
import { useSettings } from "@/store/settingsStore";
import { ipc } from "@/lib/ipc";
import type { Theme, Density, ClipboardClearDelay, AutoLockDelay } from "@/types";

export function SettingsView() {
  const settings = useSettings();
  const [showChangePw, setShowChangePw] = useState(false);

  return (
    <div className="p-6">
      <div className="mx-auto max-w-2xl space-y-6">
        <h2 className="text-lg font-semibold text-fg">Settings</h2>

        <Card padding="lg">
          <h3 className="mb-4 text-sm font-semibold text-fg">Appearance</h3>
          <div className="space-y-4">
            <SettingRow label="Theme">
              <select
                value={settings.theme}
                onChange={(e) => settings.setTheme(e.target.value as Theme)}
                className="h-9 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
                <option value="system">System</option>
              </select>
            </SettingRow>
            <SettingRow label="Density">
              <select
                value={settings.density}
                onChange={(e) => settings.setDensity(e.target.value as Density)}
                className="h-9 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              >
                <option value="comfortable">Comfortable</option>
                <option value="compact">Compact</option>
              </select>
            </SettingRow>
            <SettingRow label="Reduced motion">
              <Toggle
                checked={settings.reducedMotion}
                onChange={settings.setReducedMotion}
              />
            </SettingRow>
            <SettingRow label="Code formatting">
              <select
                value={settings.codeFormatting}
                onChange={(e) =>
                  settings.update(
                    "codeFormatting",
                    e.target.value as "grouped" | "plain",
                  )
                }
                className="h-9 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              >
                <option value="grouped">Grouped (123 456)</option>
                <option value="plain">Plain (123456)</option>
              </select>
            </SettingRow>
          </div>
        </Card>

        <Card padding="lg">
          <h3 className="mb-4 text-sm font-semibold text-fg">Security</h3>
          <div className="space-y-4">
            <SettingRow label="Change master password">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowChangePw(true)}
              >
                Change…
              </Button>
            </SettingRow>
            <SettingRow label="Auto-lock" hint="Lock after inactivity">
              <select
                value={String(settings.autoLockDelay)}
                onChange={(e) =>
                  settings.update(
                    "autoLockDelay",
                    (e.target.value === "0"
                      ? "never"
                      : Number(e.target.value)) as AutoLockDelay,
                  )
                }
                className="h-9 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              >
                <option value="60000">1 minute</option>
                <option value="300000">5 minutes</option>
                <option value="900000">15 minutes</option>
                <option value="1800000">30 minutes</option>
                <option value="3600000">1 hour</option>
                <option value="never">Never (not recommended)</option>
              </select>
            </SettingRow>
            <SettingRow label="Lock when minimized">
              <Toggle
                checked={settings.lockWhenMinimized}
                onChange={(v) => settings.update("lockWhenMinimized", v)}
              />
            </SettingRow>
            <SettingRow label="Lock when Windows locks">
              <Toggle
                checked={settings.lockWhenWindowsLocks}
                onChange={(v) => settings.update("lockWhenWindowsLocks", v)}
              />
            </SettingRow>
            <SettingRow
              label="Clipboard auto-clear"
              hint="Clear copied codes after delay"
            >
              <select
                value={String(settings.clipboardClearDelay)}
                onChange={(e) =>
                  settings.update(
                    "clipboardClearDelay",
                    (e.target.value === "never"
                      ? "never"
                      : Number(e.target.value)) as ClipboardClearDelay,
                  )
                }
                className="h-9 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              >
                <option value="10000">10 seconds</option>
                <option value="30000">30 seconds (default)</option>
                <option value="60000">60 seconds</option>
                <option value="never">Never (not recommended)</option>
              </select>
            </SettingRow>
          </div>
        </Card>

        <Card padding="lg">
          <h3 className="mb-4 text-sm font-semibold text-fg">Behavior</h3>
          <div className="space-y-4">
            <SettingRow label="Start with Windows">
              <Toggle
                checked={settings.startWithWindows}
                onChange={(v) => settings.update("startWithWindows", v)}
              />
            </SettingRow>
            <SettingRow label="Start minimized">
              <Toggle
                checked={settings.startMinimized}
                onChange={(v) => settings.update("startMinimized", v)}
              />
            </SettingRow>
            <SettingRow label="Minimize to system tray">
              <Toggle
                checked={settings.minimizeToTray}
                onChange={(v) => settings.update("minimizeToTray", v)}
              />
            </SettingRow>
            <SettingRow label="Close to tray">
              <Toggle
                checked={settings.closeToTray}
                onChange={(v) => settings.update("closeToTray", v)}
              />
            </SettingRow>
            <SettingRow label="Always on top">
              <Toggle
                checked={settings.alwaysOnTop}
                onChange={(v) => settings.update("alwaysOnTop", v)}
              />
            </SettingRow>
          </div>
        </Card>

        <Card padding="lg">
          <h3 className="mb-4 text-sm font-semibold text-fg">Data</h3>
          <div className="space-y-4">
            <SettingRow label="Vault location">
              <code className="text-xs text-fg-muted">
                %APPDATA%\Sentinel\vault.bin
              </code>
            </SettingRow>
            <SettingRow
              label="Delete all local data"
              hint="Removes the vault and settings"
            >
              <Button
                variant="danger"
                size="sm"
                onClick={() => {
                  if (
                    window.confirm(
                      "Delete ALL local data? This removes your vault and all accounts. This cannot be undone.",
                    )
                  ) {
                    window.alert(
                      "Please delete the vault file manually at %APPDATA%\\Sentinel\\vault.bin and clear browser storage.",
                    );
                  }
                }}
              >
                Delete all…
              </Button>
            </SettingRow>
          </div>
        </Card>
      </div>
      {showChangePw && <ChangePasswordDialog onClose={() => setShowChangePw(false)} />}
    </div>
  );
}

function SettingRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm font-medium text-fg">{label}</p>
        {hint && <p className="text-xs text-fg-muted">{hint}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-11 rounded-full transition-colors duration-160 ${checked ? "bg-accent" : "bg-bg-subtle border border-border"}`}
    >
      <span
        className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform duration-160 ${checked ? "translate-x-5" : "translate-x-0.5"}`}
      />
    </button>
  );
}

function ChangePasswordDialog({ onClose }: { onClose: () => void }) {
  const [oldPw, setOldPw] = useState("");
  const [newPw, setNewPw] = useState("");
  const [confirmPw, setConfirmPw] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    setError(null);
    if (newPw.length < 8) {
      setError("New password must be at least 8 characters.");
      return;
    }
    if (newPw !== confirmPw) {
      setError("Passwords do not match.");
      return;
    }
    setSubmitting(true);
    try {
      await ipc.changePassword(oldPw, newPw);
      onClose();
    } catch (e) {
      setError(String(e).replace(/^Error:\s*/, ""));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      open
      onClose={onClose}
      title="Change master password"
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={submitting}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={handleSubmit}
            loading={submitting}
            disabled={!oldPw || !newPw || !confirmPw}
          >
            Change password
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <PasswordField
          label="Current password"
          value={oldPw}
          onChange={(e) => setOldPw(e.target.value)}
          autoFocus
        />
        <PasswordField
          label="New password"
          value={newPw}
          onChange={(e) => setNewPw(e.target.value)}
          hint="At least 8 characters."
        />
        <PasswordField
          label="Confirm new password"
          value={confirmPw}
          onChange={(e) => setConfirmPw(e.target.value)}
        />
        {error && (
          <p
            role="alert"
            className="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
          >
            {error}
          </p>
        )}
      </div>
    </Dialog>
  );
}
