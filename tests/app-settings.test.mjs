import assert from "node:assert/strict";
import test from "node:test";

import { loadPermissionMode, savePermissionMode } from "../src/utils/app-settings.ts";

void test("permission mode defaults to restricted and persists valid choices", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  assert.equal(loadPermissionMode(storage), "restricted");
  savePermissionMode("full", storage);
  assert.equal(loadPermissionMode(storage), "full");
  values.set("loopcode.permission-mode", "unexpected");
  assert.equal(loadPermissionMode(storage), "restricted");
});
