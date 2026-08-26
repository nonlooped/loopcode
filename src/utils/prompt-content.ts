import type { ComposerReference, PromptPart } from "../types/index.ts";
import type { SendShortcut } from "./app-settings.ts";

export const REFERENCE_PLACEHOLDER = "\uFFFC";

export function promptParts(draft: string, references: ComposerReference[]): PromptPart[] {
  const textParts = draft.split(REFERENCE_PLACEHOLDER);
  const parts: PromptPart[] = [];

  for (const [index, text] of textParts.entries()) {
    if (text) parts.push({ type: "text", text });
    const reference = references[index];
    if (reference) parts.push({ type: "reference", reference: { ...reference } });
  }

  return parts;
}

export function promptText(parts: PromptPart[]) {
  return parts
    .map((part) => {
      if (part.type === "text") return part.text;
      return part.reference.kind === "skill"
        ? `$${part.reference.name}`
        : `@${part.reference.relativePath}`;
    })
    .join("");
}

export function hasPromptContent(draft: string, references: ComposerReference[] = []) {
  return references.length > 0 || draft.replaceAll(REFERENCE_PLACEHOLDER, "").trim().length > 0;
}

export function composerEnterAction(
  shortcut: SendShortcut,
  event: Pick<KeyboardEvent, "key" | "shiftKey" | "ctrlKey" | "metaKey" | "isComposing">,
) {
  if (event.isComposing || event.key !== "Enter") return undefined;
  if (event.shiftKey) return "newline";
  if (shortcut === "modifier-enter" && !event.ctrlKey && !event.metaKey) return "newline";
  return "send";
}

export function fuzzyScore(value: string, query: string) {
  const haystack = value.toLowerCase();
  const needle = query.trim().toLowerCase();
  if (!needle) return 0;
  if (haystack.startsWith(needle)) return 1_000 - haystack.length;

  let score = 0;
  let position = -1;
  let previous = -2;
  for (const character of needle) {
    position = haystack.indexOf(character, position + 1);
    if (position < 0) return undefined;
    score += position === previous + 1 ? 8 : 1;
    previous = position;
  }
  return score - haystack.length / 100;
}
