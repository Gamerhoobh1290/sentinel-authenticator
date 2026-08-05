import {
  forwardRef,
  type InputHTMLAttributes,
  type ReactNode,
  useId,
  useState,
} from "react";
import { cn } from "@/lib/cn";

export interface TextFieldProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "size"
> {
  label?: string;
  error?: string;
  hint?: string;
  /** Slot for trailing icon/button (e.g. password reveal toggle). */
  trailing?: ReactNode;
  /** Slot for leading icon. */
  leading?: ReactNode;
  containerClassName?: string;
}

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(
  (
    {
      label,
      error,
      hint,
      trailing,
      leading,
      className,
      containerClassName,
      id: idProp,
      type = "text",
      ...rest
    },
    ref,
  ) => {
    const autoId = useId();
    const id = idProp ?? autoId;
    const hintId = `${id}-hint`;
    const errorId = `${id}-error`;
    const describedBy = error ? errorId : hint ? hintId : undefined;

    return (
      <div className={cn("flex flex-col gap-1.5", containerClassName)}>
        {label && (
          <label htmlFor={id} className="text-xs font-medium text-fg-muted select-none">
            {label}
          </label>
        )}
        <div className="relative flex items-center">
          {leading && (
            <span className="pointer-events-none absolute left-3 text-fg-subtle">
              {leading}
            </span>
          )}
          <input
            ref={ref}
            id={id}
            type={type}
            aria-invalid={Boolean(error) || undefined}
            aria-describedby={describedBy}
            className={cn(
              "h-9 w-full rounded-lg border bg-bg-subtle px-3 text-sm text-fg",
              "placeholder:text-fg-subtle",
              "transition-colors duration-160",
              "focus:outline focus:outline-2 focus:outline-accent focus:outline-offset-0",
              "disabled:opacity-50 disabled:cursor-not-allowed",
              leading && "pl-9",
              trailing && "pr-9",
              error
                ? "border-danger focus:outline-danger"
                : "border-border focus:border-accent",
              className,
            )}
            {...rest}
          />
          {trailing && (
            <span className="absolute right-1.5 flex items-center">{trailing}</span>
          )}
        </div>
        {error ? (
          <p id={errorId} className="text-xs text-danger">
            {error}
          </p>
        ) : hint ? (
          <p id={hintId} className="text-xs text-fg-subtle">
            {hint}
          </p>
        ) : null}
      </div>
    );
  },
);
TextField.displayName = "TextField";

/** Convenience password input with built-in reveal toggle. */
export const PasswordField = forwardRef<
  HTMLInputElement,
  Omit<TextFieldProps, "type" | "trailing">
>(({ ...rest }, ref) => {
  const [revealed, setRevealed] = useState(false);
  return (
    <TextField
      ref={ref}
      type={revealed ? "text" : "password"}
      autoComplete="off"
      spellCheck={false}
      trailing={
        <button
          type="button"
          onClick={() => setRevealed((r) => !r)}
          className="grid h-7 w-7 place-items-center rounded-md text-fg-subtle transition-colors hover:bg-bg-subtle hover:text-fg"
          aria-label={revealed ? "Hide password" : "Show password"}
          aria-pressed={revealed}
          tabIndex={-1}
        >
          <EyeIcon revealed={revealed} />
        </button>
      }
      {...rest}
    />
  );
});
PasswordField.displayName = "PasswordField";

function EyeIcon({ revealed }: { revealed: boolean }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {revealed ? (
        <>
          <path d="M9.88 9.88a3 3 0 1 0 4.24 4.24" />
          <path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68" />
          <path d="M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61" />
          <line x1="2" y1="2" x2="22" y2="22" />
        </>
      ) : (
        <>
          <path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" />
          <circle cx="12" cy="12" r="3" />
        </>
      )}
    </svg>
  );
}
