import assert from "node:assert/strict";
import test from "node:test";

import { compareSidebarThreads, createThread, threadAttention } from "../src/utils/threads.ts";

const catalogs = {
  codex: {
    status: "ready",
    models: [{ id: "model", name: "Model" }],
    selectedModelId: "model",
    reasoningOptions: [],
  },
};

function makeThread(updatedAt = 0) {
  const thread = createThread("/workspace", null, catalogs);
  thread.updatedAt = updatedAt;
  return thread;
}

void test("derives actionable thread states and reasons", () => {
  const thread = makeThread();
  assert.deepEqual(threadAttention(thread), { kind: "recent" });

  thread.providers.codex.turnStatus = "running";
  thread.tools.push({
    id: "tool",
    title: "Running tests",
    kind: "execute",
    status: "in_progress",
    locations: [],
    createdAt: 1,
  });
  assert.deepEqual(threadAttention(thread), {
    kind: "working",
    label: "Working",
    reason: "Running tests",
  });

  thread.providers.codex.turnStatus = "idle";
  thread.providers.codex.connectionStatus = "connecting";
  assert.deepEqual(threadAttention(thread), {
    kind: "working",
    label: "Working",
    reason: "Connecting to provider",
  });

  thread.providers.codex.connectionStatus = "ready";
  thread.providers.codex.turnStatus = "failed";
  thread.providers.codex.error = "Tests failed";
  assert.deepEqual(threadAttention(thread), {
    kind: "failed",
    label: "Failed",
    reason: "Tests failed",
  });

  assert.deepEqual(
    threadAttention(thread, {
      type: "permission",
      requestId: "permission",
      title: "Run npm test",
      detail: "",
      options: [],
    }),
    { kind: "needs-approval", label: "Needs approval", reason: "Run npm test" },
  );
  assert.deepEqual(
    threadAttention(thread, {
      type: "question",
      requestId: "question",
      title: "Choose a migration",
      detail: "",
      options: [],
      allowMultiple: false,
      allowCustomAnswer: true,
      required: true,
    }),
    { kind: "needs-answer", label: "Needs answer", reason: "Choose a migration" },
  );
});

void test("sorts needs-you threads ahead of working and recent threads", () => {
  const recent = makeThread(4);
  const working = makeThread(3);
  working.providers.codex.turnStatus = "running";
  const failed = makeThread(2);
  failed.providers.codex.turnStatus = "failed";
  const approval = makeThread(1);
  const requests = {
    [approval.id]: {
      type: "permission",
      requestId: "permission",
      title: "Run command",
      detail: "",
      options: [],
    },
  };

  const sorted = [recent, working, failed, approval].sort((left, right) =>
    compareSidebarThreads(left, right, requests[left.id], requests[right.id]),
  );

  assert.deepEqual(
    sorted.map((thread) => thread.id),
    [approval.id, failed.id, working.id, recent.id],
  );
});
