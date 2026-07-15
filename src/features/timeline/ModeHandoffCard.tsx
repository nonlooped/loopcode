import { useState } from "react";
import { Button, cn } from "../../design";
import type { Mode, TimelineItem } from "../../ipc/types";
import { useSession } from "../../store/session";
import { isModePolicyMutatingDenial } from "./toolActivity";

function modeLabel(mode: Mode): string {
  if (mode === "plan") return "Plan";
  if (mode === "debug") return "Debug";
  return mode;
}

/** Inline CTA when Plan/Debug refuses a mutating tool — user must switch to Build. */
export function ModeHandoffCard({ item }: { item: TimelineItem }) {
  const mode = useSession((s) => s.mode);
  const setMode = useSession((s) => s.setMode);
  const [dismissed, setDismissed] = useState(false);

  if (dismissed || mode === "build" || mode === "ask") return null;
  if (!isModePolicyMutatingDenial(item)) return null;

  return (
    <div
      className={cn(
        "mt-3 rounded-card border border-line bg-surface-2 px-3.5 py-3",
        "text-[13px] leading-relaxed text-ink-2",
      )}
      role="status"
    >
      <p className="font-medium text-ink">
        {modeLabel(mode)} mode can&apos;t edit files
      </p>
      <p className="mt-1 text-ink-3">
        Switch to Build to apply changes. Build mode asks before shell commands
        and other sensitive actions.
      </p>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button
          className="px-3 py-1.5 text-[12.5px]"
          onClick={() => setMode("build")}
        >
          Switch to Build
        </Button>
        <Button
          variant="ghost"
          className="px-2.5 py-1.5 text-[12.5px] text-ink-3"
          onClick={() => setDismissed(true)}
        >
          Dismiss
        </Button>
      </div>
    </div>
  );
}
