import manifest from "virtual:material-icon-manifest";

import { materialFileIconName, materialFolderIconName } from "./material-icon-lookup";

const iconAssets = import.meta.glob<string>("../../node_modules/material-icon-theme/icons/*.svg", {
  eager: true,
  import: "default",
  query: "?url&no-inline",
});

function iconUrl(iconName: string) {
  const fallback = iconAssets["../../node_modules/material-icon-theme/icons/file.svg"];
  if (!fallback) throw new Error("Material file icon asset is missing");
  return iconAssets[`../../node_modules/material-icon-theme/icons/${iconName}.svg`] ?? fallback;
}

export function materialFileIcon(name: string) {
  return iconUrl(materialFileIconName(manifest, name));
}

export function materialFolderIcon(name: string, expanded: boolean, root = false) {
  return iconUrl(materialFolderIconName(manifest, name, expanded, root));
}
