import { diffArrays } from "diff";
import type { TimelineItem, TimelineItemKind } from "../../ipc/types";
import { pathBasename } from "../../lib/path";

export type DiffLineKind = "add" | "del" | "ctx";

export type DiffLine = {
  kind: DiffLineKind;
  text: string;
};

export type ToolCallStatus = "using" | "used" | "skipped";

/** One paired tool call for the expandable timeline list. */
export type ToolCallRow = {
  id: string;
  callId: string;
  name: string;
  status: ToolCallStatus;
  /** Secondary label: basename, URL, command snippet, etc. */
  detail: string | null;
  /** Full secondary label for tooltip (path, full URL, full command). */
  detailFull: string | null;
  added: number | null;
  removed: number | null;
  /** Mini hunk for edit / patch / write; null when not expandable. */
  hunk: DiffLine[] | null;
  expandable: boolean;
  verb: string;
};

type ToolCounts = { using: number; used: number; skipped: number };

type ToolCopy = {
  progress: string;
  one: string;
  many: (n: number) => string;
  skippedOne: string;
  skippedMany: (n: number) => string;
};

const TOOL_COPY: Record<string, ToolCopy> = {
  glob: {
    progress: "Listing files…",
    one: "Listed files",
    many: (n) => `Listed files ${n} times`,
    skippedOne: "Skipped listing files",
    skippedMany: (n) => `Skipped listing files ${n} times`,
  },
  read: {
    progress: "Reading files…",
    one: "Read a file",
    many: (n) => `Read ${n} files`,
    skippedOne: "Skipped reading a file",
    skippedMany: (n) => `Skipped reading ${n} files`,
  },
  grep: {
    progress: "Searching…",
    one: "Searched the project",
    many: (n) => `Searched ${n} times`,
    skippedOne: "Skipped a search",
    skippedMany: (n) => `Skipped ${n} searches`,
  },
  write: {
    progress: "Writing files…",
    one: "Wrote a file",
    many: (n) => `Wrote ${n} files`,
    skippedOne: "Skipped writing a file",
    skippedMany: (n) => `Skipped writing ${n} files`,
  },
  edit: {
    progress: "Editing files…",
    one: "Edited a file",
    many: (n) => `Edited ${n} files`,
    skippedOne: "Skipped editing a file",
    skippedMany: (n) => `Skipped editing ${n} files`,
  },
  patch: {
    progress: "Editing files…",
    one: "Edited a file",
    many: (n) => `Edited ${n} files`,
    skippedOne: "Skipped editing a file",
    skippedMany: (n) => `Skipped editing ${n} files`,
  },
  exec: {
    progress: "Running a command…",
    one: "Ran a command",
    many: (n) => `Ran ${n} commands`,
    skippedOne: "Skipped a command",
    skippedMany: (n) => `Skipped ${n} commands`,
  },
  webfetch: {
    progress: "Fetching…",
    one: "Fetched a URL",
    many: (n) => `Fetched ${n} URLs`,
    skippedOne: "Skipped a fetch",
    skippedMany: (n) => `Skipped ${n} fetches`,
  },
};

const EXPANDABLE_TOOLS = new Set(["edit", "patch", "write"]);

const VERBS: Record<string, { done: string; progress: string; skipped: string }> = {
  edit: { done: "Edited", progress: "Editing", skipped: "Skipped edit" },
  patch: { done: "Edited", progress: "Editing", skipped: "Skipped edit" },
  write: { done: "Wrote", progress: "Writing", skipped: "Skipped write" },
  read: { done: "Read", progress: "Reading", skipped: "Skipped read" },
  grep: { done: "Searched", progress: "Searching", skipped: "Skipped search" },
  glob: { done: "Listed", progress: "Listing", skipped: "Skipped list" },
  exec: { done: "Ran", progress: "Running", skipped: "Skipped" },
  webfetch: { done: "Fetched", progress: "Fetching", skipped: "Skipped fetch" },
  skill: { done: "Loaded skill", progress: "Loading skill", skipped: "Skipped skill" },
};

