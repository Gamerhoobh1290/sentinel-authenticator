/**
 * QR import dialog — scan or upload a QR code to import an account.
 *
 * Supports otpauth:// and otpauth-migration:// URIs.
 */

import { useState, useRef, useCallback, useEffect } from "react";
import { Dialog, Button, Badge } from "@/components/ui";
import { ipc } from "@/lib/ipc";
import type { AccountView } from "@/types";

export interface QrImportDialogProps {
  open: boolean;
  onClose: () => void;
  onImported: (count: number) => void;
}

type ScanMode = "camera" | "file";
type ScanState = "idle" | "scanning" | "success" | "error";

export function QrImportDialog({ open, onClose, onImported }: QrImportDialogProps) {
  const [mode, setMode] = useState<ScanMode>("camera");
  const [scanState, setScanState] = useState<ScanState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [importedAccounts, setImportedAccounts] = useState<AccountView[]>([]);
  const videoRef = useRef<HTMLVideoElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const controlsRef = useRef<{ stop: () => void } | null>(null);

  const stopCamera = useCallback(() => {
    if (controlsRef.current) {
      controlsRef.current.stop();
      controlsRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (!open) {
      stopCamera();
      setScanState("idle");
      setError(null);
      setImportedAccounts([]);
    }
  }, [open, stopCamera]);

  const handleQrText = useCallback(async (text: string) => {
    setError(null);
    try {
      if (text.startsWith("otpauth-migration://")) {
        const added = await ipc.importFromMigration(text);
        setImportedAccounts((prev) => [...prev, ...added]);
        setScanState("success");
      } else if (text.startsWith("otpauth://")) {
        const account = await ipc.addAccountFromOtpauth(text);
        setImportedAccounts((prev) => [...prev, account]);
        setScanState("success");
      } else {
        setError("This QR code is not a recognized OTP auth code.");
        setScanState("error");
      }
    } catch (err) {
      setError(String(err).replace(/^Error:\s*/, ""));
      setScanState("error");
    }
  }, []);

  const startCamera = useCallback(async () => {
    setError(null);
    setScanState("scanning");
    try {
      const { BrowserMultiFormatReader } = await import("@zxing/browser");
      const reader = new BrowserMultiFormatReader();
      const controls = await reader.decodeFromVideoDevice(
        undefined,
        videoRef.current!,
        (result) => {
          if (result) void handleQrText(result.getText());
        },
      );
      controlsRef.current = controls;
    } catch {
      setScanState("error");
      setError("Could not access the camera. Use 'Upload image' instead.");
    }
  }, [handleQrText]);

  const handleFileUpload = useCallback(
    async (file: File) => {
      setError(null);
      setScanState("scanning");
      try {
        const { BrowserMultiFormatReader } = await import("@zxing/browser");
        const reader = new BrowserMultiFormatReader();
        const imageUrl = URL.createObjectURL(file);
        try {
          const result = await reader.decodeFromImageUrl(imageUrl);
          await handleQrText(result.getText());
        } finally {
          URL.revokeObjectURL(imageUrl);
        }
      } catch {
        setScanState("error");
        setError("Could not read a QR code from this image.");
      }
    },
    [handleQrText],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const file = e.dataTransfer.files[0];
      if (file && file.type.startsWith("image/")) void handleFileUpload(file);
    },
    [handleFileUpload],
  );

  const handleClose = () => {
    stopCamera();
    if (importedAccounts.length > 0) onImported(importedAccounts.length);
    onClose();
  };

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      title="Import from QR code"
      description="Scan a QR code with your camera or upload an image file."
      size="lg"
      footer={
        <>
          {importedAccounts.length > 0 && (
            <span className="mr-auto text-sm text-success">
              {importedAccounts.length} account(s) imported
            </span>
          )}
          <Button variant="primary" onClick={handleClose}>
            {importedAccounts.length > 0 ? "Done" : "Close"}
          </Button>
        </>
      }
    >
      <div className="mb-4 flex gap-1 rounded-lg bg-bg-subtle p-1">
        <button
          type="button"
          onClick={() => {
            setMode("camera");
            stopCamera();
            setScanState("idle");
          }}
          className={`flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${mode === "camera" ? "bg-bg-elevated text-fg shadow-sm" : "text-fg-muted hover:text-fg"}`}
        >
          Camera
        </button>
        <button
          type="button"
          onClick={() => {
            setMode("file");
            stopCamera();
            setScanState("idle");
          }}
          className={`flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${mode === "file" ? "bg-bg-elevated text-fg shadow-sm" : "text-fg-muted hover:text-fg"}`}
        >
          Upload image
        </button>
      </div>

      {mode === "camera" ? (
        <div>
          {scanState === "idle" && (
            <div className="flex flex-col items-center gap-4 py-8">
              <div className="grid h-16 w-16 place-items-center rounded-full bg-bg-subtle text-fg-subtle">
                <CameraIcon />
              </div>
              <p className="text-center text-sm text-fg-muted max-w-sm">
                Click "Start camera" and point it at a QR code.
              </p>
              <Button variant="primary" onClick={startCamera}>
                Start camera
              </Button>
            </div>
          )}
          {scanState === "scanning" && (
            <div className="relative">
              <video
                ref={videoRef}
                className="mx-auto max-w-md rounded-xl border border-border"
                style={{ aspectRatio: "4/3" }}
                muted
                playsInline
              />
              <div className="pointer-events-none absolute inset-0 mx-auto max-w-md">
                <div className="absolute left-1/2 top-1/2 h-48 w-48 -translate-x-1/2 -translate-y-1/2 rounded-xl border-2 border-accent/70" />
              </div>
              <div className="mt-3 flex justify-center">
                <Button variant="ghost" onClick={stopCamera}>
                  Stop camera
                </Button>
              </div>
            </div>
          )}
          {scanState === "success" && (
            <div className="flex flex-col items-center gap-3 py-8">
              <div className="grid h-16 w-16 place-items-center rounded-full bg-success/10 text-success">
                <CheckCircleIcon />
              </div>
              <p className="text-sm font-medium text-fg">QR code scanned!</p>
              <p className="text-sm text-fg-muted">
                {importedAccounts.length} account(s) imported.
              </p>
              <Button
                variant="secondary"
                onClick={() => {
                  setScanState("idle");
                  void startCamera();
                }}
              >
                Scan another
              </Button>
            </div>
          )}
          {scanState === "error" && (
            <div className="flex flex-col items-center gap-3 py-8">
              <div className="grid h-16 w-16 place-items-center rounded-full bg-danger/10 text-danger">
                <ErrorIcon />
              </div>
              <p className="text-sm font-medium text-fg">Scan failed</p>
              <p className="max-w-sm text-center text-sm text-fg-muted">{error}</p>
              <Button
                variant="secondary"
                onClick={() => {
                  setScanState("idle");
                  void startCamera();
                }}
              >
                Try again
              </Button>
            </div>
          )}
        </div>
      ) : (
        <div>
          <div
            onDrop={handleDrop}
            onDragOver={(e) => e.preventDefault()}
            onClick={() => fileInputRef.current?.click()}
            className="flex cursor-pointer flex-col items-center gap-3 rounded-xl border-2 border-dashed border-border px-6 py-12 transition-colors hover:border-accent hover:bg-bg-subtle"
          >
            <div className="grid h-16 w-16 place-items-center rounded-full bg-bg-subtle text-fg-subtle">
              <UploadIcon />
            </div>
            <p className="text-sm font-medium text-fg">
              Drop a QR image here or click to browse
            </p>
            <p className="text-xs text-fg-subtle">PNG, JPEG, or WebP</p>
            <input
              ref={fileInputRef}
              type="file"
              accept="image/png,image/jpeg,image/webp"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void handleFileUpload(f);
              }}
            />
          </div>
          {error && (
            <p className="mt-3 rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
              {error}
            </p>
          )}
          {importedAccounts.length > 0 && (
            <div className="mt-4 space-y-2">
              <p className="text-sm font-medium text-fg">Imported accounts:</p>
              {importedAccounts.map((a) => (
                <div
                  key={a.id}
                  className="flex items-center gap-2 rounded-lg border border-border bg-bg-subtle px-3 py-2"
                >
                  <Badge variant="success">Added</Badge>
                  <span className="text-sm font-medium text-fg">{a.issuer}</span>
                  <span className="text-sm text-fg-muted">{a.label}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Dialog>
  );
}

function CameraIcon() {
  return (
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
      <circle cx="12" cy="13" r="4" />
    </svg>
  );
}
function CheckCircleIcon() {
  return (
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
      <polyline points="22 4 12 14.01 9 11.01" />
    </svg>
  );
}
function ErrorIcon() {
  return (
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="15" y1="9" x2="9" y2="15" />
      <line x1="9" y1="9" x2="15" y2="15" />
    </svg>
  );
}
function UploadIcon() {
  return (
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="17 8 12 3 7 8" />
      <line x1="12" y1="3" x2="12" y2="15" />
    </svg>
  );
}
