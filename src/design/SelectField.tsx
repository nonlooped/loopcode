import { useState, type ReactNode } from "react";
import { Select } from "@base-ui/react/select";
import { IconChevronDown } from "@tabler/icons-react";
import { cn } from "./cn";
import { controlChrome, pressable } from "./interactive";

export interface SelectOption<T extends string = string> {
  value: T;
  label: ReactNode;
  /** Plain string for Base UI item matching / a11y when label is rich. */
  textValue?: string;
  /** Greyed out and non-selectable (e.g. unauthenticated providers). */
  disabled?: boolean;
  /** Optional one-line description shown below the label in the dropdown. */
  description?: string;
}

interface SelectFieldProps<T extends string> {
  value: T;
  onValueChange: (value: T) => void;
  items: SelectOption<T>[];
  /** Accessible name when no visible label is shown. */
  "aria-label"?: string;
  /** default = bordered control; ghost = quiet in-composer density. */
  variant?: "default" | "ghost";
  /** Disable the whole control (empty catalog, loading, etc.). */
  disabled?: boolean;
  /**
   * Show a filter field when the list is long.
   * Defaults to true when there are more than 10 items.
   */
  searchable?: boolean;
  className?: string;
  triggerClassName?: string;
  popupClassName?: string;
}

function optionText(option: SelectOption): string {
  if (option.textValue) return option.textValue;
  if (typeof option.label === "string" || typeof option.label === "number") {
    return String(option.label);
  }
  return option.value;
}

const ghostChrome =
  pressable +
  " inline-flex items-center gap-1.5 rounded-control border border-transparent bg-transparent " +
  "px-2 py-1 text-[13px] font-medium text-ink-2 " +
  "fine-hover:bg-surface-2 fine-hover:text-ink " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent";

const itemChrome =
  "cursor-pointer rounded-md px-3 py-1.5 text-[14px] text-ink outline-none " +
  "data-[highlighted]:bg-accent-tint data-[highlighted]:text-accent " +
  "data-[selected]:font-semibold " +
  "data-[disabled]:cursor-not-allowed data-[disabled]:opacity-40 " +
  "data-[disabled]:data-[highlighted]:bg-transparent data-[disabled]:data-[highlighted]:text-ink-3";

/**
 * Origin-aware select with shared control chrome.
 * Popup scales from Base UI's --transform-origin (never from center by default).
 */
export function SelectField<T extends string>({
  value,
  onValueChange,
  items,
  "aria-label": ariaLabel,
  variant = "default",
  disabled = false,
  searchable,
  className,
  triggerClassName,
  popupClassName,
}: SelectFieldProps<T>) {
  const [query, setQuery] = useState("");
  const showSearch = searchable ?? items.length > 10;

  const selectItems = items.map((i) => ({ value: i.value, label: optionText(i) }));
  const selected = items.find((i) => i.value === value);

  const visible = (() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter((i) => {
      const hay = `${optionText(i)} ${i.value}`.toLowerCase();
      return hay.includes(q);
    });
  })();

  return (
    <Select.Root
      value={value}
      onValueChange={(v) => {
        if (v == null) return;
        const opt = items.find((i) => i.value === v);
        if (opt?.disabled) return;
        onValueChange(v as T);
      }}
      onOpenChange={(open) => {
        if (!open) setQuery("");
      }}
      items={selectItems}
      disabled={disabled}
    >
      <Select.Trigger
        aria-label={ariaLabel}
        className={cn(
          variant === "ghost" ? ghostChrome : controlChrome,
          disabled && "opacity-50",
          triggerClassName,
          className,
        )}
      >
        <Select.Value>{selected?.label ?? value}</Select.Value>
        <Select.Icon className="text-ink-3" aria-hidden="true">
          <IconChevronDown size={14} stroke={2} className="shrink-0" aria-hidden />
        </Select.Icon>
      </Select.Trigger>
      <Select.Portal>
        {/*
          alignItemWithTrigger (default true) overlays the selected row on the
          trigger, native-<select>-style. That math assumes single-line items;
          our items can render a wrapped label + description, which threw the
          computed offset way off (popups spanning the viewport or landing
          off-screen). Anchor below the trigger like every other popover instead.
        */}
        <Select.Positioner sideOffset={6} alignItemWithTrigger={false} className="z-50">
          <Select.Popup
            className={cn(
              "surface-popover flex min-w-[var(--anchor-width,8rem)] max-h-[min(22rem,55vh)] flex-col " +
                "rounded-card border border-line bg-surface p-1 shadow-soft-3 outline-none",
              popupClassName,
            )}
          >
            {showSearch && (
              <div className="sticky top-0 z-10 bg-surface p-1 pb-1.5">
                <input
                  type="search"
                  value={query}
                  onChange={(e) => setQuery(e.currentTarget.value)}
                  onKeyDown={(e) => e.stopPropagation()}
                  placeholder="Filter…"
                  aria-label="Filter options"
                  className={
                    "w-full rounded-md border border-line bg-surface-2 px-2.5 py-1.5 " +
                    "text-[13px] text-ink placeholder:text-ink-3 outline-none " +
                    "focus:border-accent"
                  }
                />
              </div>
            )}
            <div className="min-h-0 flex-1 overflow-auto">
              {visible.length === 0 && (
                <div className="px-3 py-2 text-[13px] text-ink-3">No matches</div>
              )}
              {visible.map((m) => (
                <Select.Item
                  key={m.value}
                  value={m.value}
                  label={optionText(m)}
                  disabled={m.disabled}
                  className={itemChrome}
                >
                  <div className="flex flex-col">
                    <Select.ItemText>{m.label}</Select.ItemText>
                    {m.description && (
                      <span className="text-[12px] font-normal text-ink-2">{m.description}</span>
                    )}
                  </div>
                </Select.Item>
              ))}
            </div>
          </Select.Popup>
        </Select.Positioner>
      </Select.Portal>
    </Select.Root>
  );
}
