import assert from "node:assert/strict";
import test from "node:test";

import {
  ProviderRuntime,
  applyProviderConfigState,
  bytesToBase64,
} from "../src/services/provider-runtime.ts";

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
  const attachmentId = "123e4567-e89b-12d3-a456-426614174000";
  const runtime = new Runtime(catalogs, hooks, async (requestedId) => {
    assert.equal(requestedId, attachmentId);
    return Uint8Array.of(0, 1, 2, 253, 254, 255).buffer;
  });
  const completion = runtime.runTurn(state, "Implement this", [
    { attachmentId, mimeType: "image/png", name: "reference.png" },
  ]);

  assert.ok(completion);
  await completion;
  assert.deepEqual(calls, [
    {
      text: "Implement this",
      images: [{ type: "image", data: "AAEC/f7/", mimeType: "image/png" }],
    },
  ]);
  assert.equal(state.messages[0].role, "user");
  assert.deepEqual(state.messages[0].images[0], {
    attachmentId,
    mimeType: "image/png",
    name: "reference.png",
  });
  assert.equal(JSON.stringify(state.messages).includes("AAEC/f7/"), false);
  assert.equal(state.title, "Runtime Owns Turns");
});

void test("base64 encoding remains exact across bounded chunk boundaries", () => {
  const bytes = Uint8Array.from({ length: 48 * 1024 + 5 }, (_, index) => index % 251);
  assert.equal(bytesToBase64(bytes), Buffer.from(bytes).toString("base64"));
});

void test("attachment read failures use turn errors without removing the reference", async () => {
  const connection = {
    async prompt() {
      assert.fail("prompt should not run");
    },
  };
  class Runtime extends ProviderRuntime {
    connection() {
      return connection;
    }
  }
  const state = thread();
  const image = {
    attachmentId: "123e4567-e89b-12d3-a456-426614174000",
    mimeType: "image/png",
    name: "missing.png",
  };
  const runtime = new Runtime(catalogs, hooks, async () => {
    throw new Error("missing file");
  });
  await runtime.runTurn(state, "Keep this", [image]);
  assert.deepEqual(state.messages[0].images, [image]);
  assert.equal(state.providers.codex.turnStatus, "failed");
  assert.match(state.messages.at(-1).text, /Could not read attachment: missing file/);
});

void test("connected model updates memoize capabilities for later selection", () => {
  const state = provider();
  state.selectedModelId = "model-2";

  applyProviderConfigState(state, {
    models: [],
    selectedModelId: "model-2",
    reasoningOptions: [{ id: "high", name: "High" }],
    selectedReasoningId: "high",
    fastModeConfigId: "fast-mode",
    fastModeEnabled: true,
  });

  assert.deepEqual(state.reasoningOptionsByModel["model-2"], {
    options: [{ id: "high", name: "High" }],
    selectedId: "high",
  });
  assert.equal(state.fastModeOptionsByModel["model-2"].enabled, true);
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
