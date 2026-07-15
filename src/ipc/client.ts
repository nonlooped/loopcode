import { invoke as tauriInvoke } from "@tauri-apps/api/core";

/** True when running inside the Tauri WebView (vs. a plain browser preview). */
export function isTauri(): boolean {
  const w = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(w.__TAURI_INTERNALS__ || w.__TAURI__);
}

/** Raised when an IPC command is attempted outside the Tauri host. */
export class NotInTauriError extends Error {
  constructor(command: string) {
    super(`Core is unavailable in browser preview (command: ${command}).`);
    this.name = "NotInTauriError";
  }
}

/** Typed wrapper around Tauri invoke — the single gateway to Core. */
export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri()) throw new NotInTauriError(command);
  return tauriInvoke<T>(command, args);
}
