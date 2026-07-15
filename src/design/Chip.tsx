import type { ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";
import { cn } from "./cn";

export type ChipTone = "neutral" | "accent" | "amber";

const tones: Record<ChipTone, string> = {
  neutral: "border-line-2 bg-surface text-ink-2",
  accent: "border-accent-line bg-accent-tint text-accent",
  amber: "border-amber bg-amber-tint text-amber",
};

const dotTones: Record<ChipTone, string> = {
  neutral: "bg-ink-3",
  accent: "bg-accent",
  amber: "bg-amber",
};

interface ChipProps {
  children: ReactNode;
  tone?: ChipTone;
  /** Show a state dot (status semantics never rely on color alone). */
  dot?: boolean;
  /** Subtle live pulse on the dot only (running / working status). */
  live?: boolean;
  /** Quieter visual weight for secondary metrics. */
  quiet?: boolean;
  className?: string;
}

export function Chip({
  children,
  tone = "neutral",
  dot = false,
  live = false,
  quiet = false,
  className,
}: ChipProps) {
  const reduce = useReducedMotion();
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[13px] font-semibold",
        tones[tone],
        quiet && "border-transparent bg-transparent px-1.5 py-0.5 text-[12px] font-medium text-ink-3",
        className,
      )}
    >
      {dot && (
        <motion.span
          animate={live && !reduce ? { opacity: [1, 0.45, 1] } : { opacity: live ? 0.72 : 1 }}
          transition={live && !reduce ? { duration: 1.6, repeat: Infinity, ease: "easeInOut" } : { duration: 0 }}
          className={cn("h-1.5 w-1.5 rounded-full", dotTones[tone])}
          aria-hidden="true"
        />
      )}
      {children}
    </span>
  );
}
