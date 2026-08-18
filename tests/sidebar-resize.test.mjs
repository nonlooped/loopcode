import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

void test("resized sidebar values drive the active layout widths", async () => {
  const css = await readFile(new URL("../src/styles/shell.css", import.meta.url), "utf8");
  const appShell = css.match(/\.app-shell\s*\{([^}]*)\}/)?.[1] ?? "";

  assert.match(appShell, /--sidebar-width:\s*var\(--sidebar-expanded-width\)/);
  assert.match(appShell, /--project-explorer-width:\s*var\(--project-explorer-expanded-width\)/);
});
