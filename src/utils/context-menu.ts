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

export function menuFromEvent(event: MouseEvent, items: ContextMenuItem[]): ContextMenuState {
  event.preventDefault();
  event.stopPropagation();
  return { x: event.clientX, y: event.clientY, items };
}
