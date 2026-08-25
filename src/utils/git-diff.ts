import type { GitFileDiff } from "../services/native.ts";

export type GitDiffLineKind = "add" | "del" | "hunk" | "ctx";

export function gitDiffLines(diff: GitFileDiff) {
  const text = diff.hunks.join("");
  const raw = text.endsWith("\n") ? text.slice(0, -1) : text;
  if (raw === "") return [];
  return raw.split("\n").map((line) => ({
    kind: gitDiffLineKind(line),
    text: line,
  }));
}

function gitDiffLineKind(line: string): GitDiffLineKind {
  if (line.startsWith("@@")) return "hunk";
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  return "ctx";
}
