/** Last path segment of a filesystem path (e.g. `C:/Projects/loopcode` → `loopcode`). */
export function projectDirName(path: string | null | undefined): string {
  if (!path) return "";
  return pathBasename(path) || path;
}

/** Basename of a path, normalizing `\` and trailing separators. */
export function pathBasename(path: string): string {
  const normalized = path.replace(/[\\/]+$/, "").replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts[parts.length - 1] || path;
}
