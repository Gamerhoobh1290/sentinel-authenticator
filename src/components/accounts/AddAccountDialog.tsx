/**
 * Add account dialog — manual entry form.
 *
 * Lets the user create a new OTP account by entering the secret and
 * metadata manually. Includes live preview of the generated code.
 */

import { useState, type FormEvent } from "react";
import { Dialog, TextField, PasswordField, Button } from "@/components/ui";
import type { OtpAlgorithm, OtpType } from "@/types";
import { ipc, type ManualAccountInput } from "@/lib/ipc";

export interface AddAccountDialogProps {
  open: boolean;
  onClose: () => void;
  onAdded: () => void;
}

export function AddAccountDialog({ open, onClose, onAdded }: AddAccountDialogProps) {
  const [issuer, setIssuer] = useState("");
  const [label, setLabel] = useState("");
  const [secret, setSecret] = useState("");
  const [otpType, setOtpType] = useState<OtpType>("totp");
  const [algorithm, setAlgorithm] = useState<OtpAlgorithm>("sha1");
  const [digits, setDigits] = useState<6 | 8>(6);
  const [period, setPeriod] = useState(30);
  const [counter, setCounter] = useState(0);
  const [iconColor, setIconColor] = useState("#60a5fa");
  const [iconText, setIconText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const reset = () => {
    setIssuer("");
    setLabel("");
    setSecret("");
    setOtpType("totp");
    setAlgorithm("sha1");
    setDigits(6);
    setPeriod(30);
    setCounter(0);
    setIconColor("#60a5fa");
    setIconText("");
    setError(null);
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);

    const input: ManualAccountInput = {
      issuer,
      label,
      secret,
      otpType,
      algorithm,
      digits,
      period,
      counter,
      iconColor: iconColor || undefined,
      iconText: iconText || undefined,
    };

    try {
      await ipc.addAccountManual(input);
      reset();
      onAdded();
      onClose();
    } catch (err) {
      setError(String(err).replace(/^Error:\s*/, ""));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Add account"
      description="Enter the account details manually. The secret is the Base32 key from your service provider."
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={submitting}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={handleSubmit}
            loading={submitting}
            type="submit"
            form="add-account-form"
          >
            Add account
          </Button>
        </>
      }
    >
      <form id="add-account-form" onSubmit={handleSubmit} className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <TextField
            label="Issuer"
            value={issuer}
            onChange={(e) => setIssuer(e.target.value)}
            placeholder="GitHub"
            hint="e.g. GitHub, AWS, Google"
          />
          <TextField
            label="Account name"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="alice@example.com"
            required
          />
        </div>

        <PasswordField
          label="Secret key (Base32)"
          value={secret}
          onChange={(e) => setSecret(e.target.value)}
          placeholder="JBSWY3DPEHPK3PXP"
          hint="A-Z and 2-7 only. Spaces are ignored."
          required
        />

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-fg-muted">
              OTP type
            </label>
            <select
              value={otpType}
              onChange={(e) => setOtpType(e.target.value as OtpType)}
              className="h-9 w-full rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg focus:outline focus:outline-2 focus:outline-accent"
            >
              <option value="totp">TOTP (time-based)</option>
              <option value="hotp">HOTP (counter-based)</option>
            </select>
          </div>
          <div>
            <label className="mb-1.5 block text-xs font-medium text-fg-muted">
              Algorithm
            </label>
            <select
              value={algorithm}
              onChange={(e) => setAlgorithm(e.target.value as OtpAlgorithm)}
              className="h-9 w-full rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg focus:outline focus:outline-2 focus:outline-accent"
            >
              <option value="sha1">SHA-1 (default)</option>
              <option value="sha256">SHA-256</option>
              <option value="sha512">SHA-512</option>
            </select>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-fg-muted">
              Digits
            </label>
            <select
              value={digits}
              onChange={(e) => setDigits(Number(e.target.value) as 6 | 8)}
              className="h-9 w-full rounded-lg border border-border bg-bg-subtle px-3 text-sm text-fg focus:outline focus:outline-2 focus:outline-accent"
            >
              <option value={6}>6 digits</option>
              <option value={8}>8 digits</option>
            </select>
          </div>
          {otpType === "totp" ? (
            <TextField
              label="Period (seconds)"
              type="number"
              value={period}
              onChange={(e) => setPeriod(Number(e.target.value))}
              min={1}
              max={600}
            />
          ) : (
            <TextField
              label="Counter"
              type="number"
              value={counter}
              onChange={(e) => setCounter(Number(e.target.value))}
              min={0}
            />
          )}
          <div>
            <label className="mb-1.5 block text-xs font-medium text-fg-muted">
              Icon color
            </label>
            <input
              type="color"
              value={iconColor}
              onChange={(e) => setIconColor(e.target.value)}
              className="h-9 w-full cursor-pointer rounded-lg border border-border bg-bg-subtle"
              aria-label="Icon color"
            />
          </div>
        </div>

        <TextField
          label="Icon text (optional)"
          value={iconText}
          onChange={(e) => setIconText(e.target.value.slice(0, 3))}
          placeholder="GH"
          maxLength={3}
          hint="1-3 characters shown on the account icon"
        />

        {error && (
          <p
            role="alert"
            className="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger"
          >
            {error}
          </p>
        )}
      </form>
    </Dialog>
  );
}
