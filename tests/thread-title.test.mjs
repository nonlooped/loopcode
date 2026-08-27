import assert from "node:assert/strict";
import test from "node:test";

import { normalizeThreadTitle } from "../src/utils/thread-title.ts";

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
