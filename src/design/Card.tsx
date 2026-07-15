import type { HTMLAttributes } from "react";
import { cn } from "./cn";

/** Raised surface panel with the soft elevation from the token set. */
export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-panel border border-line bg-surface shadow-soft-1",
        className,
      )}
      {...props}
    />
  );
}
