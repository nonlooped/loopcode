import assert from "node:assert/strict";
import test from "node:test";

import { WorkspacePersistence } from "../src/services/workspace-persistence.ts";

void test("retries the latest workspace after a transient save failure", async () => {
  let attempts = 0;
  let retried;
  const retry = new Promise((resolve) => {
    retried = resolve;
  });
  const persistence = new WorkspacePersistence(async () => {
    attempts += 1;
    if (attempts === 1) throw new Error("temporary failure");
    retried();
  });
  const workspace = { version: 2, selectedThreadId: "thread", projects: [], threads: [] };

  persistence.setReady();
  persistence.queue(workspace);
  await assert.rejects(persistence.flush(), /temporary failure/);
  await Promise.race([
    retry,
    new Promise((_, reject) => setTimeout(() => reject(new Error("save was not retried")), 1_000)),
  ]);

  assert.equal(attempts, 2);
});
