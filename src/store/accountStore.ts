/**
 * Account store — manages the frontend account list.
 *
 * This store holds only sanitized AccountView objects (no secrets).
 * Codes are generated on demand via IPC and cached for the current
 * TOTP period only. When the period rolls over, codes are regenerated.
 */

import { create } from "zustand";
import type { AccountView, CodeResult } from "@/types";
import { ipc } from "@/lib/ipc";

type SortMode = "custom" | "issuer" | "account";

interface AccountStore {
  accounts: AccountView[];
  codes: Map<string, CodeResult>;
  loading: boolean;
  error: string | null;
  searchQuery: string;
  showFavoritesOnly: boolean;
  activeTag: string | null;
  sortMode: SortMode;
  selectedIds: Set<string>;

  // Actions
  loadAccounts: () => Promise<void>;
  refreshCodes: () => Promise<void>;
  setSearch: (query: string) => void;
  toggleFavoriteFilter: () => void;
  setActiveTag: (tag: string | null) => void;
  setSortMode: (mode: SortMode) => void;
  toggleSelect: (id: string) => void;
  selectAll: () => void;
  clearSelection: () => void;
  deleteSelected: () => Promise<number>;
  deleteAccount: (id: string) => Promise<void>;
  toggleFavorite: (id: string) => Promise<void>;
  incrementHotpCounter: (id: string) => Promise<void>;
  addManual: (input: Parameters<typeof ipc.addAccountManual>[0]) => Promise<void>;
  addFromOtpauth: (uri: string) => Promise<void>;
  importFromMigration: (uri: string) => Promise<number>;

  // Computed
  filteredAccounts: () => AccountView[];
  visibleAccounts: () => AccountView[];
  allTags: () => string[];
}

export const useAccounts = create<AccountStore>((set, get) => ({
  accounts: [],
  codes: new Map(),
  loading: false,
  error: null,
  searchQuery: "",
  showFavoritesOnly: false,
  activeTag: null,
  sortMode: "custom",
  selectedIds: new Set(),

  loadAccounts: async () => {
    set({ loading: true, error: null });
    try {
      const accounts = await ipc.listAccounts();
      set({ accounts, loading: false });
      await get().refreshCodes();
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  refreshCodes: async () => {
    const { accounts, codes } = get();
    const newCodes = new Map(codes);
    for (const account of accounts) {
      try {
        const result = await ipc.generateCode(account.id);
        newCodes.set(account.id, result);
      } catch {
        // Skip failed code generation — don't crash the whole refresh
      }
    }
    set({ codes: newCodes });
  },

  setSearch: (searchQuery) => set({ searchQuery }),
  toggleFavoriteFilter: () => set((s) => ({ showFavoritesOnly: !s.showFavoritesOnly })),
  setActiveTag: (activeTag) => set({ activeTag }),
  setSortMode: (sortMode) => set({ sortMode }),

  toggleSelect: (id) =>
    set((s) => {
      const selectedIds = new Set(s.selectedIds);
      if (selectedIds.has(id)) {
        selectedIds.delete(id);
      } else {
        selectedIds.add(id);
      }
      return { selectedIds };
    }),

  selectAll: () => set((s) => ({ selectedIds: new Set(s.accounts.map((a) => a.id)) })),

  clearSelection: () => set({ selectedIds: new Set() }),

  deleteSelected: async () => {
    const { selectedIds } = get();
    const ids = [...selectedIds];
    if (ids.length === 0) return 0;
    const deleted = await ipc.deleteAccounts(ids);
    set({ selectedIds: new Set() });
    await get().loadAccounts();
    return deleted;
  },

  deleteAccount: async (id) => {
    await ipc.deleteAccount(id);
    await get().loadAccounts();
  },

  toggleFavorite: async (id) => {
    const account = get().accounts.find((a) => a.id === id);
    if (!account) return;
    await ipc.updateAccount({ id, favorite: !account.favorite });
    await get().loadAccounts();
  },

  incrementHotpCounter: async (id) => {
    await ipc.incrementHotpCounter(id);
    await get().loadAccounts();
  },

  addManual: async (input) => {
    await ipc.addAccountManual(input);
    await get().loadAccounts();
  },

  addFromOtpauth: async (uri) => {
    await ipc.addAccountFromOtpauth(uri);
    await get().loadAccounts();
  },

  importFromMigration: async (uri) => {
    const added = await ipc.importFromMigration(uri);
    await get().loadAccounts();
    return added.length;
  },

  filteredAccounts: () => {
    const { accounts, searchQuery, showFavoritesOnly, activeTag } = get();
    const q = searchQuery.trim().toLowerCase();
    return accounts.filter((a) => {
      if (showFavoritesOnly && !a.favorite) return false;
      if (activeTag && !a.tags.includes(activeTag)) return false;
      if (q) {
        const haystack = `${a.issuer} ${a.label}`.toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      return true;
    });
  },

  visibleAccounts: () => {
    const { sortMode } = get();
    const filtered = get().filteredAccounts();
    const sorted = [...filtered];
    switch (sortMode) {
      case "issuer":
        sorted.sort(
          (a, b) => a.issuer.localeCompare(b.issuer) || a.label.localeCompare(b.label),
        );
        break;
      case "account":
        sorted.sort((a, b) => a.label.localeCompare(b.label));
        break;
      case "custom":
      default:
        sorted.sort((a, b) => a.sortPosition - b.sortPosition);
        break;
    }
    return sorted;
  },

  allTags: () => {
    const { accounts } = get();
    const tags = new Set<string>();
    for (const a of accounts) {
      for (const t of a.tags) tags.add(t);
    }
    return [...tags].sort();
  },
}));
