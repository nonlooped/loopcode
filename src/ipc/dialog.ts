import { open } from "@tauri-apps/plugin-dialog";
import { isTauri } from "./client";

/** Native OS folder picker (via tauri-plugin-dialog). Returns null if cancelled. */
export async function pickFolder(): Promise<string | null> {
  if (!isTauri()) return null;
  const result = await open({
    directory: true,
    multiple: false,
    title: "Open a project folder",
  });
  return typeof result === "string" ? result : null;
}
