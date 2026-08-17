import assert from "node:assert/strict";
import test from "node:test";

import { discoverModelOptions } from "../src/utils/model-options.ts";
import { applyReasoningForSelectedModel } from "../src/utils/reasoning-options.ts";

void test("discovers reasoning variants for every advertised model", async () => {
  const initial = {
    modelConfigId: "model",
    models: [
      { id: "opencode/big-pickle", name: "Big Pickle" },
      { id: "meta/muse-spark-1.2-contributor", name: "Muse Spark 1.2 Contributor" },
      { id: "unsupported/model", name: "Unsupported" },
    ],
    selectedModelId: "opencode/big-pickle",
    reasoningOptions: [],
  };
  const connection = {
    async setModel(_configId, modelId) {
      if (modelId === "unsupported/model") return initial;
      return {
        ...initial,
        selectedModelId: modelId,
        reasoningConfigId: "effort",
        reasoningOptions: [
          { id: "minimal", name: "Minimal" },
          { id: "high", name: "High" },
        ],
        selectedReasoningId: "minimal",
      };
    },
  };

  const discovered = await discoverModelOptions(connection, initial);

  assert.deepEqual(discovered.reasoningOptionsByModel["opencode/big-pickle"], {
    options: [],
    selectedId: undefined,
  });
  assert.deepEqual(discovered.reasoningOptionsByModel["meta/muse-spark-1.2-contributor"], {
    options: [
      { id: "minimal", name: "Minimal" },
      { id: "high", name: "High" },
    ],
    selectedId: "minimal",
  });
  assert.equal(discovered.reasoningOptionsByModel["unsupported/model"], undefined);
});

void test("updates disconnected reasoning options when the selected model changes", () => {
  const provider = {
    status: "disconnected",
    models: [],
    selectedModelId: "meta/muse-spark-1.2-contributor",
    reasoningOptions: [],
    reasoningOptionsByModel: {
      "opencode/big-pickle": { options: [] },
      "meta/muse-spark-1.2-contributor": {
        options: [
          { id: "minimal", name: "Minimal" },
          { id: "high", name: "High" },
        ],
        selectedId: "minimal",
      },
    },
  };

  applyReasoningForSelectedModel(provider);

  assert.deepEqual(provider.reasoningOptions, [
    { id: "minimal", name: "Minimal" },
    { id: "high", name: "High" },
  ]);
  assert.equal(provider.selectedReasoningId, "minimal");
});

void test("clears stale reasoning options for a model without variants", () => {
  const provider = {
    status: "disconnected",
    models: [],
    selectedModelId: "opencode/big-pickle",
    reasoningOptions: [{ id: "high", name: "High" }],
    selectedReasoningId: "high",
    reasoningOptionsByModel: {
      "opencode/big-pickle": { options: [] },
    },
  };

  applyReasoningForSelectedModel(provider);

  assert.deepEqual(provider.reasoningOptions, []);
  assert.equal(provider.selectedReasoningId, undefined);
});
