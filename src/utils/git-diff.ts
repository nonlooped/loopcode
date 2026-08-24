import type { GitChange, GitFileDiff } from "../services/native.ts";

export function gitDiffViewData(change: GitChange, diff: GitFileDiff) {
  const oldFileName =
    change.status === "added" || change.status === "untracked"
      ? "/dev/null"
      : (change.oldPath ?? change.path);
  const newFileName = change.status === "deleted" ? "/dev/null" : change.path;
  return {
    oldFile: { fileName: oldFileName },
    newFile: { fileName: newFileName },
    hunks:
      diff.hunks.length === 0
        ? []
        : [`--- ${oldFileName}\n+++ ${newFileName}\n${diff.hunks.join("")}`],
  };
}
