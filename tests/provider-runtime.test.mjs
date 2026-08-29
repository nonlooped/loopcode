import assert from "node:assert/strict";
import test from "node:test";

import { ProviderRuntime } from "../src/services/provider-runtime.ts";
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
    },
    updatedAt: 1,
    settled: false,
  };
}

const hooks = { permission() {}, clearPermission() {} };

const [codexProfile] = providerDefinitions;
const secondaryProfile = { ...codexProfile, id: "secondary", label: "Secondary" };
function configuration(
  profiles = providerDefinitions,
  { customModels = {}, disabledProfileIds = [] } = {},
) {
  return { profiles, customModels, disabledProfileIds };
}

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

void test("a running provider can accept an advertised follow-up", async () => {
  const calls = [];
  const connection = {
    async followUp(content) {
      calls.push(content);
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }

  const state = thread();
  state.providers.codex.turnStatus = "running";
  state.providers.codex.supportsFollowups = true;
  const runtime = new Runtime(catalogs, hooks);

  await runtime.runTurn(state, "Also run the tests");

  assert.deepEqual(calls, [[{ type: "text", text: "Also run the tests" }]]);
  assert.equal(state.messages.at(-1).role, "user");
  assert.equal(state.messages.at(-1).text, "Also run the tests");
});

void test("a failed follow-up attaches its failure to the sent message instead of losing it", async () => {
  const connection = {
    async followUp() {
      throw new Error("session closed");
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }

  const state = thread();
  state.providers.codex.turnStatus = "running";
  state.providers.codex.supportsFollowups = true;
  const runtime = new Runtime(catalogs, hooks);

  await runtime.runTurn(state, "Also run the tests");

  const message = state.messages.at(-1);
  assert.equal(message.role, "user");
  assert.equal(message.followUp, true);
  assert.equal(message.failure?.title, "Could not send follow-up: session closed");
  assert.deepEqual(message.failure.actions, ["retry"]);
});

void test("retrying a failed follow-up resends it without leaving a duplicate behind", async () => {
  const calls = [];
  const connection = {
    async followUp(content) {
      calls.push(content);
      if (calls.length === 1) throw new Error("session closed");
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }

  const state = thread();
  state.providers.codex.turnStatus = "running";
  state.providers.codex.supportsFollowups = true;
  const runtime = new Runtime(catalogs, hooks);

  await runtime.runTurn(state, "Also run the tests");
  await runtime.retryMessage(state, state.messages.at(-1).id);

  assert.equal(calls.length, 2);
  const sent = state.messages.filter((message) => message.role === "user");
  assert.equal(sent.length, 1);
  assert.equal(sent[0].text, "Also run the tests");
  assert.equal(sent[0].failure, undefined);
});

void test("a retry the provider cannot accept keeps the message and its failure on screen", async () => {
  const connection = {
    async followUp() {
      throw new Error("session closed");
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }

  const state = thread();
  state.providers.codex.turnStatus = "running";
  state.providers.codex.supportsFollowups = true;
  const runtime = new Runtime(catalogs, hooks);

  await runtime.runTurn(state, "Also run the tests");
  const failed = state.messages.at(-1);
  // The connection dropped between the failure and the retry, so runTurn refuses the prompt.
  state.providers.codex.connectionStatus = "disconnected";

  await runtime.retryMessage(state, failed.id);

  assert.deepEqual(
    state.messages.map((message) => message.text),
    ["Also run the tests"],
  );
  assert.equal(state.messages[0].failure?.title, "Could not send follow-up: session closed");
});

void test("retryLastTurn resends after a turn-scoped failure", async () => {
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
  state.messages.push({ id: "user-1", role: "user", text: "Do the thing", createdAt: 1 });
  // A turn-scoped failure leaves connectionStatus "ready" with turnStatus stuck at "failed" —
  // nothing else resets it, so this reproduces the state retryLastTurn has to recover from.
  state.providers.codex.turnStatus = "failed";
  const runtime = new Runtime(catalogs, hooks);

  await runtime.retryLastTurn(state);

  assert.deepEqual(calls, [[{ type: "text", text: "Do the thing" }]]);
});

void test("retryLastTurn dismisses an informational failure without resending a completed turn", async () => {
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
  state.messages.push({
    // The id #upsertFailure assigns a synthesized failure notice.
    id: "failure-f1",
    role: "error",
    text: "Approaching your rate limit",
    failure: {
      id: "f1",
      revision: 1,
      category: "limit",
      severity: "warning",
      title: "Approaching your rate limit",
      actions: ["retry"],
    },
    createdAt: 1,
  });
  // turnStatus is left "idle" (the default), matching a session-failure notice reported as
  // metadata on a turn that otherwise completed normally.

  const runtime = new Runtime(catalogs, hooks);
  await runtime.retryLastTurn(state);

  assert.deepEqual(calls, []);
  // The notice is nothing but its failure, so dismissing it drops the whole row rather than
  // leaving the red text behind with its buttons stripped.
  assert.deepEqual(state.messages, []);
});

void test("a running turn prevents switching providers", () => {
  const state = thread();
  state.providers.codex.turnStatus = "running";
  const runtime = new ProviderRuntime({ ...catalogs, secondary: catalogs.codex }, hooks);
  runtime.configure(configuration([codexProfile, secondaryProfile]), [state]);
  state.providers.codex.turnStatus = "running";

  runtime.activate(state, "secondary", false);

  assert.equal(state.profileId, "codex");
});

void test("unavailable providers cannot become active and restored providers fall back", () => {
  const unavailableCatalogs = {
    ...catalogs,
    secondary: {
      status: "unavailable",
      models: [],
      reasoningOptions: [],
      unavailableReason: "authentication",
      error: "Authentication required",
    },
  };
  const state = thread();
  state.providers.secondary = provider();
  const runtime = new ProviderRuntime(unavailableCatalogs, hooks);
  runtime.configure(configuration([codexProfile, secondaryProfile]), [state]);

  runtime.activate(state, "secondary", false);
  assert.equal(state.profileId, "codex");

  const restored = restoreWorkspace(
    {
      version: 2,
      selectedThreadId: "restored-thread",
      projects: [],
      threads: [
        {
          id: "restored-thread",
          title: "Restored secondary thread",
          profileId: "secondary",
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
  runtime.applyCatalog("secondary", restored.threads);
  assert.equal(restored.threads[0].profileId, "codex");
  assert.equal(restored.threads[0].providers.secondary, undefined);
  assert.deepEqual(restored.providerRepairs, [
    { threadId: "restored-thread", persistedProfileId: "secondary", profileId: "codex" },
  ]);
});

void test("changing provider arguments disconnects its active harness", () => {
  const state = thread();
  const runtime = new ProviderRuntime(catalogs, hooks);
  const profiles = providerDefinitions.map((profile) =>
    profile.id === "codex" ? { ...profile, args: [...profile.args, "--changed"] } : profile,
  );

  runtime.configure(configuration(profiles), [state]);

  assert.equal(state.providers.codex.connectionStatus, "disconnected");
});

void test("unchanged custom models do not disconnect ready providers", () => {
  const state = thread();
  const runtime = new ProviderRuntime(catalogs, hooks);

  runtime.configure(configuration(), [state]);

  assert.equal(state.providers.codex.connectionStatus, "ready");
});

void test("disabling the active provider falls back to an enabled ready provider", () => {
  const state = thread();
  const readyCatalogs = { ...catalogs, secondary: catalogs.codex };
  const runtime = new ProviderRuntime(readyCatalogs, hooks);

  runtime.configure(
    configuration([codexProfile, secondaryProfile], { disabledProfileIds: ["codex"] }),
    [state],
  );

  assert.equal(state.profileId, "secondary");
  assert.equal(state.providers.codex.connectionStatus, "disconnected");
});

void test("runtime configuration applies models and provider fallback atomically", () => {
  const state = thread();
  const readyCatalogs = { ...catalogs, secondary: catalogs.codex };
  const runtime = new ProviderRuntime(readyCatalogs, hooks);

  runtime.configure(
    configuration([codexProfile, secondaryProfile], {
      customModels: { codex: [{ id: "custom", name: "Custom" }] },
      disabledProfileIds: ["codex"],
    }),
    [state],
  );

  assert.deepEqual(
    readyCatalogs.codex.models.map(({ id }) => id),
    ["model-1", "custom"],
  );
  assert.equal(state.profileId, "secondary");
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

void test("connection startup restores preselected session options", async () => {
  const selections = [];
  const modes = [
    { id: "default", name: "Manual" },
    { id: "plan", name: "Plan" },
  ];
  const agents = [
    { id: "default", name: "Default" },
    { id: "reviewer", name: "Reviewer" },
  ];
  const sessionState = (mode = "default", agent = "default") => ({
    harnessId: "harness-1",
    sessionId: "session-1",
    modelConfigId: "model",
    models: catalogs.codex.models,
    selectedModelId: "model-1",
    reasoningOptions: [],
    selects: {
      mode: { configId: "mode", options: modes, selectedId: mode },
      agent: { configId: "agent", options: agents, selectedId: agent },
    },
  });
  let selectedMode = "default";
  let selectedAgent = "default";
  const runtime = new ProviderRuntime(catalogs, hooks, (callbacks) => ({
    async connect() {
      callbacks.ready(sessionState());
    },
    async setConfigOption(configId, value) {
      selections.push([configId, value]);
      if (configId === "mode") selectedMode = value;
      if (configId === "agent") selectedAgent = value;
      return sessionState(selectedMode, selectedAgent);
    },
    async stop() {},
  }));
  const state = thread("disconnected");
  Object.assign(state.providers.codex, {
    selects: {
      mode: { configId: "mode", options: modes, selectedId: "plan" },
      agent: { configId: "agent", options: agents, selectedId: "reviewer" },
    },
  });

  await runtime.connect(state, "codex");

  assert.deepEqual(selections, [
    ["model", "model-1"],
    ["mode", "plan"],
    ["agent", "reviewer"],
  ]);
  assert.equal(state.providers.codex.selects.mode.selectedId, "plan");
  assert.equal(state.providers.codex.selects.agent.selectedId, "reviewer");
});

void test("reconnect waits for a replaced provider process to stop", async () => {
  let finishStop;
  let connectionCount = 0;
  const stopping = new Promise((resolve) => {
    finishStop = resolve;
  });
  const runtime = new ProviderRuntime(catalogs, hooks, (callbacks) => {
    connectionCount += 1;
    const connectionNumber = connectionCount;
    return {
      async connect() {
        callbacks.ready({
          harnessId: `harness-${connectionNumber}`,
          sessionId: `session-${connectionNumber}`,
          modelConfigId: "model",
          models: catalogs.codex.models,
          selectedModelId: "model-1",
          reasoningOptions: [],
        });
      },
      async setConfigOption() {
        return {
          modelConfigId: "model",
          models: catalogs.codex.models,
          selectedModelId: "model-1",
          reasoningOptions: [],
        };
      },
      async stop() {
        if (connectionNumber === 1) await stopping;
      },
    };
  });
  const state = thread("disconnected");
  await runtime.connect(state, "codex");
  const profiles = providerDefinitions.map((profile) =>
    profile.id === "codex" ? { ...profile, args: [...profile.args, "--changed"] } : profile,
  );

  runtime.configure(configuration(profiles), [state]);
  const reconnect = runtime.connect(state, "codex");
  await Promise.resolve();
  assert.equal(connectionCount, 1);
  finishStop();
  await reconnect;

  assert.equal(connectionCount, 2);
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
    async setConfigOption() {
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

void test("title generation does not replace the active thread harness", async () => {
  const titleRequests = [];
  const activeConnection = { async prompt() {} };
  class Runtime extends ProviderRuntime {
    connection() {
      return activeConnection;
    }
  }
  const runtime = new Runtime(catalogs, hooks, () => ({
    async connect(request) {
      titleRequests.push(request);
    },
    async generateTitle() {
      return "Generated title";
    },
    async stop() {},
  }));
  const state = thread();
  runtime.setTitlePreference({ profileId: "codex", modelId: "model-1" });

  await runtime.runTurn(state, "Name this thread");

  assert.equal(titleRequests.length, 1);
  assert.equal(titleRequests[0].threadId, undefined);
  assert.equal(state.title, "Generated title");
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
  state.providers.secondary = provider("disconnected");
  const runtime = new Runtime({ ...catalogs, secondary: catalogs.codex }, hooks);
  runtime.configure(configuration([codexProfile, secondaryProfile]), [state]);
  state.providers.codex.connectionStatus = "disconnected";
  const completion = runtime.runTurn(state, "Do not send after switching");
  runtime.activate(state, "secondary", false);
  finishConnect();

  await completion;
  assert.equal(state.profileId, "secondary");
  assert.equal(prompts, 0);
});
