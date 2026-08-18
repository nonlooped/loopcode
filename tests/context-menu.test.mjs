import assert from "node:assert/strict";
import test from "node:test";

import { contextMenuPosition, nextMenuItemIndex } from "../src/utils/context-menu.ts";

void test("context menus stay inside the viewport", () => {
  assert.deepEqual(contextMenuPosition(790, 590, 200, 160, 800, 600), { x: 592, y: 432 });
  assert.deepEqual(contextMenuPosition(-10, -20, 200, 160, 800, 600), { x: 8, y: 8 });
});

void test("menu navigation wraps and supports boundaries", () => {
  assert.equal(nextMenuItemIndex(2, 3, "ArrowDown"), 0);
  assert.equal(nextMenuItemIndex(0, 3, "ArrowUp"), 2);
  assert.equal(nextMenuItemIndex(1, 3, "Home"), 0);
  assert.equal(nextMenuItemIndex(1, 3, "End"), 2);
  assert.equal(nextMenuItemIndex(0, 0, "ArrowDown"), -1);
});
