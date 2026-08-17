import assert from "node:assert/strict";
import test from "node:test";

import { restoreWorkspace, workspaceSnapshot } from "../src/utils/workspace.ts";

const catalogs = {
  codex: {
    status: "ready",
    models: [{ id: "model", name: "Model" }],
    selectedModelId: "model",
    reasoningOptions: [],
  },
  claude: { status: "error", models: [], reasoningOptions: [] },
  opencode: { status: "error", models: [], reasoningOptions: [] },
};

void test("restores valid workspaces and resets transient provider runtime state", () => {
  const restored = restoreWorkspace(
    {
      version: 2,
      selectedThreadId: "thread-1",
      selectedProjectId: "project-1",
      projects: [{ id: "project-1", name: "LoopCode", path: "C:\\loopcode", createdAt: 1 }],
      threads: [
        {
          id: "thread-1",
          title: "Persist threads",
          profileId: "codex",
          cwd: "C:\\loopcode",
          messages: [{ id: "message-1", role: "user", text: "Hello", createdAt: 2 }],
          tools: [],
          draft: "draft",
          updatedAt: 2,
          settled: false,
          projectId: "project-1",
        },
      ],
    },
    "C:\\default",
    catalogs,
  );

  assert.equal(restored.selectedThreadId, "thread-1");
  assert.equal(restored.selectedProjectId, "project-1");
  assert.equal(restored.threads[0].providers.codex.status, "disconnected");
  assert.equal(restored.threads[0].providers.codex.selectedModelId, "model");
});

void test("rejects invalid workspaces and drops orphaned project references", () => {
  assert.equal(restoreWorkspace({ version: 3, threads: [] }, "C:\\default", catalogs), undefined);
  const restored = restoreWorkspace(
    {
      version: 2,
      selectedThreadId: "thread-1",
      projects: [],
      threads: [
        {
          id: "thread-1",
          title: "Thread",
          profileId: "codex",
          cwd: "C:\\workspace",
          messages: [],
          tools: [],
          draft: "",
          updatedAt: 1,
          projectId: "missing",
        },
      ],
    },
    "C:\\default",
    catalogs,
  );
  assert.equal(restored.threads[0].projectId, null);
});

void test("workspace snapshots retain private session mappings but exclude transient runtime state", () => {
  const restored = restoreWorkspace(
    {
      version: 1,
      selectedThreadId: "thread-1",
      threads: [
        {
          id: "thread-1",
          title: "Thread",
          profileId: "codex",
          cwd: "C:\\workspace",
          messages: [],
          tools: [],
          draft: "",
          updatedAt: 1,
        },
      ],
    },
    "C:\\default",
    catalogs,
  );
  restored.threads[0].providers.codex.status = "running";
  restored.threads[0].providers.codex.harnessId = "transient-harness";
  restored.threads[0].providers.codex.sessionId = "private-session";

  const snapshot = workspaceSnapshot(restored.threads, "thread-1", [], null);
  const restarted = restoreWorkspace(snapshot, "C:\\default", catalogs);
  assert.equal("providers" in snapshot.threads[0], false);
  assert.equal(JSON.stringify(snapshot).includes("transient-harness"), false);
  assert.deepEqual(snapshot.threads[0].providerSessionIds, { codex: "private-session" });
  assert.equal(restarted.threads[0].providers.codex.sessionId, "private-session");
});
