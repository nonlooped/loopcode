import assert from "node:assert/strict";
import test from "node:test";

import { generateManifest } from "material-icon-theme";

import {
  materialFileIconName,
  materialFolderIconName,
  materialIconMapsFromManifest,
} from "../src/utils/material-icon-lookup.ts";

const maps = materialIconMapsFromManifest(generateManifest());

void test("uses the material-icon-theme manifest for filenames and nested extensions", () => {
  assert.equal(materialFileIconName(maps, "package.json"), "nodejs");
  assert.equal(materialFileIconName(maps, "README.md"), "readme");
  assert.equal(materialFileIconName(maps, "foo.d.ts"), "typescript-def");
  assert.equal(materialFileIconName(maps, "app.tsx"), "react_ts");
  assert.equal(materialFileIconName(maps, "untitled"), maps.file);
});

void test("resolves clone iconPaths to SVG basenames", () => {
  const cloneMaps = materialIconMapsFromManifest({
    file: "file",
    iconDefinitions: {
      file: { iconPath: "./../icons/file.svg" },
      "angular-component": { iconPath: "./../icons/angular-component.clone.svg" },
    },
    fileExtensions: { "component.ts": "angular-component" },
  });
  assert.equal(materialFileIconName(cloneMaps, "app.component.ts"), "angular-component.clone");
});

void test("uses the material-icon-theme manifest for folders", () => {
  assert.equal(materialFolderIconName(maps, "src", false), "folder-src");
  assert.equal(materialFolderIconName(maps, "src", true), "folder-src-open");
  assert.equal(materialFolderIconName(maps, "not-a-known-folder", false), maps.folder);
  assert.equal(materialFolderIconName(maps, "project", false, true), maps.rootFolder);
  assert.equal(materialFolderIconName(maps, "project", true, true), maps.rootFolderExpanded);
});
