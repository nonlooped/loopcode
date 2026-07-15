import type { ReactNode } from "react";
import { ContextMenu as BaseContextMenu } from "@base-ui/react/context-menu";
import { cn } from "./cn";

export type ContextMenuItem = {
  label: string;
  onSelect?: () => void;
  disabled?: boolean;
  tone?: "default" | "danger";
};

type ContextMenuProps = {
  children: ReactNode;
  items: Array<ContextMenuItem | "separator">;
};

/** Native-feeling app menu, shared by every custom right-click surface. */
export function ContextMenu({ children, items }: ContextMenuProps) {
  return (
    <BaseContextMenu.Root>
      <BaseContextMenu.Trigger className="contents">{children}</BaseContextMenu.Trigger>
      <BaseContextMenu.Portal>
        <BaseContextMenu.Positioner sideOffset={6} alignOffset={-4}>
          <BaseContextMenu.Popup
            className={cn(
              "surface-popover z-50 min-w-48 rounded-control border border-line bg-surface p-1 shadow-soft-2",
              "outline-none",
            )}
          >
            {items.map((item, index) =>
              item === "separator" ? (
                <div key={`separator-${index}`} className="my-1 h-px bg-line" role="separator" />
              ) : (
                <BaseContextMenu.Item
                  key={item.label}
                  disabled={item.disabled}
                  onClick={item.onSelect}
                  className={cn(
                    "flex min-h-8 w-full items-center rounded-md px-2.5 py-1.5 text-left text-[13px] font-medium",
                    "text-ink outline-none transition-colors duration-[var(--duration-fast)]",
                    "data-[highlighted]:bg-surface-2 data-[highlighted]:text-ink",
                    "data-[disabled]:cursor-not-allowed data-[disabled]:text-ink-3",
                    item.tone === "danger" && "text-clay data-[highlighted]:bg-clay-tint data-[highlighted]:text-clay",
                  )}
                >
                  {item.label}
                </BaseContextMenu.Item>
              ),
            )}
          </BaseContextMenu.Popup>
        </BaseContextMenu.Positioner>
      </BaseContextMenu.Portal>
    </BaseContextMenu.Root>
  );
}
