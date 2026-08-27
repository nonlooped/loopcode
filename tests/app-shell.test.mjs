import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

void test("compact drawer Escape takes priority over question dismissal", () => {
  const appShell = readFileSync(new URL("../src/AppShell.svelte", import.meta.url), "utf8");

  assert.match(appShell, /event\.stopImmediatePropagation\(\)/);
  assert.match(appShell, /<svelte:window onkeydowncapture=/);
});

void test("non-macOS window controls match their keyboard order", () => {
  const titlebar = readFileSync(
    new URL("../src/components/Titlebar.svelte", import.meta.url),
    "utf8",
  );

  assert.match(
    titlebar,
    /{:else}\s*{@render minimizeControl\(\)}\s*{@render maximizeControl\(\)}\s*{@render closeControl\(\)}/,
  );
  assert.doesNotMatch(titlebar, /\border:/);
});

void test("titlebar is utility-styled without a platform stylesheet", () => {
  const titlebar = readFileSync(
    new URL("../src/components/Titlebar.svelte", import.meta.url),
    "utf8",
  );

  assert.match(titlebar, /grid-cols-\[var\(--sidebar-width\)_minmax\(0,1fr\)\]/);
  assert.match(titlebar, /bg-\[rgba\(255,95,87,\.92\)\]/);
});
