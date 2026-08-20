import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const [terminalStyles, terminalPane] = await Promise.all([
  readFile("src/styles/terminal.css", "utf8"),
  readFile("src/components/TerminalPane.svelte", "utf8"),
]);

void test("terminal uses the shell background instead of an opaque surface", () => {
  assert.match(terminalStyles, /\.terminal-drawer \{[\s\S]*?background: transparent;/);
  assert.match(
    terminalStyles,
    /\.terminal-host \.xterm-viewport \{[\s\S]*?background-color: transparent !important;/,
  );
  assert.match(terminalPane, /allowTransparency: true/);
  assert.match(terminalPane, /background: '#00000000'/);
});
