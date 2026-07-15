import { useState } from "react";
import { IconChevronRight } from "@tabler/icons-react";
import { Button, cn } from "../../../design";
import { pressable } from "../../../design/interactive";
import { SettingsCard } from "../layout";

/**
 * Custom endpoints are intentionally unavailable until Core has a provider
 * adapter that can probe and run them safely. Hiding this limitation used to
 * invite users to enter credentials into a workflow that could never work.
 */
export function CustomProviderForm() {
  const [open, setOpen] = useState(false);

  return (
    <SettingsCard>
      <Button
        variant="ghost"
        onClick={() => setOpen((value) => !value)}
        className={cn(pressable, "flex w-full items-center justify-between gap-3 px-4 py-3 text-left")}
        aria-expanded={open}
      >
        <div>
          <p className="text-[14px] font-medium text-ink">Custom provider</p>
          <p className="text-[13px] text-ink-3">Compatible endpoints will be available in a future Core adapter.</p>
        </div>
        <IconChevronRight
          size={16}
          stroke={2}
          className={cn("shrink-0 text-ink-3 transition-transform duration-[var(--duration-fast)]", open && "rotate-90")}
          aria-hidden
        />
      </Button>
      {open && (
        <div className="border-t border-line px-4 py-4 text-[13px] leading-relaxed text-ink-2">
          Custom providers are not available yet. LoopCode only accepts credentials for providers with a Core-owned adapter, so it can validate and use them without guessing success.
        </div>
      )}
    </SettingsCard>
  );
}
