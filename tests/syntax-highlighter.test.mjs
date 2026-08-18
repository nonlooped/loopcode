import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  fallbackHighlight,
  highlightCode,
  languageForPath,
  normalizeLanguage,
} from "../src/utils/syntax-highlighter.ts";

void test("plain fallback escapes content without initializing a grammar", () => {
  assert.equal(languageForPath("notes.txt"), "");
  assert.equal(normalizeLanguage("unknown-agent-language"), "");
  assert.match(fallbackHighlight("<script>&\"'"), /&lt;script&gt;&amp;&quot;&#39;/);
});

void test("concurrent languages share one lazy Shiki core creation path", async () => {
  const [javascript, javascriptAlias, python, source] = await Promise.all([
    highlightCode("const answer = 42", "javascript"),
    highlightCode("let answer = 42", "js"),
    highlightCode("answer = 42", "python"),
    readFile(new URL("../src/utils/syntax-highlighter.ts", import.meta.url), "utf8"),
  ]);

  assert.match(javascript, /class="shiki github-dark"/);
  assert.match(javascriptAlias, /class="shiki github-dark"/);
  assert.match(python, /class="shiki github-dark"/);
  assert.equal(source.match(/createHighlighterCore\(/g)?.length, 1);
  assert.match(source, /let highlighterLoad: Promise<HighlighterCore> \| undefined/);
});
