/**
 * Auto-lock hook — locks the vault after a period of inactivity.
 *
 * Tracks user activity (mouse move, key press, click, scroll, touch)
 * and locks the vault when the configured inactivity threshold is reached.
 * Also locks on window minimize and (on Windows) on session lock.
 */

import { useEffect, useRef } from "react";
import { useSettings } from "@/store/settingsStore";
import { useVault } from "@/store/vaultStore";
import { ipc } from "@/lib/ipc";
import { cancelClipboardClear } from "@/lib/clipboard";

const ACTIVITY_EVENTS: (keyof WindowEventMap)[] = [
  "mousemove",
  "mousedown",
  "keydown",
  "click",
  "scroll",
  "touchstart",
];

export function useAutoLock(): void {
  const autoLockDelay = useSettings((s) => s.autoLockDelay);
  const lockWhenMinimized = useSettings((s) => s.lockWhenMinimized);
  const vaultState = useVault((s) => s.state);
  const setVaultState = useVault((s) => s.setState);
  const lastActivityRef = useRef(Date.now());

  // Track activity
  useEffect(() => {
    if (vaultState !== "unlocked") return;

    const updateActivity = () => {
      lastActivityRef.current = Date.now();
    };

    for (const event of ACTIVITY_EVENTS) {
      window.addEventListener(event, updateActivity, { passive: true });
    }

    return () => {
      for (const event of ACTIVITY_EVENTS) {
        window.removeEventListener(event, updateActivity);
      }
    };
  }, [vaultState]);

  // Inactivity timer
  useEffect(() => {
    if (vaultState !== "unlocked" || autoLockDelay === "never") return;

    const interval = window.setInterval(() => {
      const elapsed = Date.now() - lastActivityRef.current;
      if (elapsed >= autoLockDelay) {
        lockVault();
      }
    }, 10_000); // Check every 10 seconds

    return () => window.clearInterval(interval);
  }, [vaultState, autoLockDelay]);

  // Lock on minimize
  useEffect(() => {
    if (!lockWhenMinimized || vaultState !== "unlocked") return;

    const handleVisibilityChange = () => {
      if (document.hidden) {
        lockVault();
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () =>
      document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, [vaultState, lockWhenMinimized]);

  function lockVault(): void {
    cancelClipboardClear();
    void ipc.vaultLock().catch(() => {});
    setVaultState("locked");
  }
}
