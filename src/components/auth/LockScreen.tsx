import { useState, type FormEvent } from "react";
import { SentinelLogo, PasswordField, Button } from "@/components/ui";

/**
 * Placeholder lock screen.
 *
 * Real unlock flow arrives in M3 once the Rust vault module is ready.
 * For M1 we just render the visual design and a disabled "Unlock" button.
 */
export interface LockScreenProps {
  /** First-launch flow: prompt the user to create a master password. */
  mode: "create" | "unlock";
  onSubmit: (password: string) => void;
  /** When true, render a non-interactive rate-limited state. */
  rateLimited?: boolean;
  rateLimitedSeconds?: number;
  errorMessage?: string | null;
  loading?: boolean;
}

export function LockScreen({
  mode,
  onSubmit,
  rateLimited,
  rateLimitedSeconds,
  errorMessage,
  loading,
}: LockScreenProps) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");

  const isCreate = mode === "create";
  const passwordsMatch = !isCreate || password === confirm;
  const passwordLongEnough = password.length >= 8;
  const canSubmit =
    !rateLimited && !loading && passwordLongEnough && (!isCreate || passwordsMatch);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    onSubmit(password);
  };

  return (
    <div
      className="flex h-full w-full items-center justify-center bg-bg p-4"
      data-testid="lock-screen"
      role="main"
      aria-label="Sentinel lock screen"
    >
      <div className="w-full max-w-sm animate-slide-up">
        <div className="mb-8 flex flex-col items-center gap-3 text-center">
          <span className="text-accent" aria-hidden="true">
            <SentinelLogo size={48} />
          </span>
          <div>
            <h1 className="text-xl font-semibold tracking-tight">
              Sentinel Authenticator
            </h1>
            <p className="mt-1 text-sm text-fg-muted">
              {isCreate
                ? "Create a master password to protect your accounts."
                : "Enter your master password to unlock your vault."}
            </p>
          </div>
        </div>

        <form
          onSubmit={handleSubmit}
          className="card flex flex-col gap-4 p-5"
          autoComplete="off"
        >
          <PasswordField
            label="Master password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="••••••••••"
            autoFocus
            disabled={rateLimited || loading}
            error={
              password.length > 0 && !passwordLongEnough
                ? "Use at least 8 characters."
                : undefined
            }
            hint={
              isCreate
                ? "Pick a strong password. There is no recovery if lost."
                : undefined
            }
          />

          {isCreate && (
            <PasswordField
              label="Confirm password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              placeholder="••••••••••"
              disabled={rateLimited || loading}
              error={
                confirm.length > 0 && !passwordsMatch
                  ? "Passwords do not match."
                  : undefined
              }
            />
          )}

          {errorMessage && (
            <p
              role="alert"
              className="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
            >
              {errorMessage}
            </p>
          )}

          {rateLimited && (
            <p
              role="alert"
              className="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning"
            >
              Too many attempts. Please wait
              {rateLimitedSeconds ? ` ${rateLimitedSeconds}s` : ""} before trying again.
            </p>
          )}

          <Button
            type="submit"
            variant="primary"
            size="lg"
            disabled={!canSubmit}
            loading={loading}
            className="w-full"
          >
            {isCreate ? "Create vault" : "Unlock"}
          </Button>

          <p className="text-center text-xs text-fg-subtle">
            Your password is never stored. It unlocks the vault on this device only.
          </p>
        </form>

        <p className="mt-6 text-center text-xs text-fg-subtle">
          Sentinel runs entirely offline. No accounts, no cloud, no telemetry.
        </p>
      </div>
    </div>
  );
}
