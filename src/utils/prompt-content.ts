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

export function referenceToken(reference: ComposerReference) {
  return reference.kind === "skill" ? `$${reference.name}` : `@${reference.relativePath}`;
}

export function promptText(parts: PromptPart[]) {
  return parts
    .map((part) => (part.type === "text" ? part.text : referenceToken(part.reference)))
    .join("");
}

/**
 * Rebuild structured parts from prompt text that was edited as plain text, so an edited
 * prompt still sends `resource_link` blocks instead of a bare `@path` string. Tokens the
 * edit removed are dropped; tokens it kept resolve back to their original reference.
 */
export function promptPartsFromText(
  text: string,
  references: ComposerReference[] = [],
): PromptPart[] {
  if (references.length === 0) return text ? [{ type: "text", text }] : [];
  // Longest first so `@src/foo.test.ts` wins over `@src/foo.ts` at the same position.
  const tokens = references
    .map((reference) => ({ reference, token: referenceToken(reference) }))
    .sort((left, right) => right.token.length - left.token.length);

  // Characters that can extend a reference token (file path segments, skill names) so a
  // token boundary isn't just "starts with" — `@src/foo.ts` must not match `@src/foo.tsx`.
  const isTokenChar = (character: string | undefined) =>
    character !== undefined && /[\w./-]/.test(character);

  const parts: PromptPart[] = [];
  let plain = "";
  let index = 0;
  while (index < text.length) {
    const match = tokens.find(
      ({ token }) =>
        text.startsWith(token, index) && !isTokenChar(text[index + token.length]),
    );
    if (!match) {
      plain += text[index];
      index += 1;
      continue;
    }
    if (plain) {
      parts.push({ type: "text", text: plain });
      plain = "";
    }
    parts.push({ type: "reference", reference: { ...match.reference } });
    index += match.token.length;
  }
  if (plain) parts.push({ type: "text", text: plain });
  return parts;
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
