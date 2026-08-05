import { forwardRef, type HTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  interactive?: boolean;
  padding?: "none" | "sm" | "md" | "lg";
}

const paddingClasses = {
  none: "",
  sm: "p-3",
  md: "p-4",
  lg: "p-6",
};

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, interactive, padding = "md", ...rest }, ref) => (
    <div
      ref={ref}
      className={cn(
        "rounded-xl border border-border bg-bg-elevated shadow-card",
        interactive && "card-hover cursor-pointer",
        paddingClasses[padding],
        className,
      )}
      {...rest}
    />
  ),
);
Card.displayName = "Card";
