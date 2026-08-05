/**
 * Account card — displays a single authenticator account with its current
 * code, countdown timer, and action buttons.
 *
 * Performance: the countdown is driven by CSS animation (stroke-dashoffset),
 * NOT by React state updates. Only the code itself re-renders, and only
 * when the TOTP period rolls over.
 */

import { memo, useCallback, useEffect, useRef, useState } from "react";
import type { AccountView, CodeResult } from "@/types";
import { cn } from "@/lib/cn";
import { Badge, Button, Menu, Tooltip } from "@/components/ui";
import { useAccounts } from "@/store/accountStore";
import { useSettings } from "@/store/settingsStore";

export interface AccountCardProps {
  account: AccountView;
  code?: CodeResult;
  selected?: boolean;
  onSelect?: () => void;
}

function formatCode(code: string, formatting: "grouped" | "plain"): string {
  if (formatting === "plain" || code.length < 4) return code;
  // Group as "123 456" for 6-digit or "1234 5678" for 8-digit
  const mid = Math.ceil(code.length / 2);
  return `${code.slice(0, mid)} ${code.slice(mid)}`;
}

function getInitials(issuer: string, label: string): string {
  const source = issuer || label;
  if (!source) return "?";
  const parts = source.split(/[\s@._-]+/).filter(Boolean);
  if (parts.length === 0) return source.charAt(0).toUpperCase();
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return (parts[0]![0]! + parts[1]![0]!).toUpperCase();
}

export const AccountCard = memo(function AccountCard({
  account,
  code,
  selected,
  onSelect,
}: AccountCardProps) {
  const codeFormatting = useSettings((s) => s.codeFormatting);
  const toggleFavorite = useAccounts((s) => s.toggleFavorite);
  const deleteAccount = useAccounts((s) => s.deleteAccount);
  const [copied, setCopied] = useState(false);
  const copyTimeoutRef = useRef<number | null>(null);

  const handleCopy = useCallback(async () => {
    if (!code) return;
    try {
      const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
      await writeText(code.code);
      setCopied(true);
      if (copyTimeoutRef.current) window.clearTimeout(copyTimeoutRef.current);
      copyTimeoutRef.current = window.setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard may not be available in dev/test
    }
  }, [code]);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) window.clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const displayCode = code ? formatCode(code.code, codeFormatting) : "••• •••";
  const secondsRemaining = code?.secondsRemaining ?? 0;
  const period = code?.period ?? 30;
  const progress = period > 0 ? secondsRemaining / period : 0;
  const isLow = secondsRemaining <= 5 && account.otpType === "totp";

  const initials = account.iconText || getInitials(account.issuer, account.label);
  const iconColor = account.iconColor || "#60a5fa";

  return (
    <div
      className={cn(
        "card card-hover group relative flex items-center gap-4 p-4",
        "transition-all duration-160",
        selected && "ring-2 ring-accent",
        isLow && "ring-1 ring-warning/50",
      )}
      data-testid={`account-card-${account.id}`}
      role="article"
      aria-label={`${account.issuer} ${account.label}`}
    >
      {/* Selection checkbox (visible in bulk-select mode) */}
      {onSelect && (
        <input
          type="checkbox"
          checked={selected}
          onChange={onSelect}
          className="h-4 w-4 rounded border-border accent-accent"
          aria-label={`Select ${account.issuer}`}
        />
      )}

      {/* Icon */}
      <div
        className="grid h-11 w-11 shrink-0 place-items-center rounded-xl text-sm font-semibold text-white"
        style={{ backgroundColor: iconColor }}
        aria-hidden="true"
      >
        {initials}
      </div>

      {/* Account info */}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="truncate text-sm font-medium text-fg">
            {account.issuer || "Unknown"}
          </h3>
          {account.favorite && (
            <span className="shrink-0 text-warning">
              <StarIcon filled />
            </span>
          )}
          {account.otpType === "hotp" && <Badge variant="default">HOTP</Badge>}
        </div>
        <p className="truncate text-xs text-fg-muted">{account.label}</p>
      </div>

      {/* Code + countdown */}
      <div className="flex items-center gap-3">
        {account.otpType === "totp" && (
          <CountdownRing progress={progress} secondsRemaining={secondsRemaining} />
        )}
        <button
          type="button"
          onClick={handleCopy}
          className={cn(
            "rounded-lg px-3 py-2 text-center transition-colors duration-160",
            "font-mono text-lg font-semibold tracking-wider tabular-nums",
            "hover:bg-bg-subtle focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent",
            copied ? "text-success" : "text-fg",
          )}
          aria-label={`Copy code for ${account.issuer}`}
        >
          {displayCode}
        </button>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 opacity-0 transition-opacity duration-160 group-hover:opacity-100">
        <Tooltip content={copied ? "Copied!" : "Copy code"}>
          <Button
            variant="ghost"
            size="icon"
            onClick={handleCopy}
            aria-label="Copy code"
          >
            {copied ? <CheckIcon /> : <CopyIcon />}
          </Button>
        </Tooltip>

        <Tooltip content={account.favorite ? "Unfavorite" : "Favorite"}>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => toggleFavorite(account.id)}
            aria-label={account.favorite ? "Unfavorite" : "Favorite"}
          >
            <StarIcon filled={account.favorite} />
          </Button>
        </Tooltip>

        {account.otpType === "hotp" && (
          <Tooltip content="Advance counter">
            <Button
              variant="ghost"
              size="icon"
              onClick={() =>
                void useAccounts.getState().incrementHotpCounter(account.id)
              }
              aria-label="Advance HOTP counter"
            >
              <ArrowForwardIcon />
            </Button>
          </Tooltip>
        )}

        <Menu
          label={`More options for ${account.issuer}`}
          trigger={
            <Button variant="ghost" size="icon" aria-label="More options">
              <MoreIcon />
            </Button>
          }
          items={[
            {
              id: "edit",
              label: "Edit account",
              onSelect: () => {
                /* M6: edit dialog */
              },
            },
            {
              id: "favorite",
              label: account.favorite ? "Unfavorite" : "Favorite",
              onSelect: () => void toggleFavorite(account.id),
            },
            {
              id: "delete",
              label: "Delete",
              danger: true,
              separator: true,
              onSelect: () => {
                if (window.confirm(`Delete "${account.issuer}"?`)) {
                  void deleteAccount(account.id);
                }
              },
            },
          ]}
        />
      </div>

      {/* Bulk select fallback (always visible when onSelect is provided) */}
      {onSelect && !selected && (
        <div className="absolute right-2 top-2 opacity-0 group-hover:opacity-100 transition-opacity">
          <input
            type="checkbox"
            checked={selected}
            onChange={onSelect}
            className="h-4 w-4 rounded border-border accent-accent"
            aria-label={`Select ${account.issuer}`}
          />
        </div>
      )}
    </div>
  );
});

