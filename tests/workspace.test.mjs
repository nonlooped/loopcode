import assert from "node:assert/strict";
import test from "node:test";

import { createWorkspaceState, Workspace } from "../src/services/workspace.ts";

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

function setup(cwd = "C:\\default") {
  const state = createWorkspaceState(cwd, catalogs);
  const events = [];
  const persistence = {
    setReady: () => events.push("ready"),
    queue: (snapshot) => events.push(snapshot),
    flush: async () => events.push("flushed"),
  };
  return { state, events, workspace: new Workspace(state, catalogs, persistence) };
}

void test("restores a workspace and resets transient provider runtime state", () => {
  const { state, events, workspace } = setup();
  assert.equal(
    workspace.initialize(
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
    ),
    true,
  );

  assert.equal(state.selectedThreadId, "thread-1");
  assert.equal(state.selectedProjectId, "project-1");
  assert.equal(state.threads[0].providers.codex.connectionStatus, "disconnected");
  assert.equal(state.threads[0].providers.codex.turnStatus, "idle");
  assert.equal(state.threads[0].providers.codex.selectedModelId, "model");
  assert.deepEqual(events, ["ready"]);
});

void test("rejects invalid workspaces and drops orphaned project references", () => {
  const invalid = setup();
  assert.equal(invalid.workspace.initialize({ version: 3, threads: [] }, "C:\\default"), false);
  assert.deepEqual(invalid.events, []);

  const { state, workspace } = setup();
  assert.equal(
    workspace.initialize(
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
    ),
    true,
  );
  assert.equal(state.threads[0].projectId, null);
});

void test("snapshots retain private sessions but exclude transient provider state", () => {
  const { state, events, workspace } = setup();
  assert.equal(
    workspace.initialize(
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
    ),
    true,
  );
  state.threads[0].providers.codex.connectionStatus = "ready";
  state.threads[0].providers.codex.turnStatus = "running";
  state.threads[0].providers.codex.harnessId = "transient-harness";
  state.threads[0].providers.codex.sessionId = "private-session";

  workspace.queuePersistence();
  const snapshot = events.at(-1);
  assert.equal("providers" in snapshot.threads[0], false);
  assert.equal(JSON.stringify(snapshot).includes("transient-harness"), false);
  assert.deepEqual(snapshot.threads[0].providerSessionIds, { codex: "private-session" });

  const restarted = setup();
  assert.equal(restarted.workspace.initialize(snapshot, "C:\\default"), true);
  assert.equal(restarted.state.threads[0].providers.codex.sessionId, "private-session");
});

void test("version 2 mixed attachment records round trip without adding base64 to references", () => {
  const { state, events, workspace } = setup();
  const reference = {
    attachmentId: "123e4567-e89b-12d3-a456-426614174000",
    mimeType: "image/png",
    name: "stored.png",
  };
  const legacy = { data: "AQID", mimeType: "image/png", name: "legacy.png" };
  assert.equal(
    workspace.initialize(
      {
        version: 2,
        selectedThreadId: "thread-1",
        projects: [],
        threads: [
          {
            id: "thread-1",
            title: "Images",
            profileId: "codex",
            cwd: "C:\\workspace",
            messages: [
              {
                id: "message-1",
                role: "user",
                text: "Both",
                images: [reference, legacy],
                createdAt: 1,
              },
            ],
            tools: [],
            draft: "",
            updatedAt: 1,
          },
        ],
      },
      "C:\\default",
    ),
    true,
  );
  assert.deepEqual(state.threads[0].messages[0].images, [reference, legacy]);

  workspace.queuePersistence();
  const snapshot = events.at(-1);
  assert.equal(snapshot.version, 2);
  assert.deepEqual(snapshot.threads[0].messages[0].images, [reference, legacy]);
  assert.equal("data" in snapshot.threads[0].messages[0].images[0], false);
});

void test("keeps project, selection, reuse, and deletion invariants behind one interface", () => {
  const { state, workspace } = setup();
  const project = workspace.ensureProject("C:\\loopcode");
  const first = workspace.addThread("C:\\default", () => false, project.id);
  assert.equal(first.projectId, project.id);
  assert.equal(state.selectedProjectId, project.id);
  assert.equal(state.selectedThreadId, first.id);

  assert.equal(
    workspace.addThread("C:\\default", () => false),
    first,
  );
  first.draft = "keep this";
  const second = workspace.addThread("C:\\default", () => false);
  assert.notEqual(second.id, first.id);

  workspace.removeThread(second.id, "C:\\default");
  assert.equal(state.selectedThreadId, first.id);
  workspace.removeProject(project.id);
  assert.equal(state.selectedProjectId, null);
  assert.equal(first.projectId, null);

  for (const id of state.threads.map((thread) => thread.id)) {
    workspace.removeThread(id, "C:\\default");
  }
  assert.equal(state.threads.length, 1);
  assert.equal(state.selectedThreadId, state.threads[0].id);
  assert.equal(state.threads[0].cwd, "C:\\default");
});
