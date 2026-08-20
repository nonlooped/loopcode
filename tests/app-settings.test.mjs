import assert from "node:assert/strict";
import test from "node:test";

import {
  loadPermissionMode,
  loadSidebarWidths,
  loadTerminalHeight,
  savePermissionMode,
  saveSidebarWidth,
  saveTerminalHeight,
} from "../src/utils/app-settings.ts";

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

void test("sidebar widths retain CSS defaults until resized and persist valid sizes", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  assert.deepEqual(loadSidebarWidths(storage), { left: null, right: null });
  saveSidebarWidth("left", 318, storage);
  saveSidebarWidth("right", 364, storage);
  assert.deepEqual(loadSidebarWidths(storage), { left: 318, right: 364 });

  values.set("loopcode.left-sidebar-width", "20");
  values.set("loopcode.right-sidebar-width", "2000");
  assert.deepEqual(loadSidebarWidths(storage), { left: 190, right: 520 });
});

void test("terminal height has a useful default and clamps persisted values", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  assert.equal(loadTerminalHeight(storage), 260);
  saveTerminalHeight(340, storage);
  assert.equal(loadTerminalHeight(storage), 340);

  values.set("loopcode.terminal-height", "900");
  assert.equal(loadTerminalHeight(storage), 520);
  values.set("loopcode.terminal-height", "invalid");
  assert.equal(loadTerminalHeight(storage), 260);
});
