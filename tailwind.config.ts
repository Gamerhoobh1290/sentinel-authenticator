import type { Config } from "tailwindcss";

// Sentinel Authenticator — Tailwind config.
// Tokens map to CSS variables so theme switching is a single `data-theme` attribute swap.
// No plugin chain — keeps the bundle minimal.
const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: ["class", '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        // Semantic tokens — resolve at runtime via CSS variables defined in globals.css
        bg: "rgb(var(--bg) / <alpha-value>)",
        "bg-elevated": "rgb(var(--bg-elevated) / <alpha-value>)",
        "bg-subtle": "rgb(var(--bg-subtle) / <alpha-value>)",
        border: "rgb(var(--border) / <alpha-value>)",
        "border-strong": "rgb(var(--border-strong) / <alpha-value>)",
        fg: "rgb(var(--fg) / <alpha-value>)",
        "fg-muted": "rgb(var(--fg-muted) / <alpha-value>)",
        "fg-subtle": "rgb(var(--fg-subtle) / <alpha-value>)",
        accent: "rgb(var(--accent) / <alpha-value>)",
        "accent-fg": "rgb(var(--accent-fg) / <alpha-value>)",
        "accent-hover": "rgb(var(--accent-hover) / <alpha-value>)",
        danger: "rgb(var(--danger) / <alpha-value>)",
        "danger-fg": "rgb(var(--danger-fg) / <alpha-value>)",
        success: "rgb(var(--success) / <alpha-value>)",
        warning: "rgb(var(--warning) / <alpha-value>)",
      },
      fontFamily: {
        sans: [
          "Inter",
          "Segoe UI Variable",
          "Segoe UI",
          "-apple-system",
          "BlinkMacSystemFont",
          "system-ui",
          "sans-serif",
        ],
        mono: [
          "Cascadia Code",
          "JetBrains Mono",
          "SFMono-Regular",
          "Consolas",
          "monospace",
        ],
      },
      fontSize: {
        "2xs": ["0.6875rem", { lineHeight: "1rem" }],
      },
      borderRadius: {
        // Fluent-style soft rounding
        xl: "0.625rem",
        "2xl": "0.875rem",
        "3xl": "1.25rem",
      },
      boxShadow: {
        card: "0 1px 2px 0 rgb(0 0 0 / 0.04), 0 1px 3px 0 rgb(0 0 0 / 0.06)",
        "card-hover":
          "0 2px 4px 0 rgb(0 0 0 / 0.06), 0 4px 12px 0 rgb(0 0 0 / 0.08)",
        popover:
          "0 8px 24px -4px rgb(0 0 0 / 0.18), 0 4px 8px -2px rgb(0 0 0 / 0.10)",
      },
      transitionTimingFunction: {
        fluent: "cubic-bezier(0.1, 0.9, 0.2, 1)",
      },
      transitionDuration: {
        // Most interactions land in 120–220ms per spec
        "120": "120ms",
        "160": "160ms",
        "220": "220ms",
      },
      keyframes: {
        "fade-in": {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        "scale-in": {
          from: { opacity: "0", transform: "scale(0.96)" },
          to: { opacity: "1", transform: "scale(1)" },
        },
        "slide-up": {
          from: { opacity: "0", transform: "translateY(8px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
      },
      animation: {
        "fade-in": "fade-in 160ms cubic-bezier(0.1, 0.9, 0.2, 1)",
        "scale-in": "scale-in 160ms cubic-bezier(0.1, 0.9, 0.2, 1)",
        "slide-up": "slide-up 220ms cubic-bezier(0.1, 0.9, 0.2, 1)",
      },
    },
  },
  plugins: [],
};

export default config;
