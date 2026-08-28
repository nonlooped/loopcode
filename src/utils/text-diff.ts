export type DiffLineKind = "add" | "del" | "hunk" | "ctx";

export interface DiffLine {
  kind: DiffLineKind;
  text: string;
}

/** Beyond this many changed lines on a side, the quadratic pass is skipped for a block replace. */
const MAX_LCS_LINES = 1_200;
const DEFAULT_CONTEXT = 3;

type Op = { kind: "add" | "del" | "ctx"; text: string };

export function unifiedDiffLines(
  oldText: string | null,
  newText: string | null,
  context = DEFAULT_CONTEXT,
): DiffLine[] {
  const before = splitLines(oldText);
  const after = splitLines(newText);
  if (before.length === 0 && after.length === 0) return [];
  return toHunks(diffOps(before, after), context);
}

/**
 * Codex sends a unified diff for edits but whole file contents for adds and deletes, so each
 * shape has to be turned into display lines differently.
 */
export function fileChangeDiffLines(diff: string, kind: "add" | "update" | "delete"): DiffLine[] {
  const lines = splitLines(diff);
  if (kind === "update") return lines.map((text) => ({ kind: diffLineKind(text), text }));
  const marker = kind === "add" ? "+" : "-";
  return lines.map((text) => ({ kind: kind === "add" ? "add" : "del", text: `${marker}${text}` }));
}

function diffLineKind(line: string): DiffLineKind {
  if (line.startsWith("@@")) return "hunk";
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  return "ctx";
}

export function diffLineStats(lines: DiffLine[]) {
  let additions = 0;
  let deletions = 0;
  for (const line of lines) {
    if (line.kind === "add") additions += 1;
    else if (line.kind === "del") deletions += 1;
  }
  return { additions, deletions };
}

function splitLines(text: string | null) {
  if (!text) return [];
  const trimmed = text.endsWith("\n") ? text.slice(0, -1) : text;
  return trimmed.split("\n");
}

function diffOps(before: string[], after: string[]): Op[] {
  let head = 0;
  while (head < before.length && head < after.length && before[head] === after[head]) head += 1;
  let tail = 0;
  while (
    tail < before.length - head &&
    tail < after.length - head &&
    before[before.length - 1 - tail] === after[after.length - 1 - tail]
  ) {
    tail += 1;
  }

  const middleBefore = before.slice(head, before.length - tail);
  const middleAfter = after.slice(head, after.length - tail);
  return [
    ...before.slice(0, head).map((text): Op => ({ kind: "ctx", text })),
    ...middleOps(middleBefore, middleAfter),
    ...before.slice(before.length - tail).map((text): Op => ({ kind: "ctx", text })),
  ];
}

function middleOps(before: string[], after: string[]): Op[] {
  if (before.length === 0) return after.map((text) => ({ kind: "add", text }));
  if (after.length === 0) return before.map((text) => ({ kind: "del", text }));
  if (before.length > MAX_LCS_LINES || after.length > MAX_LCS_LINES) {
    return [
      ...before.map((text): Op => ({ kind: "del", text })),
      ...after.map((text): Op => ({ kind: "add", text })),
    ];
  }
  return backtrack(lcsLengths(before, after), before, after);
}

/** Row `i` holds the LCS length for `before[i..]` against each suffix of `after`. */
function lcsLengths(before: string[], after: string[]) {
  const width = after.length + 1;
  const table = new Uint32Array((before.length + 1) * width);
  for (let i = before.length - 1; i >= 0; i -= 1) {
    for (let j = after.length - 1; j >= 0; j -= 1) {
      table[i * width + j] =
        before[i] === after[j]
          ? table[(i + 1) * width + j + 1] + 1
          : Math.max(table[(i + 1) * width + j], table[i * width + j + 1]);
    }
  }
  return table;
}

function backtrack(table: Uint32Array, before: string[], after: string[]): Op[] {
  const width = after.length + 1;
  const ops: Op[] = [];
  let i = 0;
  let j = 0;
  while (i < before.length && j < after.length) {
    if (before[i] === after[j]) {
      ops.push({ kind: "ctx", text: before[i] });
      i += 1;
      j += 1;
    } else if (table[(i + 1) * width + j] >= table[i * width + j + 1]) {
      ops.push({ kind: "del", text: before[i] });
      i += 1;
    } else {
      ops.push({ kind: "add", text: after[j] });
      j += 1;
    }
  }
  while (i < before.length) ops.push({ kind: "del", text: before[i++] });
  while (j < after.length) ops.push({ kind: "add", text: after[j++] });
  return ops;
}

/** Keeps `context` unchanged lines around each edit and drops the untouched stretches between. */
function keptLines(ops: Op[], context: number) {
  const keep: boolean[] = Array.from({ length: ops.length }, () => false);
  let changed = false;
  for (const [index, op] of ops.entries()) {
    if (op.kind === "ctx") continue;
    changed = true;
    const from = Math.max(0, index - context);
    const to = Math.min(ops.length - 1, index + context);
    for (let near = from; near <= to; near += 1) keep[near] = true;
  }
  return changed ? keep : undefined;
}

function toHunks(ops: Op[], context: number): DiffLine[] {
  const keep = keptLines(ops, context);
  if (!keep) return [];

  const lines: DiffLine[] = [];
  let oldLine = 1;
  let newLine = 1;
  let index = 0;
  while (index < ops.length) {
    if (!keep[index]) {
      if (ops[index].kind !== "add") oldLine += 1;
      if (ops[index].kind !== "del") newLine += 1;
      index += 1;
      continue;
    }
    const start = index;
    const oldStart = oldLine;
    const newStart = newLine;
    let oldCount = 0;
    let newCount = 0;
    while (index < ops.length && keep[index]) {
      if (ops[index].kind !== "add") {
        oldLine += 1;
        oldCount += 1;
      }
      if (ops[index].kind !== "del") {
        newLine += 1;
        newCount += 1;
      }
      index += 1;
    }
    lines.push({
      kind: "hunk",
      text: `@@ -${oldStart},${oldCount} +${newStart},${newCount} @@`,
    });
    for (const op of ops.slice(start, index)) lines.push({ kind: op.kind, text: prefix(op) });
  }
  return lines;
}

function prefix(op: Op) {
  if (op.kind === "add") return `+${op.text}`;
  if (op.kind === "del") return `-${op.text}`;
  return ` ${op.text}`;
}
