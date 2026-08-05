import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { AppShell, type NavSection } from "@/components/layout/AppShell";
import userEvent from "@testing-library/user-event";

describe("AppShell", () => {
  it("renders the Sentinel brand and primary navigation", () => {
    let active: NavSection = "accounts";
    render(
      <AppShell
        active={active}
        onNavigate={(s) => (active = s)}
        onLock={() => {}}
        onAddAccount={() => {}}
      >
        <div>content</div>
      </AppShell>,
    );
    expect(screen.getByText("Sentinel")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /accounts/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /favorites/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /import/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /backup/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /settings/i })).toBeInTheDocument();
  });

  it("calls onNavigate when a nav item is clicked", async () => {
    const user = userEvent.setup();
    let active: NavSection = "accounts";
    const onNavigate = (s: NavSection) => (active = s);
    render(
      <AppShell
        active={active}
        onNavigate={onNavigate}
        onLock={() => {}}
        onAddAccount={() => {}}
      >
        <div>content</div>
      </AppShell>,
    );
    await user.click(screen.getByRole("button", { name: /settings/i }));
    expect(active).toBe("settings");
  });

  it("calls onLock when the lock button is clicked", async () => {
    const user = userEvent.setup();
    let locked = false;
    render(
      <AppShell
        active="accounts"
        onNavigate={() => {}}
        onLock={() => (locked = true)}
        onAddAccount={() => {}}
      >
        <div>content</div>
      </AppShell>,
    );
    await user.click(screen.getByRole("button", { name: /lock vault/i }));
    expect(locked).toBe(true);
  });
});
