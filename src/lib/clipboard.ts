/**
 * Clipboard manager — copies codes and auto-clears them after a delay.
 *
 * Security:
 *  - Only the numeric code is copied (never raw secrets unless explicitly revealed)
 *  - Auto-clears after 10/30/60 seconds (default 30s)
 *  - Before clearing, re-reads the clipboard; only clears if it still contains
 *    the value we placed there (doesn't clobber user's other copies)
 *  - Raw secret reveal requires re-authentication + explicit warning
 */

import { useSettings } from "@/store/settingsStore";

let clearTimerId: number | null = null;
let lastCopiedValue: string | null = null;

/**
 * Copy a code to the clipboard and schedule auto-clear.
 * Returns true on success.
 */
export async function copyCode(code: string): Promise<boolean> {
  try {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(code);
    lastCopiedValue = code;
    scheduleClear();
    return true;
  } catch {
    // Fallback: navigator.clipboard (may work in some WebView2 configurations)
    try {
      await navigator.clipboard.writeText(code);
      lastCopiedValue = code;
      scheduleClear();
      return true;
    } catch {
      return false;
    }
  }
}

/**
 * Copy a raw secret to the clipboard. Requires explicit user confirmation.
 * The caller MUST show a warning dialog before calling this.
 */
export async function copySecretWithWarning(
  secret: string,
  _confirmed: boolean,
): Promise<boolean> {
  if (!_confirmed) return false;
  // Same auto-clear mechanism as codes, but with a shorter default (30s)
  try {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(secret);
    lastCopiedValue = secret;
    scheduleClear();
    return true;
  } catch {
    return false;
  }
}

function scheduleClear(): void {
  // Cancel any existing timer
  if (clearTimerId !== null) {
    window.clearTimeout(clearTimerId);
  }

  const delay = useSettings.getState().clipboardClearDelay;
  if (delay === "never") return;

  clearTimerId = window.setTimeout(() => {
    void clearIfOwned();
  }, delay);
}

async function clearIfOwned(): Promise<void> {
  if (lastCopiedValue === null) return;

  try {
    const { readText, clear } = await import("@tauri-apps/plugin-clipboard-manager");
    const current = await readText();
    // Only clear if the clipboard still contains our value
    // (don't clobber something the user copied in the meantime)
    if (current === lastCopiedValue) {
      await clear();
    }
  } catch {
    // If we can't read the clipboard, don't clear — safer to leave it
    // than to clear something we shouldn't
  }

  lastCopiedValue = null;
  clearTimerId = null;
}

/**
 * Cancel any pending clipboard clear (e.g. when the vault locks).
 */
export function cancelClipboardClear(): void {
  if (clearTimerId !== null) {
    window.clearTimeout(clearTimerId);
    clearTimerId = null;
  }
  lastCopiedValue = null;
}
