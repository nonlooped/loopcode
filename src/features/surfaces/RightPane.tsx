import type { ComponentType } from "react";
import { IconFileDiff, IconFileText } from "@tabler/icons-react";
import { Tabs } from "@base-ui/react/tabs";
import { FilesPanel } from "./FilesPanel";
import { DiffsPanel } from "./DiffsPanel";
import { useUi } from "../../store/ui";
import type { RightTab } from "../../store/ui";
import { cn, pressable } from "../../design";
import { SidebarCollapseButton } from "../cockpit/SidebarCollapseButton";

const TABS: {
  value: RightTab;
  label: string;
  icon: ComponentType<{ size?: number; stroke?: number; className?: string; "aria-hidden"?: boolean }>;
}[] = [
  { value: "files", label: "Files", icon: IconFileText },
  { value: "diffs", label: "Diffs", icon: IconFileDiff },
];

export function RightPane() {
  const rightTab = useUi((s) => s.rightTab);
  const setRightTab = useUi((s) => s.setRightTab);
  const toggleRight = useUi((s) => s.toggleRight);

  return (
    <Tabs.Root
      value={rightTab}
      onValueChange={(v) => setRightTab(v as RightTab)}
      className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden border-l border-line bg-surface-2"
    >
      <Tabs.List className="relative flex min-w-0 items-center gap-1 border-b border-line px-1.5 py-2">
        {TABS.map((t) => {
          const Icon = t.icon;
          return (
            <Tabs.Tab
              key={t.value}
              value={t.value}
              title={t.label}
              className={cn(
                pressable,
                "inline-flex min-w-0 flex-1 items-center justify-center gap-1.5 rounded-md px-1.5 py-1.5 text-[12px] outline-none",
                "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
                // Base UI uses data-active (not data-selected) for the current tab.
                "text-ink-3 fine-hover:bg-surface fine-hover:text-ink",
                "data-[active]:bg-accent-tint data-[active]:font-semibold data-[active]:text-accent",
                "data-[active]:shadow-[inset_0_0_0_1px_var(--accent-line)]",
              )}
            >
              <Icon size={14} stroke={1.75} className="shrink-0" aria-hidden />
              <span className="min-w-0 truncate">{t.label}</span>
            </Tabs.Tab>
          );
        })}
        <SidebarCollapseButton side="right" onCollapse={toggleRight} />
      </Tabs.List>
      <Tabs.Panel value="files" keepMounted className="min-h-0 flex-1">
        <FilesPanel />
      </Tabs.Panel>
      <Tabs.Panel value="diffs" keepMounted className="min-h-0 flex-1">
        <DiffsPanel />
      </Tabs.Panel>
    </Tabs.Root>
  );
}
