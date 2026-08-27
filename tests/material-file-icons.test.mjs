import assert from "node:assert/strict";
import test from "node:test";

import { generateManifest } from "material-icon-theme";

import {
  materialFileIconName,
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
