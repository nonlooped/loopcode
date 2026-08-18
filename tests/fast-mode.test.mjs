import assert from "node:assert/strict";
import test from "node:test";

import { applyFastModeForSelectedModel, fastModeAvailable } from "../src/utils/fast-mode.ts";

void test("uses the selected model's discovered Fast mode capability", () => {
  const provider = {
    status: "disconnected",
    selectedModelId: "gpt-5.5-codex",
    fastModeConfigId: "fast_mode",
    fastModeModelId: "gpt-5.6-codex",
    fastModeOptionsByModel: {
      "gpt-5.5-codex": {
        configId: "fast_mode",
        enabled: false,
        description: "Faster responses at a higher cost.",
      },
      "gpt-5.6-codex": {
        configId: "fast_mode",
        enabled: true,
      },
    },
    models: [],
    reasoningOptions: [],
  };

  applyFastModeForSelectedModel(provider);

  assert.equal(provider.fastModeModelId, "gpt-5.5-codex");
  assert.equal(provider.fastModeEnabled, false);
  assert.equal(fastModeAvailable(provider), true);
});
