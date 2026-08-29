export type DiffLineKind = "add" | "del" | "hunk" | "ctx";

export interface DiffLine {
  kind: DiffLineKind;
  text: string;
}

export function unifiedDiffLines(
  oldText: string | null,
  newText: string | null,
  context = 3,
): DiffLine[] {
  const before = splitLines(oldText);
  const after = splitLines(newText);
  let head = 0;
  while (head < before.length && before[head] === after[head]) head += 1;
  if (head === before.length && head === after.length) return [];

  let tail = 0;
  while (
    tail < before.length - head &&
    tail < after.length - head &&
    before.at(-tail - 1) === after.at(-tail - 1)
  )
    tail += 1;

  const start = Math.max(0, head - context);
  const beforeEnd = before.length - tail;
  const afterEnd = after.length - tail;
  const trailing = before.slice(beforeEnd, beforeEnd + context);
  const oldCount = beforeEnd - start + trailing.length;
  const newCount = afterEnd - start + trailing.length;

  // ponytail: one contiguous hunk; add an LCS library if coarse multi-edit diffs become a problem.
  return [
    { kind: "hunk", text: `@@ -${start + 1},${oldCount} +${start + 1},${newCount} @@` },
    ...before.slice(start, head).map((text): DiffLine => ({ kind: "ctx", text: ` ${text}` })),
    ...before.slice(head, beforeEnd).map((text): DiffLine => ({ kind: "del", text: `-${text}` })),
    ...after.slice(head, afterEnd).map((text): DiffLine => ({ kind: "add", text: `+${text}` })),
    ...trailing.map((text): DiffLine => ({ kind: "ctx", text: ` ${text}` })),
  ];
}

export function fileChangeDiffLines(diff: string, kind: "add" | "update" | "delete"): DiffLine[] {
  const lines = splitLines(diff);
  if (kind === "update") return lines.map((text) => ({ kind: diffLineKind(text), text }));
  const added = kind === "add";
  return lines.map((text) => ({
    kind: added ? "add" : "del",
    text: `${added ? "+" : "-"}${text}`,
  }));
}

function diffLineKind(line: string): DiffLineKind {
  if (line.startsWith("@@")) return "hunk";
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  return "ctx";
}

export function diffLineClass(kind: string, padding: string) {
  const tint = {
    add: "bg-[color-mix(in_srgb,var(--success)_13%,transparent)]",
    del: "bg-[color-mix(in_srgb,var(--danger)_13%,transparent)]",
    hunk: "bg-[color-mix(in_srgb,var(--muted)_8%,transparent)] text-faint",
    ctx: "",
  }[kind];
  return `block ${padding} break-anywhere whitespace-pre-wrap ${tint ?? ""}`;
}

export function diffLineStats(lines: DiffLine[]) {
  return {
    additions: lines.filter((line) => line.kind === "add").length,
    deletions: lines.filter((line) => line.kind === "del").length,
  };
}

function splitLines(text: string | null) {
  if (!text) return [];
  return (text.endsWith("\n") ? text.slice(0, -1) : text).split("\n");
}
