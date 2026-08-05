import { create } from "zustand";
import type { VaultState } from "@/types";

/**
 * Vault lifecycle store.
 *
 * This store only tracks the *state machine* of the vault — it never holds
 * decrypted account data. The decrypted `Vec<AccountRecord>` lives in
 * Rust memory and is zeroized on lock. The frontend receives only the
 * sanitized `AccountView` list via IPC, and only when the vault is unlocked.
 */
interface VaultStore {
  state: VaultState;
  lastError: string | null;
  failedAttempts: number;
  lockedUntil: number | null; // epoch ms; rate-limit backoff
  setState: (state: VaultState) => void;
  setError: (error: string | null) => void;
  recordFailedAttempt: () => void;
  resetAttempts: () => void;
  /** Returns true if currently rate-limited; computes expiry on demand. */
  isRateLimited: () => boolean;
}

export const useVault = create<VaultStore>((set, get) => ({
  state: "uninitialised",
  lastError: null,
  failedAttempts: 0,
  lockedUntil: null,
  setState: (state) =>
    set({ state, lastError: state === "unlocked" ? null : get().lastError }),
  setError: (lastError) => set({ lastError }),
  recordFailedAttempt: () => {
    const count = get().failedAttempts + 1;
    // Exponential backoff: 0, 0, 1s, 2s, 4s, 8s, 16s, 30s, 30s, ...
    const delay = count <= 2 ? 0 : Math.min(30_000, 2 ** (count - 3) * 1000);
    set({
      failedAttempts: count,
      lockedUntil: delay > 0 ? Date.now() + delay : null,
    });
  },
  resetAttempts: () => set({ failedAttempts: 0, lockedUntil: null }),
  isRateLimited: () => {
    const until = get().lockedUntil;
    if (!until) return false;
    if (Date.now() >= until) {
      set({ lockedUntil: null });
      return false;
    }
    return true;
  },
}));
