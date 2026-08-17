import type { PermissionMode } from "../types/index.ts";

const PERMISSION_MODE_KEY = "loopcode.permission-mode";

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
