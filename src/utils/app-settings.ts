import type { PermissionMode } from "../types/index.ts";

const PERMISSION_MODE_KEY = "loopcode.permission-mode";
const LEFT_SIDEBAR_WIDTH_KEY = "loopcode.left-sidebar-width";
const RIGHT_SIDEBAR_WIDTH_KEY = "loopcode.right-sidebar-width";
const TERMINAL_HEIGHT_KEY = "loopcode.terminal-height";
const LINUX_SHELL_TRANSPARENCY_KEY = "loopcode.linux-shell-transparency-level";
const LEGACY_LINUX_SHELL_TRANSPARENCY_KEY = "loopcode.linux-shell-transparency";

export const LEFT_SIDEBAR_WIDTH_RANGE = { min: 190, max: 480 } as const;
export const RIGHT_SIDEBAR_WIDTH_RANGE = { min: 210, max: 520 } as const;
export const TERMINAL_HEIGHT_RANGE = { min: 140, max: 520 } as const;
export const DEFAULT_TERMINAL_HEIGHT = 260;
export const LINUX_SHELL_TRANSPARENCY_RANGE = { min: 0, max: 100 } as const;
export const MAX_LINUX_SHELL_TRANSPARENCY = 20;

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

export function loadTerminalHeight(storage: Pick<Storage, "getItem"> = localStorage) {
  try {
    return (
      storedWidth(storage.getItem(TERMINAL_HEIGHT_KEY), TERMINAL_HEIGHT_RANGE) ??
      DEFAULT_TERMINAL_HEIGHT
    );
  } catch {
    return DEFAULT_TERMINAL_HEIGHT;
  }
}

export function saveTerminalHeight(
  height: number,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  try {
    storage.setItem(TERMINAL_HEIGHT_KEY, String(height));
  } catch {
    // Layout persistence is best-effort when web storage is unavailable.
  }
}

export function loadLinuxShellTransparency(storage: Pick<Storage, "getItem"> = localStorage) {
  try {
    const savedLevel = storedWidth(
      storage.getItem(LINUX_SHELL_TRANSPARENCY_KEY),
      LINUX_SHELL_TRANSPARENCY_RANGE,
    );
    if (savedLevel !== null) return savedLevel;

    const legacyTransparency = storedWidth(storage.getItem(LEGACY_LINUX_SHELL_TRANSPARENCY_KEY), {
      min: 0,
      max: MAX_LINUX_SHELL_TRANSPARENCY,
    });
    return legacyTransparency === null
      ? LINUX_SHELL_TRANSPARENCY_RANGE.min
      : Math.round((legacyTransparency / MAX_LINUX_SHELL_TRANSPARENCY) * 100);
  } catch {
    return LINUX_SHELL_TRANSPARENCY_RANGE.min;
  }
}

export function saveLinuxShellTransparency(
  transparency: number,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  const clamped = Math.min(
    LINUX_SHELL_TRANSPARENCY_RANGE.max,
    Math.max(LINUX_SHELL_TRANSPARENCY_RANGE.min, Math.round(transparency)),
  );
  try {
    storage.setItem(LINUX_SHELL_TRANSPARENCY_KEY, String(clamped));
  } catch {
    // Settings persistence is best-effort when web storage is unavailable.
  }
  return clamped;
}

function storedWidth(value: string | null, range: { min: number; max: number }) {
  if (value === null) return null;
  const width = Number(value);
  if (!Number.isFinite(width)) return null;
  return Math.min(range.max, Math.max(range.min, width));
}
