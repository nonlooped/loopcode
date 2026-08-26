export type MaterialIconMaps = {
  file?: string;
  folder?: string;
  folderExpanded?: string;
  rootFolder?: string;
  rootFolderExpanded?: string;
  fileNames?: Record<string, string>;
  fileExtensions?: Record<string, string>;
  folderNames?: Record<string, string>;
  folderNamesExpanded?: Record<string, string>;
};

function mapped(values: Record<string, string> | undefined, key: string) {
  if (!values || !Object.hasOwn(values, key)) return undefined;
  // SAFETY: Object.hasOwn established that key is present in values.
  return values[key];
}

export function materialFileIconName(maps: MaterialIconMaps, name: string) {
  const normalized = name.toLowerCase();
  const namedIcon = mapped(maps.fileNames, normalized);
  if (namedIcon) return namedIcon;

  let from = 0;
  while (from < normalized.length) {
    const dot = normalized.indexOf(".", from);
    if (dot === -1) break;
    const extensionIcon = mapped(maps.fileExtensions, normalized.slice(dot + 1));
    if (extensionIcon) return extensionIcon;
    from = dot + 1;
  }

  return maps.file ?? "file";
}

export function materialFolderIconName(
  maps: MaterialIconMaps,
  name: string,
  expanded: boolean,
  root = false,
) {
  if (root)
    return (
      (expanded ? maps.rootFolderExpanded : maps.rootFolder) ??
      (expanded ? "folder-root-open" : "folder-root")
    );
  return (
    mapped(expanded ? maps.folderNamesExpanded : maps.folderNames, name.toLowerCase()) ??
    (expanded ? maps.folderExpanded : maps.folder) ??
    (expanded ? "folder-open" : "folder")
  );
}
