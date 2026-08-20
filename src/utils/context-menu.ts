export interface ContextMenuItem {
  label: string;
  action: () => void | Promise<void>;
  danger?: boolean;
  disabled?: boolean;
  separatorBefore?: boolean;
  shortcut?: string;
}

export interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
}

export function contextMenuPosition(
  x: number,
  y: number,
  menuWidth: number,
  menuHeight: number,
  viewportWidth: number,
  viewportHeight: number,
  margin = 8,
) {
  return {
    x: Math.max(margin, Math.min(x, viewportWidth - menuWidth - margin)),
    y: Math.max(margin, Math.min(y, viewportHeight - menuHeight - margin)),
  };
}

export type MenuNavigationKey = "ArrowDown" | "ArrowUp" | "Home" | "End";

export function isMenuNavigationKey(key: string): key is MenuNavigationKey {
  return key === "ArrowDown" || key === "ArrowUp" || key === "Home" || key === "End";
}

export function nextMenuItemIndex(current: number, length: number, key: MenuNavigationKey) {
  if (length === 0) return -1;
  if (key === "Home") return 0;
  if (key === "End") return length - 1;
  return (current + (key === "ArrowDown" ? 1 : -1) + length) % length;
}

export function menuFromEvent(event: MouseEvent, items: ContextMenuItem[]): ContextMenuState {
  event.preventDefault();
  event.stopPropagation();
  return { x: event.clientX, y: event.clientY, items };
}
