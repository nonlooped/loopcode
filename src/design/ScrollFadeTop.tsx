import { cn } from "./cn";

/**
 * Soft top-edge fade for scrollable rails/panels — the same Apple-materials
 * technique the composer uses to blend the timeline under it. Render inside
 * a `relative` wrapper as a sibling above the scrollable element.
 */
export function ScrollFadeTop({ className }: { className?: string }) {
  return (
    <div
      aria-hidden
      className={cn(
        "pointer-events-none absolute inset-x-0 top-0 z-10 h-6 bg-gradient-to-b from-surface-2 to-transparent",
        className,
      )}
    />
  );
}
