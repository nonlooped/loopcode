import assert from "node:assert/strict";
import test from "node:test";

import {
  discoverProviderModelOptions,
  mergeProviderModels,
  ProviderRuntime,
} from "../src/services/provider-runtime.ts";
import { providerDefinitions } from "../src/config/provider-definitions.ts";
import { restoreWorkspace } from "../src/utils/workspace.ts";

const catalogs = {
  codex: {
    status: "ready",
    models: [{ id: "model-1", name: "Model 1" }],
    selectedModelId: "model-1",
    reasoningOptions: [],
  },
};

function provider(connectionStatus = "ready") {
  return {
    connectionStatus,
    turnStatus: "idle",
    models: catalogs.codex.models,
    selectedModelId: "model-1",
    reasoningOptions: [],
  };
}

function thread(connectionStatus) {
  return {
    id: "thread-1",
    title: "New thread",
    profileId: "codex",
    cwd: "C:\\workspace",
    messages: [],
    tools: [],
    draft: "",
    draftReferences: [],
    providers: {
      codex: provider(connectionStatus),
      claude: provider(),
    },
    updatedAt: 1,
    settled: false,
  };
}

const hooks = { permission() {}, clearPermission() {} };

void test("ProviderRuntime owns prompt and title lifecycle", async () => {
  const calls = [];
  const connection = {
    async prompt(content) {
      calls.push(content);
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }

  const state = thread();
  const runtime = new Runtime(catalogs, hooks);
  const completion = runtime.runTurn(
    state,
    "Implement this @src/Composer.svelte",
    [{ data: "image-data", mimeType: "image/png", name: "reference.png" }],
    [
      { type: "text", text: "Implement this " },
      {
        type: "reference",
        reference: {
          id: "ref-1",
          kind: "file",
          name: "Composer.svelte",
          path: "C:\\workspace\\src\\Composer.svelte",
          relativePath: "src/Composer.svelte",
          uri: "file:///C:/workspace/src/Composer.svelte",
        },
      },
    ],
  );

  assert.ok(completion);
  await completion;
  assert.deepEqual(calls, [
    [
      { type: "text", text: "Implement this " },
      {
        type: "resource_link",
        uri: "file:///C:/workspace/src/Composer.svelte",
        name: "Composer.svelte",
        title: "src/Composer.svelte",
      },
      { type: "image", data: "image-data", mimeType: "image/png" },
    ],
  ]);
  assert.equal(state.messages[0].role, "user");
  assert.equal(state.messages[0].images[0].name, "reference.png");
  assert.equal(state.title, "Implement this @src/Composer.svelte");
});

void test("unavailable providers cannot become active and restored providers fall back", () => {
  const unavailableCatalogs = {
    ...catalogs,
    grok: {
      status: "unavailable",
      models: [],
      reasoningOptions: [],
      unavailableReason: "authentication",
      error: "Authentication required",
    },
  };
  const state = thread();
  state.providers.grok = provider();
  const runtime = new ProviderRuntime(unavailableCatalogs, hooks);

  runtime.activate(state, "grok", false);
  assert.equal(state.profileId, "codex");

  const restored = restoreWorkspace(
    {
      version: 1,
      selectedThreadId: "restored-thread",
      threads: [
        {
          id: "restored-thread",
          title: "Restored Grok thread",
          profileId: "grok",
          cwd: "C:\\workspace",
          messages: [],
          tools: [],
          draft: "",
          updatedAt: 1,
        },
      ],
    },
    "C:\\workspace",
    unavailableCatalogs,
  );
  assert.ok(restored);
  runtime.applyCatalog("grok", restored.threads);
  assert.equal(restored.threads[0].profileId, "codex");
  assert.equal(restored.threads[0].providers.grok.selectedModelId, undefined);
});

void test("changing provider arguments disconnects its active harness", () => {
  const state = thread();
  const runtime = new ProviderRuntime(catalogs, hooks);
  const profiles = providerDefinitions.map((profile) =>
    profile.id === "codex" ? { ...profile, args: [...profile.args, "--changed"] } : profile,
  );

  runtime.setProfiles(profiles, [state]);

  assert.equal(state.providers.codex.connectionStatus, "disconnected");
});

void test("unchanged custom models do not disconnect ready providers", () => {
  const state = thread();
  const runtime = new ProviderRuntime(catalogs, hooks);

  runtime.setCustomModels("codex", [], [state]);

  assert.equal(state.providers.codex.connectionStatus, "ready");
});

void test("custom models extend advertised models and replace duplicate labels", () => {
  assert.deepEqual(
    mergeProviderModels(
      [
        { id: "model-1", name: "Model 1" },
        { id: "model-2", name: "Old model 2" },
      ],
      [
        { id: "model-2", name: "Model 2" },
        { id: "custom", name: "Custom" },
      ],
    ),
    [
      { id: "model-1", name: "Model 1" },
      { id: "model-2", name: "Model 2" },
      { id: "custom", name: "Custom" },
    ],
  );
});

void test("disabling the active provider falls back to an enabled ready provider", () => {
  const state = thread();
  const readyCatalogs = { ...catalogs, claude: catalogs.codex };
  const runtime = new ProviderRuntime(readyCatalogs, hooks);

  runtime.setDisabledProfiles(["codex"], [state]);

  assert.equal(state.profileId, "claude");
  assert.equal(state.providers.codex.connectionStatus, "disconnected");
});

void test("model defaults apply while discovery is still loading", async () => {
  const state = thread("disconnected");
  state.providers.codex.reasoningOptionsByModel = {
    "model-2": { options: [{ id: "high", name: "High" }], selectedId: "high" },
  };
  const runtime = new ProviderRuntime(
    { codex: { status: "loading", models: [], reasoningOptions: [] } },
    hooks,
  );

  await runtime.selectModel(state, "codex", "model-2");

  assert.equal(state.providers.codex.selectedModelId, "model-2");
  assert.equal(state.providers.codex.selectedReasoningId, "high");
});

void test("Grok and fx image turns are rejected before timeline changes", () => {
  for (const profileId of ["grok", "fx"]) {
    const imageCatalogs = {
      ...catalogs,
      [profileId]: catalogs.codex,
    };
    const state = thread();
    state.profileId = profileId;
    state.providers[profileId] = provider();
    const runtime = new ProviderRuntime(imageCatalogs, hooks);

    assert.equal(
      runtime.runTurn(state, "Describe this", [
        { data: "image-data", mimeType: "image/png", name: "reference.png" },
      ]),
      undefined,
    );
    assert.deepEqual(state.messages, []);
  }
});

void test("Pi discovery skips per-model option probing", async () => {
  const pi = providerDefinitions.find((profile) => profile.id === "pi");
  assert.ok(pi);
  let modelChanges = 0;
  const state = {
    modelConfigId: "model",
    models: [
      { id: "pi-fast", name: "Pi Fast" },
      { id: "pi-smart", name: "Pi Smart" },
    ],
    selectedModelId: "pi-fast",
    reasoningConfigId: "thinking",
    reasoningOptions: [{ id: "medium", name: "Medium" }],
    selectedReasoningId: "medium",
  };

  const options = await discoverProviderModelOptions(
    pi,
    {
      async setModel() {
        modelChanges += 1;
        return state;
      },
    },
    state,
  );

  assert.equal(modelChanges, 0);
  assert.deepEqual(options.reasoningOptionsByModel, {
    "pi-fast": {
      options: [{ id: "medium", name: "Medium" }],
      selectedId: "medium",
    },
  });
});

void test("a failed reconnect stop drops the defunct connection", async () => {
  let failStop = false;
  const runtime = new ProviderRuntime(catalogs, hooks, (callbacks) => ({
    async connect() {
      callbacks.ready({
        harnessId: "harness-1",
        sessionId: "session-1",
        modelConfigId: "model",
        models: catalogs.codex.models,
        selectedModelId: "model-1",
        reasoningOptions: [],
      });
    },
    async setModel() {
      return {
        modelConfigId: "model",
        models: catalogs.codex.models,
        selectedModelId: "model-1",
        reasoningOptions: [],
      };
    },
    async stop() {
      if (failStop) throw new Error("stop failed");
    },
  }));
  const state = thread("disconnected");

  await runtime.connect(state, "codex");
  failStop = true;
  await runtime.connect(state, "codex");

  assert.equal(runtime.connection(state.id, "codex"), undefined);
  assert.equal(state.providers.codex.error, "stop failed");
});

void test("a provider switch prevents a stale reconnect from prompting", async () => {
  let finishConnect;
  let prompts = 0;
  const connected = new Promise((resolve) => {
    finishConnect = resolve;
  });
  const connection = {
    async prompt() {
      prompts += 1;
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {}
    async connect(state, profileId) {
      await connected;
      state.providers[profileId].connectionStatus = "ready";
      return connection;
    }
  }

  const state = thread("disconnected");
  const runtime = new Runtime(catalogs, hooks);
  const completion = runtime.runTurn(state, "Do not send after switching");
  runtime.activate(state, "claude", false);
  finishConnect();

  await completion;
  assert.equal(prompts, 0);
});
