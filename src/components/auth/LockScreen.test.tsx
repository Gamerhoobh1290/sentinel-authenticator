import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LockScreen } from "@/components/auth/LockScreen";

describe("LockScreen", () => {
  it("disables the submit button until both passwords match in create mode", async () => {
    const user = userEvent.setup();
    render(<LockScreen mode="create" onSubmit={() => {}} />);

    const submit = screen.getByRole("button", { name: /create vault/i });
    expect(submit).toBeDisabled();

    // Type mismatched passwords
    await user.type(screen.getByLabelText(/master password/i), "short");
    expect(submit).toBeDisabled(); // too short

    await user.type(screen.getByLabelText(/confirm password/i), "different");
    expect(submit).toBeDisabled(); // mismatched
  });

  it("enables the submit button when both passwords match and are long enough", async () => {
    const user = userEvent.setup();
    render(<LockScreen mode="create" onSubmit={() => {}} />);

    await user.type(screen.getByLabelText(/master password/i), "correct horse");
    await user.type(screen.getByLabelText(/confirm password/i), "correct horse");

    const submit = screen.getByRole("button", { name: /create vault/i });
    expect(submit).toBeEnabled();
  });

  it("shows rate-limited message when rateLimited is true", () => {
    render(
      <LockScreen
        mode="unlock"
        onSubmit={() => {}}
        rateLimited
        rateLimitedSeconds={5}
      />,
    );
    expect(screen.getByText(/too many attempts/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /unlock/i })).toBeDisabled();
  });

  it("renders error message when provided", () => {
    render(
      <LockScreen
        mode="unlock"
        onSubmit={() => {}}
        errorMessage="Incorrect password."
      />,
    );
    expect(screen.getByText(/incorrect password/i)).toBeInTheDocument();
  });
});
