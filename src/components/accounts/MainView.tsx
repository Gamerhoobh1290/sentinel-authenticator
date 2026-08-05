import { Card, Badge, Button } from "@/components/ui";
import { SentinelLogo } from "@/components/ui";

/**
 * Placeholder main view shown after unlock (M1).
 * Real account list arrives in M6.
 */
export function MainView() {
  return (
    <div className="p-6" data-testid="main-view">
      <div className="mx-auto max-w-3xl">
        <Card padding="lg" className="mb-4">
          <div className="flex items-start gap-4">
            <span className="text-accent">
              <SentinelLogo size={32} />
            </span>
            <div className="min-w-0 flex-1">
              <h2 className="text-lg font-semibold">Welcome to Sentinel</h2>
              <p className="mt-1 text-sm text-fg-muted">
                Your encrypted vault is unlocked. Account management, import flows, and
                QR scanning arrive in the next milestones.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                <Badge variant="success">Vault unlocked</Badge>
                <Badge variant="accent">Offline</Badge>
                <Badge variant="default">Encrypted</Badge>
              </div>
            </div>
          </div>
        </Card>

        <Card padding="lg" className="text-center py-12">
          <div className="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-full bg-bg-subtle text-fg-subtle">
            <svg
              width="24"
              height="24"
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
          </div>
          <h3 className="text-base font-medium">No accounts yet</h3>
          <p className="mt-1 text-sm text-fg-muted">
            Add your first account or import from Google Authenticator.
          </p>
          <div className="mt-4 flex justify-center gap-2">
            <Button variant="primary">Add account</Button>
            <Button variant="secondary">Import QR</Button>
          </div>
        </Card>
      </div>
    </div>
  );
}
