import "@testing-library/jest-dom/vitest";

// Vitest global setup for Sentinel Authenticator frontend tests.
// jsdom provides a minimal DOM; we don't need Tauri IPC here —
// component tests stub the IPC layer per-test.

// Silence React 19 act() warnings in test output — they're noise for our
// state-update patterns.
const originalError = console.error;
console.error = (...args: unknown[]) => {
  const first = args[0];
  if (typeof first === "string" && first.includes("not wrapped in act")) {
    return;
  }
  originalError(...args);
};
