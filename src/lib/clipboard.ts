import { clipboardWriteText } from "../ipc/commands";
import { isTauri } from "../ipc/client";

/** Copy text through Core; browser preview deliberately does not access the OS clipboard. */
export async function copyText(text: string): Promise<void> {
  if (!isTauri()) return;
  await clipboardWriteText(text);
}
