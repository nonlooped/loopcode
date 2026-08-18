import assert from "node:assert/strict";
import test from "node:test";

import { contextMenuPosition } from "../src/utils/context-menu.ts";

void test("context menus stay inside the viewport", () => {
  assert.deepEqual(contextMenuPosition(790, 590, 200, 160, 800, 600), { x: 592, y: 432 });
  assert.deepEqual(contextMenuPosition(-10, -20, 200, 160, 800, 600), { x: 8, y: 8 });
});