function fallbackCopy(name: string): ToolCopy {
  return {
    progress: `Using ${name}…`,
    one: `Used ${name}`,
    many: (n) => `Used ${name} ${n} times`,
    skippedOne: `Skipped ${name}`,
    skippedMany: (n) => `Skipped ${name} ${n} times`,
  };
}

export function toolNameFromTitle(item: TimelineItem): string {
  const t = item.title ?? "";
  const stripped = t
    .replace(/^Tool result:\s*/i, "")
    .replace(/^Tool:\s*/i, "")
    .replace(/^Rejected:\s*/i, "")
    .replace(/^Approve:\s*/i, "")
    .trim();
  return stripped || "tool";
}

/** True when a tool was rejected solely by Ask/Plan/Debug mode policy (not approval). */
export function isModePolicyMutatingDenial(item: TimelineItem): boolean {
  if (item.kind !== "tool_rejected") return false;
  const hay = [
    item.body ?? "",
    item.title ?? "",
    typeof item.payload === "string" ? item.payload : JSON.stringify(item.payload ?? ""),
  ]
    .join("\n")
    .toLowerCase();
  return (
    hay.includes("mode_policy") ||
    hay.includes("mode policy") ||
    hay.includes("does not allow") ||
    hay.includes("mode_policy_denied")
  );
}

/** Collapse proposal/result pairs and count by tool name. */
export function countTools(tools: TimelineItem[]): Map<string, ToolCounts> {
  const counts = new Map<string, ToolCounts>();
  for (const item of tools) {
    const name = item.toolName?.trim() || toolNameFromTitle(item);
    const entry = counts.get(name) ?? { using: 0, used: 0, skipped: 0 };
    const kind = item.kind as TimelineItemKind | string;
    if (kind === "tool_proposal") {
      entry.using += 1;
    } else if (kind === "tool_rejected") {
      if (entry.using > 0) entry.using -= 1;
      entry.skipped += 1;
    } else {
      if (entry.using > 0) entry.using -= 1;
      entry.used += 1;
    }
    counts.set(name, entry);
  }
  return counts;
}

