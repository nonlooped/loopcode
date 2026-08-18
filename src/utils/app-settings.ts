import type { PermissionMode } from "../types/index.ts";

const PERMISSION_MODE_KEY = "loopcode.permission-mode";
const LEFT_SIDEBAR_WIDTH_KEY = "loopcode.left-sidebar-width";
const RIGHT_SIDEBAR_WIDTH_KEY = "loopcode.right-sidebar-width";

export const LEFT_SIDEBAR_WIDTH_RANGE = { min: 190, max: 480 } as const;
export const RIGHT_SIDEBAR_WIDTH_RANGE = { min: 210, max: 520 } as const;

export interface SidebarWidths {
  left: number | null;
  right: number | null;
}

export function loadPermissionMode(
  storage: Pick<Storage, "getItem"> = localStorage,
): PermissionMode {
  try {
    return storage.getItem(PERMISSION_MODE_KEY) === "full" ? "full" : "restricted";
  } catch {
    return "restricted";
  }
}

export function savePermissionMode(
  mode: PermissionMode,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  try {
    storage.setItem(PERMISSION_MODE_KEY, mode);
  } catch {
    // Settings persistence is best-effort when web storage is unavailable.
  }
}

export function loadSidebarWidths(storage: Pick<Storage, "getItem"> = localStorage): SidebarWidths {
  try {
    return {
      left: storedWidth(storage.getItem(LEFT_SIDEBAR_WIDTH_KEY), LEFT_SIDEBAR_WIDTH_RANGE),
      right: storedWidth(storage.getItem(RIGHT_SIDEBAR_WIDTH_KEY), RIGHT_SIDEBAR_WIDTH_RANGE),
    };
  } catch {
    return { left: null, right: null };
  }
}

export function saveSidebarWidth(
  side: keyof SidebarWidths,
  width: number,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  try {
    storage.setItem(
      side === "left" ? LEFT_SIDEBAR_WIDTH_KEY : RIGHT_SIDEBAR_WIDTH_KEY,
      String(width),
    );
  } catch {
    // Layout persistence is best-effort when web storage is unavailable.
  }
}

function storedWidth(value: string | null, range: { min: number; max: number }) {
  if (value === null) return null;
  const width = Number(value);
  if (!Number.isFinite(width)) return null;
  return Math.min(range.max, Math.max(range.min, width));
}
