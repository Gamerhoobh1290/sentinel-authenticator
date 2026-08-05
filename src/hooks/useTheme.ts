import { useEffect } from "react";
import type { Theme } from "@/types";

/**
 * Apply theme by toggling `data-theme` on <html>.
 * System theme reacts to `prefers-color-scheme` changes live.
 */
export function useTheme(theme: Theme) {
  useEffect(() => {
    const root = document.documentElement;
    const apply = (t: Theme) => {
      const resolved =
        t === "system"
          ? window.matchMedia("(prefers-color-scheme: dark)").matches
            ? "dark"
            : "light"
          : t;
      root.setAttribute("data-theme", resolved);
    };

    apply(theme);

    if (theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => apply("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [theme]);
}

/**
 * Apply reduced-motion preference.
 * Honors the user setting; falls back to OS preference.
 */
export function useReducedMotion(reduce: boolean) {
  useEffect(() => {
    const root = document.documentElement;
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const shouldReduce = reduce || mq.matches;
    root.setAttribute("data-reduced-motion", shouldReduce ? "true" : "false");
  }, [reduce]);
}