/** One quiet line: "Read 5 files · Listed files 2 times". */
export function summarizeTools(tools: TimelineItem[]): string | null {
  if (tools.length === 0) return null;
  const parts: string[] = [];
  for (const [name, c] of countTools(tools)) {
    const copy = TOOL_COPY[name] ?? fallbackCopy(name);
    if (c.using > 0) {
      parts.push(copy.progress);
      continue;
    }
    if (c.used > 0) {
      parts.push(c.used === 1 ? copy.one : copy.many(c.used));
    }
    if (c.skipped > 0) {
      parts.push(c.skipped === 1 ? copy.skippedOne : copy.skippedMany(c.skipped));
    }
  }
  return parts.length > 0 ? parts.join(" · ") : null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

function parseJsonObject(raw: string): Record<string, unknown> | null {
  if (!raw.trim()) return null;
  try {
    return asRecord(JSON.parse(raw));
  } catch {
    return null;
  }
}

function fieldString(obj: Record<string, unknown> | null, ...keys: string[]): string | null {
  if (!obj) return null;
  for (const key of keys) {
    const value = obj[key];
    if (typeof value === "string" && value.length > 0) return value;
  }
  return null;
}

function payloadRecord(item: TimelineItem): Record<string, unknown> | null {
  return asRecord(item.payload) ?? parseJsonObject(item.body);
}

function callIdOf(item: TimelineItem): string {
  const payload = payloadRecord(item);
  return (
    fieldString(payload, "callId", "call_id") ??
    fieldString(asRecord(payload?.content), "callId", "call_id") ??
    item.id
  );
}

function argumentsOf(item: TimelineItem): Record<string, unknown> | null {
  const payload = payloadRecord(item);
  if (!payload) return parseJsonObject(item.body);
  const raw = payload.arguments;
  const fromPayload = asRecord(raw) ?? (typeof raw === "string" ? parseJsonObject(raw) : null);
  if (fromPayload) return fromPayload;
  // tool.proposal body is stringified arguments
  if (item.kind === "tool_proposal") return parseJsonObject(item.body);
  return null;
}

function resultContentOf(item: TimelineItem): Record<string, unknown> | null {
  const payload = payloadRecord(item);
  if (!payload) return parseJsonObject(item.body);
  const content = asRecord(payload.content);
  if (content) return content;
  if (item.kind === "tool_result") return parseJsonObject(item.body);
  return null;
}

function pathFromArgs(args: Record<string, unknown> | null): string | null {
  return fieldString(args, "path", "filePath", "file_path");
}

function truncateMiddle(text: string, max: number): string {
  if (text.length <= max) return text;
  if (max < 8) return text.slice(0, max);
  const head = Math.ceil((max - 1) / 2);
  const tail = Math.floor((max - 1) / 2);
  return `${text.slice(0, head)}…${text.slice(text.length - tail)}`;
}

/** Secondary label beside the verb (file, URL, command, …). */
function detailForTool(
  name: string,
  args: Record<string, unknown> | null,
  result: Record<string, unknown> | null,
): { detail: string | null; detailFull: string | null } {
  if (name === "webfetch") {
    const url = fieldString(args, "url") ?? fieldString(result, "url");
    if (!url) return { detail: null, detailFull: null };
    return { detail: truncateMiddle(url, 64), detailFull: url };
  }
  if (name === "exec") {
    const command = fieldString(args, "command");
    if (!command) return { detail: null, detailFull: null };
    return { detail: truncateMiddle(command, 56), detailFull: command };
  }
  if (name === "grep") {
    const pattern = fieldString(args, "pattern", "query", "regex");
    if (!pattern) return { detail: null, detailFull: null };
    return { detail: truncateMiddle(pattern, 48), detailFull: pattern };
  }
  if (name === "glob") {
    const pattern = fieldString(args, "pattern", "glob", "query");
    if (!pattern) return { detail: null, detailFull: null };
    return { detail: truncateMiddle(pattern, 48), detailFull: pattern };
  }

  const pathFull =
    pathFromArgs(args) ?? fieldString(result, "path", "filePath", "file_path");
  if (!pathFull) return { detail: null, detailFull: null };
  return { detail: pathBasename(pathFull), detailFull: pathFull };
}

/** Split into lines without a trailing empty segment from a final newline. */
export function splitLines(text: string): string[] {
  if (text.length === 0) return [];
  const normalized = text.replace(/\r\n/g, "\n");
  const parts = normalized.split("\n");
  if (parts.length > 0 && parts[parts.length - 1] === "") parts.pop();
  return parts;
}

/** Line-oriented diff for a small replacement hunk (add/del only, no context). */
export function diffHunk(oldText: string, newText: string): {
  lines: DiffLine[];
  added: number;
  removed: number;
} {
  const a = splitLines(oldText);
  const b = splitLines(newText);
  const parts = diffArrays(a, b);
  const lines: DiffLine[] = [];
  let added = 0;
  let removed = 0;
  for (const part of parts) {
    if (part.added) {
      for (const text of part.value) {
        lines.push({ kind: "add", text });
        added += 1;
      }
    } else if (part.removed) {
      for (const text of part.value) {
        lines.push({ kind: "del", text });
        removed += 1;
      }
    }
  }
  return { lines, added, removed };
}

const HUNK_LINE_CAP = 120;

function truncateHunk(lines: DiffLine[]): DiffLine[] {
  if (lines.length <= HUNK_LINE_CAP) return lines;
  const kept = lines.slice(0, HUNK_LINE_CAP);
  kept.push({
    kind: "ctx",
    text: `… ${lines.length - HUNK_LINE_CAP} more lines`,
  });
  return kept;
}

function hunkForTool(
  name: string,
  args: Record<string, unknown> | null,
): { hunk: DiffLine[]; added: number; removed: number } | null {
  if (!args || !EXPANDABLE_TOOLS.has(name)) return null;

  if (name === "edit") {
    const oldText = fieldString(args, "oldString", "old_string") ?? "";
    const newText = fieldString(args, "newString", "new_string") ?? "";
    // Empty oldString = create; treat as all additions.
    if (oldText.length === 0 && newText.length > 0) {
      const lines = splitLines(newText).map((text) => ({ kind: "add" as const, text }));
      return {
        hunk: truncateHunk(lines),
        added: lines.length,
        removed: 0,
      };
    }
    const diff = diffHunk(oldText, newText);
    return { hunk: truncateHunk(diff.lines), added: diff.added, removed: diff.removed };
  }

  // write / patch — only new content is in the proposal; show as additions.
  const content =
    fieldString(args, "content", "body", "newContent", "new_content") ?? "";
  if (!content) return { hunk: [], added: 0, removed: 0 };
  const lines = splitLines(content).map((text) => ({ kind: "add" as const, text }));
  return {
    hunk: truncateHunk(lines),
    added: lines.length,
    removed: 0,
  };
}

function verbFor(name: string, status: ToolCallStatus): string {
  const entry = VERBS[name];
  if (!entry) {
    if (status === "using") return "Using";
    if (status === "skipped") return "Skipped";
    return "Used";
  }
  if (status === "using") return entry.progress;
  if (status === "skipped") return entry.skipped;
  return entry.done;
}

/**
 * Pair proposal/result/rejected items into stable rows (order = first sighting).
 */
export function buildToolCallRows(tools: TimelineItem[]): ToolCallRow[] {
  type Acc = {
    id: string;
    callId: string;
    name: string;
    status: ToolCallStatus;
    args: Record<string, unknown> | null;
    result: Record<string, unknown> | null;
  };

  const order: string[] = [];
  const map = new Map<string, Acc>();

  for (const item of tools) {
    const name = item.toolName?.trim() || toolNameFromTitle(item);
    const callId = callIdOf(item);
    const kind = item.kind as TimelineItemKind | string;
    let acc = map.get(callId);
    if (!acc) {
      acc = {
        id: item.id,
        callId,
        name,
        status: "using",
        args: null,
        result: null,
      };
      map.set(callId, acc);
      order.push(callId);
    }
    if (kind === "tool_proposal") {
      acc.args = argumentsOf(item) ?? acc.args;
      acc.status = acc.status === "used" || acc.status === "skipped" ? acc.status : "using";
      acc.name = name;
    } else if (kind === "tool_rejected") {
      acc.status = "skipped";
      acc.args = argumentsOf(item) ?? acc.args;
    } else {
      // tool_result
      acc.status = "used";
      acc.result = resultContentOf(item) ?? acc.result;
      acc.args = argumentsOf(item) ?? acc.args;
    }
  }

  return order.map((callId) => {
    const acc = map.get(callId)!;
    const { detail, detailFull } = detailForTool(acc.name, acc.args, acc.result);
    const hunkInfo = hunkForTool(acc.name, acc.args);
    const expandable =
      EXPANDABLE_TOOLS.has(acc.name) &&
      acc.status === "used" &&
      hunkInfo != null &&
      hunkInfo.hunk.length > 0;

    return {
      id: acc.id,
      callId,
      name: acc.name,
      status: acc.status,
      detail,
      detailFull,
      added: hunkInfo ? hunkInfo.added : null,
      removed: hunkInfo ? hunkInfo.removed : null,
      hunk: expandable ? hunkInfo!.hunk : null,
      expandable,
      verb: verbFor(acc.name, acc.status),
    };
  });
}
