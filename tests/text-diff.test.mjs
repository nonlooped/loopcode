import assert from "node:assert/strict";
import test from "node:test";

import { diffLineStats, unifiedDiffLines } from "../src/utils/text-diff.ts";

const text = (...lines) => `${lines.join("\n")}\n`;

void test("an unchanged file produces no hunks", () => {
  assert.deepEqual(unifiedDiffLines(text("a", "b"), text("a", "b")), []);
});

void test("a new file marks every line as an addition", () => {
  const lines = unifiedDiffLines(null, text("one", "two"));

  assert.deepEqual(
    lines.map((line) => line.text),
    ["@@ -1,0 +1,2 @@", "+one", "+two"],
  );
  assert.deepEqual(diffLineStats(lines), { additions: 2, deletions: 0 });
});

void test("a deleted file marks every line as a removal", () => {
  const lines = unifiedDiffLines(text("one", "two"), "");

  assert.deepEqual(
    lines.map((line) => line.kind),
    ["hunk", "del", "del"],
  );
  assert.deepEqual(diffLineStats(lines), { additions: 0, deletions: 2 });
});

void test("an edit keeps surrounding context and numbers the hunk", () => {
  const before = text("a", "b", "c", "d", "e");
  const after = text("a", "b", "C", "d", "e");
  const lines = unifiedDiffLines(before, after);

  assert.deepEqual(
    lines.map((line) => line.text),
    ["@@ -1,5 +1,5 @@", " a", " b", "-c", "+C", " d", " e"],
  );
  assert.deepEqual(diffLineStats(lines), { additions: 1, deletions: 1 });
});

void test("distant edits split into separate hunks and drop the untouched middle", () => {
  const before = text(...Array.from({ length: 40 }, (_, index) => `line ${index}`));
  const after = before.replace("line 0", "first").replace("line 39", "last");
  const lines = unifiedDiffLines(before, after);

  assert.deepEqual(
    lines.filter((line) => line.kind === "hunk").map((line) => line.text),
    ["@@ -1,4 +1,4 @@", "@@ -37,4 +37,4 @@"],
  );
  assert.equal(
    lines.some((line) => line.text.includes("line 20")),
    false,
  );
  assert.deepEqual(diffLineStats(lines), { additions: 2, deletions: 2 });
});

void test("pure insertions do not rewrite the surrounding lines", () => {
  const lines = unifiedDiffLines(text("a", "d"), text("a", "b", "c", "d"));

  assert.deepEqual(
    lines.map((line) => line.text),
    ["@@ -1,2 +1,4 @@", " a", "+b", "+c", " d"],
  );
});

void test("files larger than the pairwise limit still diff as a block replace", () => {
  const before = text(...Array.from({ length: 1_400 }, (_, index) => `old ${index}`));
  const after = text(...Array.from({ length: 1_400 }, (_, index) => `new ${index}`));
  const stats = diffLineStats(unifiedDiffLines(before, after));

  assert.deepEqual(stats, { additions: 1_400, deletions: 1_400 });
});

void test("trailing newline differences do not register as changes", () => {
  assert.deepEqual(unifiedDiffLines("a\nb\n", "a\nb"), []);
});
