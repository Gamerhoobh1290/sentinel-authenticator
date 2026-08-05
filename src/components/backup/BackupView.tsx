/**
 * Backup view — create and restore encrypted backups.
 */

import { useState } from "react";
import { Card, Button, PasswordField, Dialog, Badge } from "@/components/ui";
import { ipc, type BackupPreviewEntry } from "@/lib/ipc";

export function BackupView() {
  const [showCreate, setShowCreate] = useState(false);
  const [showRestore, setShowRestore] = useState(false);

  return (
    <div className="p-6">
      <div className="mx-auto max-w-2xl space-y-6">
        <h2 className="text-lg font-semibold text-fg">Backup & Restore</h2>

        <Card padding="lg">
          <h3 className="mb-2 text-sm font-semibold text-fg">Create backup</h3>
          <p className="mb-4 text-sm text-fg-muted">
            Create an encrypted backup of all your accounts. The backup uses a separate
            backup password — it is independent of your vault master password.
          </p>
          <Button variant="primary" onClick={() => setShowCreate(true)}>
            Create backup…
          </Button>
        </Card>

        <Card padding="lg">
          <h3 className="mb-2 text-sm font-semibold text-fg">Restore backup</h3>
          <p className="mb-4 text-sm text-fg-muted">
            Restore accounts from a previously created backup file. Preview the contents
            before importing. Accounts are merged into your current vault.
          </p>
          <Button variant="secondary" onClick={() => setShowRestore(true)}>
            Restore backup…
          </Button>
        </Card>

        <Card padding="lg">
          <h3 className="mb-2 text-sm font-semibold text-fg">Important</h3>
          <ul className="space-y-1.5 text-sm text-fg-muted">
            <li>Backups are encrypted with AES-256-GCM + Argon2id.</li>
            <li>The backup password is NOT your vault master password.</li>
            <li>
              If you lose both your master password and your backup, your accounts
              cannot be recovered.
            </li>
            <li>Store the backup file and password in separate secure locations.</li>
          </ul>
        </Card>
      </div>
      {showCreate && <CreateBackupDialog onClose={() => setShowCreate(false)} />}
      {showRestore && <RestoreBackupDialog onClose={() => setShowRestore(false)} />}
    </div>
  );
}

function CreateBackupDialog({ onClose }: { onClose: () => void }) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [done, setDone] = useState(false);

  const handleBrowse = async () => {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const selected = await save({
        defaultPath: "sentinel-backup.sentinelbak",
        filters: [{ name: "Sentinel Backup", extensions: ["sentinelbak"] }],
      });
      if (selected) setPath(selected);
    } catch {
      /* test */
    }
  };

  const handleSubmit = async () => {
    setError(null);
    if (password.length < 8) {
      setError("Backup password must be at least 8 characters.");
      return;
    }
    if (password !== confirm) {
      setError("Passwords do not match.");
      return;
    }
    if (!path) {
      setError("Choose a file location.");
      return;
    }
    setSubmitting(true);
    try {
      await ipc.createBackup(path, password);
      setDone(true);
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
      title="Create backup"
      size="md"
      footer={
        done ? (
          <Button variant="primary" onClick={onClose}>
            Done
          </Button>
        ) : (
          <>
            <Button variant="ghost" onClick={onClose} disabled={submitting}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleSubmit}
              loading={submitting}
              disabled={!password || !confirm || !path}
            >
              Create backup
            </Button>
          </>
        )
      }
    >
      {done ? (
        <div className="py-4 text-center">
          <p className="text-sm font-medium text-success">
            Backup created successfully!
          </p>
          <p className="mt-1 text-xs text-fg-muted">{path}</p>
        </div>
      ) : (
        <div className="space-y-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-fg-muted">
              Backup file location
            </label>
            <div className="flex gap-2">
              <input
                readOnly
                value={path}
                placeholder="Choose a location…"
                className="h-9 flex-1 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              />
              <Button variant="secondary" size="sm" onClick={handleBrowse}>
                Browse…
              </Button>
            </div>
          </div>
          <PasswordField
            label="Backup password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            hint="At least 8 characters. Independent from your vault password."
          />
          <PasswordField
            label="Confirm password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
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
      )}
    </Dialog>
  );
}

function RestoreBackupDialog({ onClose }: { onClose: () => void }) {
  const [path, setPath] = useState("");
  const [password, setPassword] = useState("");
  const [preview, setPreview] = useState<BackupPreviewEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [restored, setRestored] = useState(0);

  const handleBrowse = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        filters: [{ name: "Sentinel Backup", extensions: ["sentinelbak"] }],
      });
      if (typeof selected === "string") setPath(selected);
    } catch {
      /* test */
    }
  };

  const handlePreview = async () => {
    setError(null);
    setSubmitting(true);
    try {
      const result = await ipc.previewBackup(path, password);
      setPreview(result);
    } catch (e) {
      setError(String(e).replace(/^Error:\s*/, ""));
    } finally {
      setSubmitting(false);
    }
  };

  const handleRestore = async () => {
    setError(null);
    setSubmitting(true);
    try {
      const count = await ipc.restoreBackup(path, password);
      setRestored(count);
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
      title="Restore backup"
      size="md"
      footer={
        restored > 0 ? (
          <Button variant="primary" onClick={onClose}>
            Done
          </Button>
        ) : (
          <>
            <Button variant="ghost" onClick={onClose} disabled={submitting}>
              Cancel
            </Button>
            {preview ? (
              <Button variant="primary" onClick={handleRestore} loading={submitting}>
                Restore {preview.length} accounts
              </Button>
            ) : (
              <Button
                variant="primary"
                onClick={handlePreview}
                loading={submitting}
                disabled={!path || !password}
              >
                Preview
              </Button>
            )}
          </>
        )
      }
    >
      {restored > 0 ? (
        <div className="py-4 text-center">
          <p className="text-sm font-medium text-success">
            {restored} account(s) restored successfully!
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-fg-muted">
              Backup file
            </label>
            <div className="flex gap-2">
              <input
                readOnly
                value={path}
                placeholder="Choose a backup file…"
                className="h-9 flex-1 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg"
              />
              <Button variant="secondary" size="sm" onClick={handleBrowse}>
                Browse…
              </Button>
            </div>
          </div>
          <PasswordField
            label="Backup password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          {error && (
            <p
              role="alert"
              className="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
            >
              {error}
            </p>
          )}
          {preview && (
            <div>
              <p className="mb-2 text-sm font-medium text-fg">
                Backup contents ({preview.length} accounts):
              </p>
              <div className="max-h-48 space-y-1 overflow-y-auto">
                {preview.map((entry, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 rounded-lg border border-border bg-bg-subtle px-3 py-1.5"
                  >
                    <Badge variant="default">{entry.otpType.toUpperCase()}</Badge>
                    <span className="text-sm font-medium text-fg">{entry.issuer}</span>
                    <span className="text-sm text-fg-muted">{entry.label}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </Dialog>
  );
}
