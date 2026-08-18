import assert from "node:assert/strict";
import test from "node:test";

import { loadComposerImages, migrateLegacyAttachments } from "../src/utils/attachments.ts";

const ID = "123e4567-e89b-12d3-a456-426614174000";

void test("legacy attachment migration replaces only successful bounded writes", async () => {
  const workspace = {
    threads: [
      {
        messages: [
          {
            images: [
              { data: "AQIDBA==", mimeType: "image/png", name: "ok.png" },
              { data: "BQY=", mimeType: "image/png", name: "keep.png" },
              { data: "not base64", mimeType: "image/png", name: "invalid.png" },
            ],
          },
        ],
      },
    ],
  };
  let write = 0;
  const result = await migrateLegacyAttachments(
    workspace,
    async (_id, bytes) => {
      write += 1;
      if (write === 2) throw new Error("disk full");
      assert.deepEqual([...bytes], [1, 2, 3, 4]);
    },
    () => ID,
  );

  assert.deepEqual(result, { migrated: 1, failed: 2 });
  assert.deepEqual(workspace.threads[0].messages[0].images[0], {
    attachmentId: ID,
    mimeType: "image/png",
    name: "ok.png",
  });
  assert.equal(workspace.threads[0].messages[0].images[1].data, "BQY=");
  assert.equal(workspace.threads[0].messages[0].images[2].data, "not base64");
});

void test("composer selection rolls back stored files and object URLs after a partial failure", async () => {
  const originalCreate = URL.createObjectURL;
  const originalRevoke = URL.revokeObjectURL;
  const revoked = [];
  URL.createObjectURL = (file) => `blob:${file.name}`;
  URL.revokeObjectURL = (url) => revoked.push(url);
  const files = ["one.png", "two.png"].map((name) => ({
    name,
    type: "image/png",
    size: 1,
    async arrayBuffer() {
      return Uint8Array.of(1).buffer;
    },
  }));
  const stored = [];
  const removed = [];
  try {
    await assert.rejects(
      loadComposerImages(
        files,
        0,
        async (id) => {
          if (stored.length === 1) throw new Error("write failed");
          stored.push(id);
        },
        async (id) => {
          removed.push(id);
        },
      ),
      /write failed/,
    );
  } finally {
    URL.createObjectURL = originalCreate;
    URL.revokeObjectURL = originalRevoke;
  }
  assert.deepEqual(removed, stored);
  assert.deepEqual(revoked, ["blob:one.png"]);
});
