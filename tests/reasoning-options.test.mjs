import assert from "node:assert/strict";
import test from "node:test";

import { discoverModelOptions } from "../src/utils/model-options.ts";
import { applyReasoningForSelectedModel } from "../src/utils/reasoning-options.ts";

void test("inspects only the selected model when discovery already matches", async () => {
  const initial = {
    modelConfigId: "model",
    models: [
      { id: "opencode/big-pickle", name: "Big Pickle" },
      { id: "meta/muse-spark-1.2-contributor", name: "Muse Spark 1.2 Contributor" },
    ],
    selectedModelId: "opencode/big-pickle",
    reasoningOptions: [{ id: "high", name: "High" }],
    selectedReasoningId: "high",
  };
  let setModelCalls = 0;
  const connection = {
    async setModel() {
      setModelCalls += 1;
      throw new Error("should not select during inspection");
    },
  };

  const discovered = await discoverModelOptions(connection, initial, initial.selectedModelId);

  assert.equal(setModelCalls, 0);
  assert.deepEqual(discovered.reasoningOptionsByModel[initial.selectedModelId], {
    options: [{ id: "high", name: "High" }],
    selectedId: "high",
  });
  assert.equal(discovered.reasoningOptionsByModel["meta/muse-spark-1.2-contributor"], undefined);
});

void test("uses at most one fallback selection for a normalized model", async () => {
  const initial = {
    modelConfigId: "model",
    models: [{ id: "model-1", name: "Model 1" }],
    selectedModelId: undefined,
    reasoningOptions: [],
  };
  const calls = [];
  const connection = {
    async setModel(configId, modelId) {
      calls.push([configId, modelId]);
      return { ...initial, selectedModelId: modelId, reasoningOptions: [] };
    },
  };

  await discoverModelOptions(connection, initial, "model-1");

  assert.deepEqual(calls, [["model", "model-1"]]);
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
