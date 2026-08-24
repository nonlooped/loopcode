const TITLE_INSTRUCTION =
  "Generate a 3–6 word title for this request. Return only the title; do not use tools.";
const MAX_REQUEST_LENGTH = 800;
const MAX_TITLE_LENGTH = 60;

export function newThreadTitle(): string {
  return "New Thread";
}

export function buildThreadTitlePrompt(request: string): string {
  const conciseRequest = request.trim().replace(/\s+/g, " ").slice(0, MAX_REQUEST_LENGTH).trimEnd();
  return `${TITLE_INSTRUCTION}\n\n${conciseRequest}`;
}

export function normalizeThreadTitle(value: string): string | undefined {
  let title = value.trim().split(/\r?\n/, 1)[0] ?? "";
  title = title
    .replace(/^[#>\s]+/, "")
    .replace(/[*_`]/g, "")
    .replace(/^title\s*:\s*/i, "")
    .replace(/^["'“”‘’]+|["'“”‘’]+$/g, "")
    .replace(/[.!?:;,–—-]+$/, "")
    .replace(/\s+/g, " ")
    .trim();
  if (!/[\p{L}\p{N}]/u.test(title)) return undefined;
  if (title.length <= MAX_TITLE_LENGTH) return title;
  return title
    .slice(0, MAX_TITLE_LENGTH)
    .replace(/\s+\S*$/, "")
    .trimEnd();
}
