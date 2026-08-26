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
  const controls = readFileSync(
    new URL("../src/styles/window-controls.css", import.meta.url),
    "utf8",
  );

  assert.match(
    titlebar,
    /{:else}\s*{@render minimizeControl\(\)}\s*{@render maximizeControl\(\)}\s*{@render closeControl\(\)}/,
  );
  assert.doesNotMatch(controls, /^\s*order:/m);
});

void test("non-macOS panels leave titlebar controls usable", () => {
  const controls = readFileSync(
    new URL("../src/styles/window-controls.css", import.meta.url),
    "utf8",
  );

  assert.doesNotMatch(controls, /\.thread-sidebar \{\s*top: 0;/);
  assert.match(controls, /\.app-shell:not\(\.sidebar-collapsed\):has\(\.sidebar-heading\)/);
  assert.match(
    controls,
    /\.project-explorer-actions \{\s*right: calc\(var\(--window-controls-width\) \+ 6px\);/,
  );
  assert.match(
    controls,
    /\.project-explorer-tab-list \{\s*top: calc\(var\(--titlebar-height\) \+ 7px\);/,
  );
});
