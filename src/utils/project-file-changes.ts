export function pathKey(path: string): string {
  const normalized = path.replaceAll("\\", "/").replace(/\/$/, "");
  return /^[a-z]:\//i.test(normalized) ? normalized.toLowerCase() : normalized;
}

export function changedParentDirectories(projectRoot: string, paths: string[]): string[] {
  const root = pathKey(projectRoot);
  return [
    ...new Set(
      paths.flatMap((path) => {
        const normalized = pathKey(path);
        if (normalized === root) return [root];
        const separator = normalized.lastIndexOf("/");
        const parent = separator < 0 ? root : normalized.slice(0, separator);
        return parent === root || parent.startsWith(`${root}/`) ? [parent] : [];
      }),
    ),
  ];
}
