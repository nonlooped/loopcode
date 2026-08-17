import assert from "node:assert/strict";
import test from "node:test";

import { readModelState } from "../src/utils/model-state.ts";

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
    fastModeDescription: "Faster responses at a higher cost.",
  });
});
