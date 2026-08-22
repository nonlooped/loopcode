import assert from "node:assert/strict";
import test from "node:test";

import { copyImage } from "../src/utils/clipboard.ts";

function replaceGlobal(context, name, value) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, name);
  Object.defineProperty(globalThis, name, { configurable: true, value });
  context.after(() => {
    if (descriptor) Object.defineProperty(globalThis, name, descriptor);
    else delete globalThis[name];
  });
}

void test("rejects image copy when the clipboard API is unavailable", async (context) => {
  replaceGlobal(context, "navigator", { clipboard: {} });
  await assert.rejects(copyImage("blob:test"), /not supported/i);
});

void test("uses a PNG type when fetched image metadata has no MIME type", async (context) => {
  const writes = [];
  replaceGlobal(context, "navigator", {
    clipboard: { write: async (items) => writes.push(items) },
  });
  replaceGlobal(context, "fetch", async () => ({ blob: async () => new Blob(["image"]) }));
  replaceGlobal(
    context,
    "ClipboardItem",
    class {
      constructor(items) {
        this.items = items;
      }
    },
  );

  await copyImage("blob:test");

  assert.ok(writes[0][0].items["image/png"]);
});
