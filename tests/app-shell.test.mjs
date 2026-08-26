import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

void test("compact drawer Escape takes priority over question dismissal", () => {
  const appShell = readFileSync(new URL("../src/AppShell.svelte", import.meta.url), "utf8");

  assert.match(appShell, /event\.stopImmediatePropagation\(\)/);
  assert.match(appShell, /<svelte:window onkeydowncapture=/);
});
