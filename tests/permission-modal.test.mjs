import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

void test("permission prompts focus their preferred action", () => {
  const modal = readFileSync(
    new URL("../src/components/PermissionModal.svelte", import.meta.url),
    "utf8",
  );

  assert.match(modal, /function focusPreferredOption/);
  assert.match(modal, /onOpenAutoFocus=\{focusPreferredOption\}/);
});
