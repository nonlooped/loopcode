import assert from "node:assert/strict";
import test from "node:test";

import {
  buildThreadTitlePrompt,
  newThreadTitle,
  normalizeThreadTitle,
} from "../src/utils/thread-title.ts";

void test("temporary thread titles do not include timestamps", () => {
  assert.equal(newThreadTitle(), "New Thread");
});

void test("title prompts are concise and bounded", () => {
  const prompt = buildThreadTitlePrompt("x".repeat(2_000));
  assert.ok(prompt.length < 1_000);
  assert.match(prompt, /^Generate a 3–6 word title/);
  assert.doesNotMatch(prompt, /x{801}/);
});

void test("generated titles are normalized for the thread list", () => {
  assert.equal(
    normalizeThreadTitle('  **Title:** "Persist Local Threads."  '),
    "Persist Local Threads",
  );
  assert.equal(
    normalizeThreadTitle("# Add Markdown Rendering\nExtra explanation"),
    "Add Markdown Rendering",
  );
  assert.equal(normalizeThreadTitle("``"), undefined);
});
