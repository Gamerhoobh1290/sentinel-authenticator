/**
 * Account list view — the main content area when the vault is unlocked.
 *
 * Features:
 *  - Virtualized list (handles hundreds of accounts without lag)
 *  - Instant search by issuer + account name
 *  - Favorites filter
 *  - Tag filter
 *  - Sort by custom order / issuer / account name
 *  - Bulk select + bulk delete
 *  - Empty state, loading state, error state
 *  - Code auto-refresh when TOTP period rolls over
 */

import { useEffect, useRef, useState, useCallback } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useAccounts } from "@/store/accountStore";
import { AccountCard } from "./AccountCard";
import { Button, Card, Badge, TextField } from "@/components/ui";
import { cn } from "@/lib/cn";

export function AccountListView() {
  const {
    accounts,
    codes,
    loading,
    error,
    searchQuery,
    showFavoritesOnly,
    activeTag,
    sortMode,
    selectedIds,
    loadAccounts,
    refreshCodes,
    setSearch,
    toggleFavoriteFilter,
    setActiveTag,
    setSortMode,
    clearSelection,
    deleteSelected,
  } = useAccounts();

  const parentRef = useRef<HTMLDivElement>(null);
  const [bulkMode, setBulkMode] = useState(false);
  const [lastRefresh, setLastRefresh] = useState(0);

  // Load accounts on mount
  useEffect(() => {
    void loadAccounts();
  }, [loadAccounts]);

  // Auto-refresh codes when the TOTP period rolls over.
  // We check every second but only refresh when the period has changed.
  useEffect(() => {
    const interval = window.setInterval(() => {
      const now = Math.floor(Date.now() / 1000);
      const currentPeriod = Math.floor(now / 30);
      if (currentPeriod !== Math.floor(lastRefresh / 30)) {
        void refreshCodes();
        setLastRefresh(now);
      }
    }, 1000);
    return () => window.clearInterval(interval);
  }, [refreshCodes, lastRefresh]);

  const visibleAccounts = useAccounts((s) => s.visibleAccounts());
  const allTags = useAccounts((s) => s.allTags());

  const virtualizer = useVirtualizer({
    count: visibleAccounts.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 76,
    overscan: 8,
  });

  const handleBulkDelete = useCallback(async () => {
    const count = selectedIds.size;
    if (count === 0) return;
    if (!window.confirm(`Delete ${count} account(s)? This cannot be undone.`)) return;
    await deleteSelected();
    setBulkMode(false);
  }, [selectedIds.size, deleteSelected]);

  if (loading && accounts.length === 0) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="flex flex-col items-center gap-3 text-fg-muted">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-accent border-r-transparent" />
          <p className="text-sm">Loading accounts…</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <Card padding="lg" className="max-w-md text-center">
          <div className="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-full bg-danger/10 text-danger">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
          </div>
          <h3 className="text-base font-medium text-fg">Failed to load accounts</h3>
          <p className="mt-1 text-sm text-fg-muted">{error}</p>
          <Button
            variant="primary"
            className="mt-4"
            onClick={() => void loadAccounts()}
          >
            Retry
          </Button>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      <div className="flex items-center gap-2 border-b border-border px-4 py-2">
        <TextField
          type="search"
          placeholder="Search accounts… (Ctrl+K)"
          value={searchQuery}
          onChange={(e) => setSearch(e.target.value)}
          aria-label="Search accounts"
          leading={<SearchIcon />}
          className="max-w-md"
        />

        <div className="flex-1" />

        {/* Sort */}
        <select
          value={sortMode}
          onChange={(e) =>
            setSortMode(e.target.value as "custom" | "issuer" | "account")
          }
          className="h-9 rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg focus:outline focus:outline-2 focus:outline-accent"
          aria-label="Sort accounts"
        >
          <option value="custom">Custom order</option>
          <option value="issuer">By issuer</option>
          <option value="account">By account name</option>
        </select>

        <Button
          variant={showFavoritesOnly ? "primary" : "secondary"}
          size="sm"
          onClick={toggleFavoriteFilter}
          aria-pressed={showFavoritesOnly}
        >
          <StarIcon filled={showFavoritesOnly} />
          <span className="ml-1.5">Favorites</span>
        </Button>

        <Button
          variant={bulkMode ? "primary" : "secondary"}
          size="sm"
          onClick={() => {
            setBulkMode(!bulkMode);
            if (bulkMode) clearSelection();
          }}
          aria-pressed={bulkMode}
        >
          {bulkMode ? "Done" : "Select"}
        </Button>
      </div>

      {/* Tag filter */}
      {allTags.length > 0 && (
        <div className="flex items-center gap-1.5 border-b border-border px-4 py-2 overflow-x-auto">
          <span className="text-xs font-medium text-fg-muted shrink-0">Tags:</span>
          <button
            type="button"
            onClick={() => setActiveTag(null)}
            className={cn(
              "rounded-full px-2.5 py-0.5 text-xs transition-colors",
              !activeTag
                ? "bg-accent text-accent-fg"
                : "bg-bg-subtle text-fg-muted hover:text-fg",
            )}
          >
            All
          </button>
          {allTags.map((tag) => (
            <button
              key={tag}
              type="button"
              onClick={() => setActiveTag(activeTag === tag ? null : tag)}
              className={cn(
                "rounded-full px-2.5 py-0.5 text-xs transition-colors",
                activeTag === tag
                  ? "bg-accent text-accent-fg"
                  : "bg-bg-subtle text-fg-muted hover:text-fg",
              )}
            >
              {tag}
            </button>
          ))}
        </div>
      )}

      {/* Bulk action bar */}
      {bulkMode && selectedIds.size > 0 && (
        <div className="flex items-center gap-3 border-b border-accent/30 bg-accent/5 px-4 py-2">
          <span className="text-sm text-fg-muted">{selectedIds.size} selected</span>
          <div className="flex-1" />
          <Button variant="danger" size="sm" onClick={handleBulkDelete}>
            Delete selected
          </Button>
        </div>
      )}

      {/* Account list (virtualized) */}
      <div ref={parentRef} className="flex-1 overflow-y-auto p-4">
        {visibleAccounts.length === 0 ? (
          <EmptyState searchQuery={searchQuery} />
        ) : (
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const account = visibleAccounts[virtualItem.index];
              if (!account) return null;
              return (
                <div
                  key={account.id}
                  data-index={virtualItem.index}
                  ref={virtualizer.measureElement}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                  className="mb-2"
                >
                  <AccountCard
                    account={account}
                    code={codes.get(account.id)}
                    selected={selectedIds.has(account.id)}
                    onSelect={
                      bulkMode
                        ? () => useAccounts.getState().toggleSelect(account.id)
                        : undefined
                    }
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Status bar */}
      <div className="flex items-center gap-2 border-t border-border px-4 py-1.5 text-xs text-fg-subtle">
        <span>{accounts.length} total</span>
        {visibleAccounts.length !== accounts.length && (
          <span>· {visibleAccounts.length} shown</span>
        )}
        {showFavoritesOnly && <Badge variant="accent">Favorites</Badge>}
        {activeTag && <Badge variant="accent">{activeTag}</Badge>}
      </div>
    </div>
  );
}

function EmptyState({ searchQuery }: { searchQuery: string }) {
  if (searchQuery) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
        <div className="grid h-12 w-12 place-items-center rounded-full bg-bg-subtle text-fg-subtle">
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
        </div>
        <h3 className="text-base font-medium text-fg">No matching accounts</h3>
        <p className="text-sm text-fg-muted">Try a different search term.</p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
      <div className="grid h-12 w-12 place-items-center rounded-full bg-bg-subtle text-fg-subtle">
        <svg
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <rect x="3" y="4" width="18" height="4" rx="1" />
          <rect x="3" y="11" width="18" height="4" rx="1" />
          <rect x="3" y="18" width="18" height="3" rx="1" />
        </svg>
      </div>
      <h3 className="text-base font-medium text-fg">No accounts yet</h3>
      <p className="max-w-xs text-sm text-fg-muted">
        Add your first account or import from Google Authenticator to get started.
      </p>
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

function StarIcon({ filled }: { filled?: boolean }) {
  return (
    <svg
      width="14"
      height="14"
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
