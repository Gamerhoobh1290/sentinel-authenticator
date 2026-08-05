import { type ReactNode } from "react";
import { cn } from "@/lib/cn";
import { SentinelLogo, Button, Tooltip } from "@/components/ui";

export type NavSection = "accounts" | "favorites" | "import" | "backup" | "settings";

export interface AppShellProps {
  active: NavSection;
  onNavigate: (section: NavSection) => void;
  onLock: () => void;
  onAddAccount: () => void;
  children: ReactNode;
  /** Compact mode shrinks the sidebar to icons only. */
  compact?: boolean;
  /** Top-right search bar slot. */
  searchSlot?: ReactNode;
}

interface NavItem {
  id: NavSection;
  label: string;
  icon: ReactNode;
}

const NAV_ITEMS: NavItem[] = [
  {
    id: "accounts",
    label: "Accounts",
    icon: <AccountsIcon />,
  },
  {
    id: "favorites",
    label: "Favorites",
    icon: <StarIcon />,
  },
  {
    id: "import",
    label: "Import",
    icon: <ImportIcon />,
  },
  {
    id: "backup",
    label: "Backup",
    icon: <BackupIcon />,
  },
  {
    id: "settings",
    label: "Settings",
    icon: <SettingsIcon />,
  },
];

export function AppShell({
  active,
  onNavigate,
  onLock,
  onAddAccount,
  children,
  compact = false,
  searchSlot,
}: AppShellProps) {
  return (
    <div
      className="flex h-full w-full bg-bg text-fg"
      data-testid="app-shell"
      role="application"
      aria-label="Sentinel Authenticator"
    >
      {/* Sidebar */}
      <aside
        className={cn(
          "flex flex-col border-r border-border bg-bg-elevated",
          compact ? "w-[52px]" : "w-[200px]",
        )}
        aria-label="Primary navigation"
      >
        {/* Brand */}
        <div className="flex h-14 items-center gap-2.5 px-4 border-b border-border">
          <span className="text-accent">
            <SentinelLogo size={22} />
          </span>
          {!compact && (
            <span className="text-sm font-semibold tracking-tight">Sentinel</span>
          )}
        </div>

        {/* Nav */}
        <nav className="flex-1 px-2 py-3" aria-label="Main">
          <ul className="flex flex-col gap-0.5">
            {NAV_ITEMS.map((item) => {
              const isActive = item.id === active;
              return (
                <li key={item.id}>
                  <button
                    type="button"
                    onClick={() => onNavigate(item.id)}
                    aria-current={isActive ? "page" : undefined}
                    title={compact ? item.label : undefined}
                    className={cn(
                      "flex w-full items-center gap-3 rounded-lg px-2.5 h-9 text-sm",
                      "transition-colors duration-160",
                      "focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2",
                      isActive
                        ? "bg-bg-subtle text-fg font-medium"
                        : "text-fg-muted hover:text-fg hover:bg-bg-subtle",
                      compact && "justify-center px-0",
                    )}
                  >
                    <span className="shrink-0">{item.icon}</span>
                    {!compact && <span className="truncate">{item.label}</span>}
                  </button>
                </li>
              );
            })}
          </ul>
        </nav>

        {/* Add + Lock */}
        <div className="border-t border-border p-2 space-y-1">
          <Tooltip content="Add account (Ctrl+N)" side="right">
            <Button
              variant="primary"
              size={compact ? "icon" : "md"}
              onClick={onAddAccount}
              className={compact ? "w-full" : "w-full"}
              aria-label="Add account"
            >
              {compact ? (
                <PlusIcon />
              ) : (
                <>
                  <PlusIcon /> Add
                </>
              )}
            </Button>
          </Tooltip>
          <Tooltip content="Lock vault (Ctrl+L)" side="right">
            <Button
              variant="ghost"
              size={compact ? "icon" : "md"}
              onClick={onLock}
              className={compact ? "w-full" : "w-full"}
              aria-label="Lock vault"
            >
              <LockIcon />
              {!compact && <span className="ml-2">Lock</span>}
            </Button>
          </Tooltip>
        </div>
      </aside>

      {/* Main area */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Top bar */}
        <header className="flex h-14 items-center gap-3 border-b border-border px-4">
          {searchSlot}
        </header>

        {/* Content */}
        <main
          className="flex-1 overflow-y-auto"
          tabIndex={-1}
          aria-label="Main content"
        >
          {children}
        </main>
      </div>
    </div>
  );
}

/* ─── Icons ─── */
function AccountsIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="3" y="4" width="18" height="4" rx="1" />
      <rect x="3" y="11" width="18" height="4" rx="1" />
      <rect x="3" y="18" width="18" height="3" rx="1" />
    </svg>
  );
}
function StarIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
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
function ImportIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  );
}
function BackupIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <ellipse cx="12" cy="5" rx="9" ry="3" />
      <path d="M3 5v14a9 3 0 0 0 18 0V5" />
      <path d="M3 12a9 3 0 0 0 18 0" />
    </svg>
  );
}
function SettingsIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}
function PlusIcon() {
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
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}
function LockIcon() {
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
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>
  );
}
