import assert from "node:assert/strict";
import test from "node:test";

import { readModelState } from "../src/utils/model-state.ts";

void test("prefers the model selector when an agent advertises provider and model categories", () => {
  const state = readModelState({
    configOptions: [
      {
        id: "provider",
        name: "Provider",
        category: "model",
        type: "select",
        currentValue: "gateway",
        options: [{ value: "gateway", name: "Vercel AI Gateway" }],
      },
      {
        id: "model",
        name: "Model",
        category: "model",
        type: "select",
        currentValue: "anthropic/claude-opus-4.6",
        options: [
          { value: "anthropic/claude-opus-4.6", name: "Claude Opus 4.6" },
          { value: "openai/gpt-5.4", name: "GPT-5.4" },
        ],
      },
    ],
  });

  assert.equal(state.modelConfigId, "model");
  assert.equal(state.selectedModelId, "anthropic/claude-opus-4.6");
  assert.deepEqual(
    state.models.map(({ id }) => id),
    ["anthropic/claude-opus-4.6", "openai/gpt-5.4"],
  );
});

void test("reads model and reasoning selectors from typed ACP config options", () => {
  const state = readModelState({
    configOptions: [
      {
        id: "model",
        name: "Model",
        category: "model",
        type: "select",
        currentValue: "fast",
        options: [
          { value: "fast", name: "Fast" },
          { value: "smart", name: "Smart", description: "More capable" },
        ],
      },
      {
        id: "fast_mode",
        name: "Fast mode",
        category: "model_config",
        type: "boolean",
        currentValue: true,
        description: "Faster responses at a higher cost.",
      },
      {
        id: "thinking",
        name: "Thinking",
        category: "thought_level",
        type: "select",
        currentValue: "true",
        options: [
          { value: "false", name: "Off" },
          { value: "true", name: "On" },
        ],
      },
      {
        id: "reasoning_effort",
        name: "Reasoning effort",
        category: "model_config",
        type: "select",
        currentValue: "medium",
        options: [
          {
            group: "effort",
            name: "Effort",
            options: [
              { value: "low", name: "Low" },
              { value: "medium", name: "Medium" },
            ],
          },
        ],
      },
    ],
  });

  assert.deepEqual(state, {
    modelConfigId: "model",
    models: [
      { id: "fast", name: "Fast", description: undefined },
      { id: "smart", name: "Smart", description: "More capable" },
    ],
    selectedModelId: "fast",
    reasoningConfigId: "reasoning_effort",
    reasoningOptions: [
      { id: "low", name: "Low", description: undefined },
      { id: "medium", name: "Medium", description: undefined },
    ],
    selectedReasoningId: "medium",
    fastModeConfigId: "fast_mode",
    fastModeEnabled: true,
    fastModeValueType: "boolean",
    fastModeDescription: "Faster responses at a higher cost.",
  });
});