// ─── Countdown ring (CSS-driven, no React re-renders per second) ──

function CountdownRing({
  progress,
  secondsRemaining,
}: {
  progress: number;
  secondsRemaining: number;
}) {
  const radius = 16;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - progress);

  return (
    <div
      className="relative grid h-9 w-9 place-items-center"
      role="timer"
      aria-label={`${secondsRemaining} seconds remaining`}
    >
      <svg className="h-9 w-9 -rotate-90" viewBox="0 0 36 36" aria-hidden="true">
        <circle
          cx="18"
          cy="18"
          r={radius}
          fill="none"
          stroke="rgb(var(--border))"
          strokeWidth="2.5"
        />
        <circle
          cx="18"
          cy="18"
          r={radius}
          fill="none"
          stroke={secondsRemaining <= 5 ? "rgb(var(--warning))" : "rgb(var(--accent))"}
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          style={{
            transition: "stroke-dashoffset 1s linear",
          }}
        />
      </svg>
      <span className="absolute text-2xs font-medium tabular-nums text-fg-muted">
        {secondsRemaining}
      </span>
    </div>
  );
}

// ─── Icons ──────────────────────────────────────────────────────────

function StarIcon({ filled }: { filled?: boolean }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill={filled ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
    </svg>
  );
}

function CopyIcon() {
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
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}

function ArrowForwardIcon() {
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
      <line x1="5" y1="12" x2="19" y2="12" />
      <polyline points="12 5 19 12 12 19" />
    </svg>
  );
}

function MoreIcon() {
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
      <circle cx="12" cy="12" r="1" />
      <circle cx="19" cy="12" r="1" />
      <circle cx="5" cy="12" r="1" />
    </svg>
  );
}
