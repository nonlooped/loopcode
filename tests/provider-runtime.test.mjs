import assert from "node:assert/strict";
import test from "node:test";

import { ProviderRuntime } from "../src/services/provider-runtime.ts";

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
    async prompt(text, images) {
      calls.push({ text, images });
    },
    async generateTitle() {
      return "**Runtime Owns Turns.**";
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }

  const state = thread();
  const runtime = new Runtime(catalogs, hooks);
  const completion = runtime.runTurn(state, "Implement this", [
    { data: "image-data", mimeType: "image/png", name: "reference.png" },
  ]);

  assert.ok(completion);
  await completion;
  assert.deepEqual(calls, [
    {
      text: "Implement this",
      images: [{ type: "image", data: "image-data", mimeType: "image/png" }],
    },
  ]);
  assert.equal(state.messages[0].role, "user");
  assert.equal(state.messages[0].images[0].name, "reference.png");
  assert.equal(state.title, "Runtime Owns Turns");
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
