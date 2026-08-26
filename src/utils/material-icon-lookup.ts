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

export type MaterialIconManifest = MaterialIconMaps & {
  iconDefinitions?: Record<string, { iconPath: string }>;
};

function iconBasename(
  definitions: Record<string, { iconPath: string }> | undefined,
  id: string | undefined,
  fallback: string,
) {
  if (!id) return fallback;
  const iconPath = definitions?.[id]?.iconPath;
  if (!iconPath) return id;
  const file = iconPath.slice(iconPath.lastIndexOf("/") + 1);
  return file.endsWith(".svg") ? file.slice(0, -4) : file || fallback;
}

function remap(
  definitions: Record<string, { iconPath: string }> | undefined,
  values: Record<string, string> | undefined,
) {
  const mappedValues: Record<string, string> = {};
  for (const [key, id] of Object.entries(values ?? {})) {
    mappedValues[key] = iconBasename(definitions, id, id);
  }
  return mappedValues;
}

export function materialIconMapsFromManifest(manifest: MaterialIconManifest): MaterialIconMaps {
  const definitions = manifest.iconDefinitions;
  return {
    file: iconBasename(definitions, manifest.file, "file"),
    folder: iconBasename(definitions, manifest.folder, "folder"),
    folderExpanded: iconBasename(definitions, manifest.folderExpanded, "folder-open"),
    rootFolder: iconBasename(definitions, manifest.rootFolder, "folder-root"),
    rootFolderExpanded: iconBasename(definitions, manifest.rootFolderExpanded, "folder-root-open"),
    fileNames: remap(definitions, manifest.fileNames),
    fileExtensions: remap(definitions, manifest.fileExtensions),
    folderNames: remap(definitions, manifest.folderNames),
    folderNamesExpanded: remap(definitions, manifest.folderNamesExpanded),
  };
}

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
