import assert from "node:assert/strict";
import test from "node:test";

import {
  providerDefinitions,
  providerSupportsPlatform,
} from "../src/config/provider-definitions.ts";
import {
  providerDisplayStatus,
  providerVersionLabel,
  readyProviderId,
  titleGenerationSelection,
  unavailableReason,
} from "../src/utils/provider-availability.ts";

const ready = (modelId) => ({
  status: "ready",
  models: [{ id: modelId, name: modelId }],
  selectedModelId: modelId,
  reasoningOptions: [],
});

void test("provider availability covers platform, discovery, executable, and auth failures", () => {
  const fx = providerDefinitions.find((profile) => profile.id === "fx");
  assert.ok(fx);
  assert.equal(providerSupportsPlatform(fx, "linux"), true);
  assert.equal(providerSupportsPlatform(fx, "windows"), false);
  assert.equal(unavailableReason(new Error("Authentication required")), "authentication");
  assert.equal(
    unavailableReason(new Error("Could not start grok: No such file or directory")),
    "missing-executable",
  );
  assert.equal(unavailableReason(new Error("Invalid ACP response")), "discovery");
  const pi = providerDefinitions.find((profile) => profile.id === "pi");
  assert.ok(pi);
  assert.match(pi.command, /^npx/);
  assert.match(pi.args.join(" "), /@victor-software-house\/pi-acp@/);
});

void test("an unavailable saved default falls back", () => {
  const catalogs = {
    codex: ready("codex-model"),
    grok: {
      status: "unavailable",
      models: [],
      reasoningOptions: [],
      unavailableReason: "authentication",
      error: "Authentication required",
    },
  };

  assert.equal(readyProviderId("grok", providerDefinitions, catalogs), "codex");
});

void test("title generation resolves only a ready safe provider and model", () => {
  const catalogs = {
    pi: ready("pi-model"),
    grok: ready("grok-model"),
  };

  assert.equal(
    titleGenerationSelection(
      { profileId: "pi", modelId: "pi-model" },
      providerDefinitions,
      catalogs,
    )?.model.id,
    "pi-model",
  );
  assert.equal(
    titleGenerationSelection(
      { profileId: "grok", modelId: "grok-model" },
      providerDefinitions,
      catalogs,
    ),
    undefined,
  );
});

void test("provider display status distinguishes auth, connections, and missing CLIs", () => {
  const missing = {
    status: "unavailable",
    models: [],
    reasoningOptions: [],
    unavailableReason: "missing-executable",
    error: "not found",
  };

  assert.equal(
    providerDisplayStatus("claude", true, ready("claude-model"), false),
    "Not logged in",
  );
  assert.equal(providerDisplayStatus("claude", true, ready("claude-model"), true), "Authenticated");
  for (const profileId of ["fx", "opencode", "pi"]) {
    assert.equal(providerDisplayStatus(profileId, true, ready(`${profileId}-model`)), "Connected");
  }
  assert.equal(providerDisplayStatus("grok", true, missing), "Not installed");
  assert.equal(providerDisplayStatus("grok", false, missing), "Disabled");
  assert.equal(providerVersionLabel(missing), "");
});
