import { cubicOut } from "svelte/easing";

/** Matches shell.css layout timing: cubic-bezier(0.32, 0.72, 0, 1). */
export const shellLayoutEase = cubicOut;

export const SHELL_LAYOUT_MS = 220;

export function shellLayoutDuration(reducedMotion: boolean, resizing = false) {
  return reducedMotion || resizing ? 0 : SHELL_LAYOUT_MS;
}
