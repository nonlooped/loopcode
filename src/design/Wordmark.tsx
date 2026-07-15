import { IconInfinity } from "@tabler/icons-react";
import { cn } from "./cn";

/** LoopCode mark + wordmark. Accent-colored, theme-aware. */
export function Wordmark({ className, markOnly = false }: { className?: string; markOnly?: boolean }) {
  return (
    <span className={cn("inline-flex items-center gap-2.5 font-bold tracking-tight", className)}>
      <IconInfinity
        className="h-[1.4em] w-[1.4em] shrink-0 text-accent"
        stroke={2.25}
        aria-hidden
      />
      {!markOnly && (
        <span>
          Loop<span className="text-accent">Code</span>
        </span>
      )}
    </span>
  );
}
