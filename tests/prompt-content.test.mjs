import assert from "node:assert/strict";
import test from "node:test";

import {
  composerEnterAction,
  fuzzyScore,
  promptParts,
  promptPartsFromText,
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

const skill = {
  id: "ref-2",
  kind: "skill",
  name: "deslop",
  path: "C:\\repo\\.claude\\skills\\deslop",
  relativePath: ".claude/skills/deslop",
  uri: "file:///C:/repo/.claude/skills/deslop",
};

void test("edited prompt text resolves its tokens back to references", () => {
  const parts = promptPartsFromText("Read @src/Composer.svelte then $deslop it", [
    reference,
    skill,
  ]);
  assert.deepEqual(parts, [
    { type: "text", text: "Read " },
    { type: "reference", reference },
    { type: "text", text: " then " },
    { type: "reference", reference: skill },
    { type: "text", text: " it" },
  ]);
});

void test("editing away a reference token drops it from the resent prompt", () => {
  assert.deepEqual(promptPartsFromText("Read the composer", [reference]), [
    { type: "text", text: "Read the composer" },
  ]);
  assert.deepEqual(promptPartsFromText("", [reference]), []);
  assert.deepEqual(promptPartsFromText("plain text", []), [{ type: "text", text: "plain text" }]);
});

void test("overlapping reference tokens resolve to the longest match", () => {
  const nested = { ...reference, id: "ref-3", relativePath: "src/Composer.svelte.bak" };
  const parts = promptPartsFromText("@src/Composer.svelte.bak", [reference, nested]);
  assert.deepEqual(parts, [{ type: "reference", reference: nested }]);
});

void test("editing a reference token to extend it is treated as plain text, not the original reference", () => {
  const parts = promptPartsFromText("Read @src/Composer.svelte.bak first", [reference]);
  assert.deepEqual(parts, [{ type: "text", text: "Read @src/Composer.svelte.bak first" }]);
});

void test("composer send shortcuts preserve a newline path", () => {
  const key = { key: "Enter", shiftKey: false, ctrlKey: false, metaKey: false };
  assert.equal(composerEnterAction("enter", key), "send");
  assert.equal(composerEnterAction("enter", { ...key, shiftKey: true }), "newline");
  assert.equal(composerEnterAction("modifier-enter", key), "newline");
  assert.equal(composerEnterAction("modifier-enter", { ...key, ctrlKey: true }), "send");
  assert.equal(composerEnterAction("enter", { ...key, isComposing: true }), undefined);
});

void test("fuzzy matching accepts ordered path characters", () => {
  assert.notEqual(fuzzyScore("src/components/Composer.svelte", "scps"), undefined);
  assert.equal(fuzzyScore("src/components/Composer.svelte", "xyz"), undefined);
});
