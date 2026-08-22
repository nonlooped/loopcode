import assert from "node:assert/strict";
import test from "node:test";

import { createWorkspaceState, Workspace } from "../src/services/workspace.ts";
import { restoreWorkspace } from "../src/utils/workspace.ts";

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

void test("repairs unknown persisted provider ids explicitly", () => {
  const restored = restoreWorkspace(
    {
      version: 1,
      selectedThreadId: "thread-1",
      threads: [
        {
          id: "thread-1",
          title: "Thread",
          profileId: "removed-provider",
          cwd: "C:\\workspace",
          messages: [],
          tools: [],
          updatedAt: 1,
        },
      ],
    },
    "C:\\default",
    catalogs,
  );

  assert.equal(restored.threads[0].profileId, "codex");
  assert.deepEqual(restored.providerRepairs, [
    { threadId: "thread-1", persistedProfileId: "removed-provider", profileId: "codex" },
  ]);
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
  assert.notEqual(
    workspace.addThread("C:\\default", () => false, project.id, false),
    first,
  );
  state.threads = [first];
  state.selectedThreadId = first.id;
  first.draft = "keep this";
  const second = workspace.addThread("C:\\default", () => false);
  assert.notEqual(second.id, first.id);

  workspace.removeThread(second.id, "C:\\default");
  assert.equal(state.selectedThreadId, first.id);
  workspace.removeProject(project.id);
  assert.equal(state.selectedProjectId, null);
  assert.equal(first.projectId, null);

  let replacement;
  for (const id of state.threads.map((thread) => thread.id)) {
    replacement = workspace.removeThread(id, "C:\\default") ?? replacement;
  }
  assert.equal(replacement, state.threads[0]);
  assert.equal(state.threads.length, 1);
  assert.equal(state.selectedThreadId, state.threads[0].id);
  assert.equal(state.threads[0].cwd, "C:\\default");
});
