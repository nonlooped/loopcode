import type { ReactNode } from "react";
import { Switch } from "@base-ui/react/switch";
import { cn } from "../../design";

/** Settings card group label. */
export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <h3 className="mb-2 px-0.5 text-[13px] font-semibold text-ink-2">
      {children}
    </h3>
  );
}

/** Raised settings card — groups related rows. */
export function SettingsCard({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "overflow-hidden rounded-card border border-line bg-surface shadow-soft-1",
        className,
      )}
    >
      {children}
    </div>
  );
}

/** One labelled row: title + optional description + trailing control. */
export function SettingRow({
  title,
  description,
  children,
  align = "center",
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  align?: "center" | "start";
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex gap-5 border-b border-line px-4 py-3.5 last:border-b-0",
        align === "center" ? "items-center" : "items-start",
        className,
      )}
    >
      <div className="min-w-0 flex-1">
        <div className="text-[14px] font-medium leading-snug text-ink">{title}</div>
        {description != null && description !== false && (
          <div className="mt-0.5 text-[13px] leading-snug text-ink-3">{description}</div>
        )}
      </div>
      {children != null && <div className="shrink-0">{children}</div>}
    </div>
  );
}

/**
 * Empty state with a clear path forward (Apple: agency + wayfinding).
 * Primary action sits under the copy — not buried in chrome.
 */
export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-start gap-3 px-4 py-8">
      <div>
        <p className="text-[14px] font-medium text-ink">{title}</p>
        <div className="mt-1 max-w-md text-[13px] leading-relaxed text-ink-3">{description}</div>
      </div>
      {action}
    </div>
  );
}

/** Inline status under a card — success or error feedback after an action. */
export function StatusLine({
  tone = "neutral",
  children,
}: {
  tone?: "neutral" | "ok" | "error";
  children: ReactNode;
}) {
  return (
    <p
      className={cn(
        "mt-2 px-0.5 text-[12.5px] leading-snug break-all",
        tone === "ok" && "text-accent",
        tone === "error" && "text-clay",
        tone === "neutral" && "text-ink-3",
      )}
      role={tone === "error" ? "alert" : "status"}
    >
      {children}
    </p>
  );
}

/** Compact switch with instant press feedback (Emil: buttons must feel responsive). */
export function SettingsSwitch({
  checked,
  onCheckedChange,
  disabled,
  "aria-label": ariaLabel,
}: {
  checked: boolean;
  onCheckedChange: (next: boolean) => void;
  disabled?: boolean;
  "aria-label": string;
}) {
  return (
    <Switch.Root
      checked={checked}
      onCheckedChange={onCheckedChange}
      disabled={disabled}
      aria-label={ariaLabel}
      className={cn(
        "relative inline-flex h-[22px] w-[40px] shrink-0 cursor-pointer items-center rounded-full",
        "border border-transparent outline-none",
        "transition-[background-color,box-shadow,transform] duration-[var(--duration-press)] ease-[var(--ease-out)]",
        "active:scale-[0.97]",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
        "data-[checked]:bg-accent data-[unchecked]:bg-line-2",
        "disabled:cursor-not-allowed disabled:opacity-50 disabled:active:scale-100",
      )}
    >
      <Switch.Thumb
        className={cn(
          "pointer-events-none block size-[18px] rounded-full bg-white shadow-soft-1",
          "transition-transform duration-[var(--duration-press)] ease-[var(--ease-out)]",
          "translate-x-[2px] data-[checked]:translate-x-[18px]",
        )}
      />
    </Switch.Root>
  );
}

/** Mono path / slash badge used in list rows. */
export function MonoBadge({ children }: { children: ReactNode }) {
  return (
    <span className="inline-flex max-w-full truncate rounded-md border border-line-2 bg-surface-2 px-1.5 py-0.5 font-mono text-[11.5px] text-ink-2">
      {children}
    </span>
  );
}

/** Group wrapper: label + card + optional footer hint. */
export function SettingsGroup({
  label,
  children,
  hint,
}: {
  label?: string;
  children: ReactNode;
  hint?: ReactNode;
}) {
  return (
    <div>
      {label && <SectionLabel>{label}</SectionLabel>}
      <SettingsCard>{children}</SettingsCard>
      {hint != null && (
        <p className="mt-2 px-0.5 text-[12.5px] leading-snug text-ink-3">{hint}</p>
      )}
    </div>
  );
}
