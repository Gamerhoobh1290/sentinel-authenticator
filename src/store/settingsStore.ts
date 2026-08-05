import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { DEFAULT_SETTINGS, type AppSettings, type Theme, type Density } from "@/types";

/**
 * Settings store.
 *
 * ⚠️ SECURITY: This store uses `persist` to write to localStorage. This is
 * intentional and SAFE because:
 *   1. The store ONLY contains non-sensitive UI preferences (theme, density,
 *      auto-lock delay, etc.).
 *   2. The ESLint config forbids localStorage access outside this file
 *      via the `no-restricted-syntax` rule on `MemberExpression`.
 *   3. No secrets, account names, issuers, or OTP codes ever pass through
 *      this store.
 *
 * The vault itself (encrypted account data) is managed entirely in Rust
 * and written to %APPDATA%\Sentinel\vault.bin — never to browser storage.
 */
interface SettingsStore extends AppSettings {
  setTheme: (theme: Theme) => void;
  setDensity: (density: Density) => void;
  setReducedMotion: (reduce: boolean) => void;
  update: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void;
  updateMany: (partial: Partial<AppSettings>) => void;
  reset: () => void;
}

export const useSettings = create<SettingsStore>()(
  persist(
    (set) => ({
      ...DEFAULT_SETTINGS,
      setTheme: (theme) => set({ theme }),
      setDensity: (density) => set({ density }),
      setReducedMotion: (reducedMotion) => set({ reducedMotion }),
      update: (key, value) => set({ [key]: value } as Pick<SettingsStore, typeof key>),
      updateMany: (partial) => set(partial),
      reset: () => set({ ...DEFAULT_SETTINGS }),
    }),
    {
      name: "sentinel-settings",
      storage: createJSONStorage(() => localStorage),
      // Only persist UI prefs — never anything sensitive
      partialize: (s) => ({
        theme: s.theme,
        density: s.density,
        reducedMotion: s.reducedMotion,
        autoLockDelay: s.autoLockDelay,
        lockWhenMinimized: s.lockWhenMinimized,
        lockWhenWindowsLocks: s.lockWhenWindowsLocks,
        clipboardClearDelay: s.clipboardClearDelay,
        codeFormatting: s.codeFormatting,
        startWithWindows: s.startWithWindows,
        startMinimized: s.startMinimized,
        minimizeToTray: s.minimizeToTray,
        closeToTray: s.closeToTray,
        alwaysOnTop: s.alwaysOnTop,
        confirmDestructiveActions: s.confirmDestructiveActions,
      }),
    },
  ),
);
