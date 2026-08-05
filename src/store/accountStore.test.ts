import { describe, it, expect, beforeEach } from "vitest";
import { useAccounts } from "@/store/accountStore";
import type { AccountView } from "@/types";

// Mock the IPC module
vi.mock("@/lib/ipc", () => ({
  ipc: {
    listAccounts: vi.fn(),
    generateCode: vi.fn(),
    addAccountManual: vi.fn(),
    addAccountFromOtpauth: vi.fn(),
    importFromMigration: vi.fn(),
    deleteAccount: vi.fn(),
    deleteAccounts: vi.fn(),
    updateAccount: vi.fn(),
  },
}));

import { ipc } from "@/lib/ipc";

function makeAccount(overrides: Partial<AccountView> = {}): AccountView {
  return {
    id: "acc-1",
    issuer: "GitHub",
    label: "alice@example.com",
    otpType: "totp",
    algorithm: "sha1",
    digits: 6,
    period: 30,
    counter: 0,
    tags: [],
    favorite: false,
    sortPosition: 0,
    iconColor: undefined,
    iconText: undefined,
    createdAt: 0,
    updatedAt: 0,
    ...overrides,
  };
}

describe("accountStore", () => {
  beforeEach(() => {
    useAccounts.setState({
      accounts: [],
      codes: new Map(),
      loading: false,
      error: null,
      searchQuery: "",
      showFavoritesOnly: false,
      activeTag: null,
      sortMode: "custom",
      selectedIds: new Set(),
    });
    vi.clearAllMocks();
  });

  describe("filteredAccounts", () => {
    it("filters by search query on issuer and label", () => {
      const accounts = [
        makeAccount({ id: "1", issuer: "GitHub", label: "alice@example.com" }),
        makeAccount({ id: "2", issuer: "GitLab", label: "bob@example.com" }),
        makeAccount({ id: "3", issuer: "AWS", label: "carol@example.com" }),
      ];
      useAccounts.setState({ accounts });

      useAccounts.getState().setSearch("git");
      const filtered = useAccounts.getState().filteredAccounts();
      expect(filtered).toHaveLength(2);
      expect(filtered.map((a) => a.id).sort()).toEqual(["1", "2"]);

      useAccounts.getState().setSearch("alice");
      const aliceFiltered = useAccounts.getState().filteredAccounts();
      expect(aliceFiltered).toHaveLength(1);
      expect(aliceFiltered[0]!.id).toBe("1");
    });

    it("filters by favorites", () => {
      const accounts = [
        makeAccount({ id: "1", favorite: true }),
        makeAccount({ id: "2", favorite: false }),
        makeAccount({ id: "3", favorite: true }),
      ];
      useAccounts.setState({ accounts });
      useAccounts.getState().toggleFavoriteFilter();
      const filtered = useAccounts.getState().filteredAccounts();
      expect(filtered).toHaveLength(2);
      expect(filtered.every((a) => a.favorite)).toBe(true);
    });

    it("filters by tag", () => {
      const accounts = [
        makeAccount({ id: "1", tags: ["work"] }),
        makeAccount({ id: "2", tags: ["personal"] }),
        makeAccount({ id: "3", tags: ["work", "personal"] }),
      ];
      useAccounts.setState({ accounts });
      useAccounts.getState().setActiveTag("work");
      expect(useAccounts.getState().filteredAccounts()).toHaveLength(2);
    });

    it("combines search + favorites + tag filters", () => {
      const accounts = [
        makeAccount({
          id: "1",
          issuer: "GitHub",
          label: "alice",
          favorite: true,
          tags: ["work"],
        }),
        makeAccount({
          id: "2",
          issuer: "GitHub",
          label: "bob",
          favorite: false,
          tags: ["work"],
        }),
        makeAccount({
          id: "3",
          issuer: "GitLab",
          label: "alice",
          favorite: true,
          tags: ["personal"],
        }),
      ];
      useAccounts.setState({ accounts });
      useAccounts.getState().setSearch("git");
      useAccounts.getState().toggleFavoriteFilter();
      useAccounts.getState().setActiveTag("work");

      const combined = useAccounts.getState().filteredAccounts();
      expect(combined).toHaveLength(1);
      expect(combined[0]!.id).toBe("1");
    });
  });

  describe("visibleAccounts (sorting)", () => {
    it("sorts by issuer", () => {
      const accounts = [
        makeAccount({ id: "3", issuer: "Zebra", sortPosition: 0 }),
        makeAccount({ id: "1", issuer: "Apple", sortPosition: 2 }),
        makeAccount({ id: "2", issuer: "Mango", sortPosition: 1 }),
      ];
      useAccounts.setState({ accounts });
      useAccounts.getState().setSortMode("issuer");
      const sorted = useAccounts.getState().visibleAccounts();
      expect(sorted.map((a) => a.id)).toEqual(["1", "2", "3"]);
    });

    it("sorts by account name", () => {
      const accounts = [
        makeAccount({ id: "3", label: "zoe@example.com" }),
        makeAccount({ id: "1", label: "alice@example.com" }),
        makeAccount({ id: "2", label: "bob@example.com" }),
      ];
      useAccounts.setState({ accounts });
      useAccounts.getState().setSortMode("account");
      const sorted = useAccounts.getState().visibleAccounts();
      expect(sorted.map((a) => a.id)).toEqual(["1", "2", "3"]);
    });

    it("sorts by custom order (sortPosition)", () => {
      const accounts = [
        makeAccount({ id: "3", sortPosition: 3 }),
        makeAccount({ id: "1", sortPosition: 1 }),
        makeAccount({ id: "2", sortPosition: 2 }),
      ];
      useAccounts.setState({ accounts });
      useAccounts.getState().setSortMode("custom");
      const sorted = useAccounts.getState().visibleAccounts();
      expect(sorted.map((a) => a.id)).toEqual(["1", "2", "3"]);
    });
  });

  describe("selection", () => {
    it("toggles selection", () => {
      useAccounts.setState({ accounts: [makeAccount({ id: "1" })] });
      useAccounts.getState().toggleSelect("1");
      expect(useAccounts.getState().selectedIds.has("1")).toBe(true);
      useAccounts.getState().toggleSelect("1");
      expect(useAccounts.getState().selectedIds.has("1")).toBe(false);
    });

    it("selects all", () => {
      useAccounts.setState({
        accounts: [makeAccount({ id: "1" }), makeAccount({ id: "2" })],
      });
      useAccounts.getState().selectAll();
      expect(useAccounts.getState().selectedIds.size).toBe(2);
    });

    it("clears selection", () => {
      useAccounts.setState({
        accounts: [makeAccount({ id: "1" })],
        selectedIds: new Set(["1"]),
      });
      useAccounts.getState().clearSelection();
      expect(useAccounts.getState().selectedIds.size).toBe(0);
    });

    it("deletes selected accounts via IPC", async () => {
      vi.mocked(ipc.deleteAccounts).mockResolvedValue(2);
      useAccounts.setState({
        accounts: [makeAccount({ id: "1" }), makeAccount({ id: "2" })],
        selectedIds: new Set(["1", "2"]),
      });
      const deleted = await useAccounts.getState().deleteSelected();
      expect(deleted).toBe(2);
      expect(ipc.deleteAccounts).toHaveBeenCalledWith(["1", "2"]);
    });
  });

  describe("allTags", () => {
    it("collects unique tags from all accounts", () => {
      useAccounts.setState({
        accounts: [
          makeAccount({ id: "1", tags: ["work", "important"] }),
          makeAccount({ id: "2", tags: ["personal"] }),
          makeAccount({ id: "3", tags: ["work"] }),
        ],
      });
      const tags = useAccounts.getState().allTags();
      expect(tags).toEqual(["important", "personal", "work"]);
    });
  });
});
