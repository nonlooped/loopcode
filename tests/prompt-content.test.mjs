import assert from "node:assert/strict";
import test from "node:test";

import {
  fuzzyScore,
  promptParts,
  promptText,
  REFERENCE_PLACEHOLDER,
} from "../src/utils/prompt-content.ts";

const reference = {
  id: "ref-1",
  kind: "file",
  name: "Composer.svelte",
  path: "C:\\repo\\src\\Composer.svelte",
  relativePath: "src/Composer.svelte",
  uri: "file:///C:/repo/src/Composer.svelte",
};

void test("prompt references keep their inline position", () => {
  const parts = promptParts(`Read ${REFERENCE_PLACEHOLDER} first`, [reference]);
  assert.deepEqual(parts, [
    { type: "text", text: "Read " },
    { type: "reference", reference },
    { type: "text", text: " first" },
  ]);
  assert.equal(promptText(parts), "Read @src/Composer.svelte first");
});

void test("fuzzy matching accepts ordered path characters", () => {
  assert.notEqual(fuzzyScore("src/components/Composer.svelte", "scps"), undefined);
  assert.equal(fuzzyScore("src/components/Composer.svelte", "xyz"), undefined);
});
