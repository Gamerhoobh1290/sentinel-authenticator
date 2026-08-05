import { describe, it, expect, beforeEach, vi } from "vitest";

// Mock the clipboard plugin with typed functions
const mockWriteText = vi.fn<(text: string) => Promise<void>>();
const mockReadText = vi.fn<() => Promise<string>>();
const mockClear = vi.fn<() => Promise<void>>();

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: (text: string) => mockWriteText(text),
  readText: () => mockReadText(),
  clear: () => mockClear(),
}));

// Mock settings store
vi.mock("@/store/settingsStore", () => ({
  useSettings: {
    getState: () => ({
      clipboardClearDelay: 30000 as const,
    }),
  },
}));

import { copyCode, cancelClipboardClear } from "@/lib/clipboard";

describe("clipboard manager", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    cancelClipboardClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("copies a code to the clipboard", async () => {
    mockWriteText.mockResolvedValue(undefined);
    const result = await copyCode("123456");
    expect(result).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith("123456");
  });

  it("schedules auto-clear after the configured delay", async () => {
    mockWriteText.mockResolvedValue(undefined);
    mockReadText.mockResolvedValue("123456");
    mockClear.mockResolvedValue(undefined);

    await copyCode("123456");

    // Advance past the 30s delay
    await vi.advanceTimersByTimeAsync(31000);

    expect(mockReadText).toHaveBeenCalled();
    expect(mockClear).toHaveBeenCalled();
  });

  it("does not clear if clipboard content changed", async () => {
    mockWriteText.mockResolvedValue(undefined);
    mockReadText.mockResolvedValue("different-value");
    mockClear.mockResolvedValue(undefined);

    await copyCode("123456");
    await vi.advanceTimersByTimeAsync(31000);

    expect(mockReadText).toHaveBeenCalled();
    expect(mockClear).not.toHaveBeenCalled();
  });

  it("cancelClipboardClear prevents the scheduled clear", async () => {
    mockWriteText.mockResolvedValue(undefined);
    mockReadText.mockResolvedValue("123456");
    mockClear.mockResolvedValue(undefined);

    await copyCode("123456");
    cancelClipboardClear();
    await vi.advanceTimersByTimeAsync(31000);

    expect(mockClear).not.toHaveBeenCalled();
  });
});
