import assert from "node:assert/strict";
import test from "node:test";

import {
  desktopPlatform,
  providerDefinitions,
  providerSupportsPlatform,
} from "../src/config/provider-definitions.ts";
import {
  providerCanToggle,
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
  assert.equal(desktopPlatform("darwin"), "macos");
  assert.equal(desktopPlatform("macos"), "macos");
  assert.equal(desktopPlatform("windows"), "windows");
  assert.equal(desktopPlatform("linux"), "linux");
  assert.equal(desktopPlatform(undefined), "linux");
  assert.ok(providerDefinitions.every((profile) => profile.platforms.includes("macos")));
  assert.equal(providerSupportsPlatform({ platforms: ["linux", "macos"] }, "macos"), true);
  assert.equal(providerSupportsPlatform({ platforms: ["linux", "macos"] }, "windows"), false);
  assert.equal(unavailableReason(new Error("Authentication required")), "authentication");
  assert.equal(
    unavailableReason(new Error("Could not start codex: No such file or directory")),
    "missing-executable",
  );
  assert.equal(unavailableReason(new Error("Invalid ACP response")), "discovery");
});

void test("an unavailable saved default falls back", () => {
  const catalogs = {
    codex: ready("codex-model"),
    retired: {
      status: "unavailable",
      models: [],
      reasoningOptions: [],
      unavailableReason: "authentication",
      error: "Authentication required",
    },
  };

  assert.equal(readyProviderId("retired", providerDefinitions, catalogs), "codex");
});

void test("title generation resolves only a ready safe provider and model", () => {
  const profiles = [
    { id: "codex", titleGeneration: true },
    { id: "untitled", titleGeneration: false },
  ];
  const catalogs = {
    codex: ready("codex-model"),
    untitled: ready("untitled-model"),
  };

  assert.equal(
    titleGenerationSelection({ profileId: "codex", modelId: "codex-model" }, profiles, catalogs)
      ?.model.id,
    "codex-model",
  );
  assert.equal(
    titleGenerationSelection(
      { profileId: "untitled", modelId: "untitled-model" },
      profiles,
      catalogs,
    ),
    undefined,
  );
});

void test("provider display status distinguishes ready, disabled, and missing CLIs", () => {
  const missing = {
    status: "unavailable",
    models: [],
    reasoningOptions: [],
    unavailableReason: "missing-executable",
    error: "not found",
  };

  assert.equal(providerDisplayStatus(true, ready("codex-model")), "Authenticated");
  assert.equal(providerDisplayStatus(true, undefined), "Not logged in");
  assert.equal(providerDisplayStatus(true, missing), "Not installed");
  assert.equal(providerDisplayStatus(false, missing), "Disabled");
  assert.equal(providerVersionLabel(missing), "");
});

void test("only providers with a ready catalog can be toggled", () => {
  assert.equal(providerCanToggle(ready("codex-model")), true);
  assert.equal(providerCanToggle(undefined), false);
});
