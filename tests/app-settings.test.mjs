import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  configuredProviderProfiles,
  loadAppPreferences,
  loadPermissionMode,
  loadSidebarWidths,
  loadTerminalHeight,
  providerVersionFromOutput,
  resetAppSettings,
  saveAppPreference,
  savePermissionMode,
  saveSidebarWidth,
  saveTerminalHeight,
  THEME_OPTIONS,
} from "../src/utils/app-settings.ts";

void test("settings writes tolerate unavailable web storage", () => {
  const storage = {
    setItem: () => {
      throw new Error("storage unavailable");
    },
  };

  assert.doesNotThrow(() => savePermissionMode("full", storage));
});

void test("permission mode defaults to restricted and persists valid choices", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  assert.equal(loadPermissionMode(storage), "restricted");
  savePermissionMode("full", storage);
  assert.equal(loadPermissionMode(storage), "full");
  values.set("loopcode.permission-mode", "unexpected");
  assert.equal(loadPermissionMode(storage), "restricted");
});

void test("app preferences use safe defaults and validate persisted values", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  const defaults = {
    colorMode: "system",
    theme: "violet",
    compactSessionRows: false,
    startupBehavior: "last-thread",
    newThreadProject: "selected",
    defaultWorkingFolder: "",
    motionMode: "system",
    interfaceZoom: 100,
    transcriptDensity: "comfortable",
    contentWidth: 720,
    wrapCode: true,
    showMessageTimestamps: false,
    composerSpellcheck: true,
    defaultProviderId: "codex",
    automaticTitleGeneration: true,
    providerModelDefaults: {},
    providerSettings: {},
    titleProviderId: "codex",
    titleModelId: "",
    terminalFontSize: 12,
    terminalScrollback: 5_000,
    sendShortcut: "enter",
  };
  assert.deepEqual(loadAppPreferences(storage), defaults);

  saveAppPreference("colorMode", "light", storage);
  saveAppPreference("theme", "rose", storage);
  saveAppPreference("compactSessionRows", true, storage);
  saveAppPreference("startupBehavior", "new-thread", storage);
  saveAppPreference("motionMode", "reduced", storage);
  saveAppPreference("interfaceZoom", 140, storage);
  saveAppPreference("defaultWorkingFolder", "C:\\work", storage);
  saveAppPreference("automaticTitleGeneration", false, storage);
  saveAppPreference("providerModelDefaults", { codex: "gpt-5" }, storage);
  saveAppPreference(
    "providerSettings",
    {
      codex: {
        enabled: false,
        command: "/opt/codex-acp",
        models: [{ id: "custom-model", name: "Custom model" }],
      },
    },
    storage,
  );
  saveAppPreference("titleProviderId", "pi", storage);
  saveAppPreference("titleModelId", "pi-model", storage);
  saveAppPreference("sendShortcut", "modifier-enter", storage);
  assert.deepEqual(loadAppPreferences(storage), {
    ...defaults,
    colorMode: "light",
    theme: "rose",
    compactSessionRows: true,
    startupBehavior: "new-thread",
    motionMode: "reduced",
    interfaceZoom: 140,
    defaultWorkingFolder: "C:\\work",
    automaticTitleGeneration: false,
    providerModelDefaults: { codex: "gpt-5" },
    providerSettings: {
      codex: {
        enabled: false,
        command: "/opt/codex-acp",
        models: [{ id: "custom-model", name: "Custom model" }],
      },
    },
    titleProviderId: "pi",
    titleModelId: "pi-model",
    sendShortcut: "modifier-enter",
  });

  values.set("loopcode.color-mode", "unexpected");
  values.set("loopcode.theme", "not-a-theme");
  values.set("loopcode.interface-zoom", "500");
  values.set("loopcode.content-width", "200");
  values.set("loopcode.provider-model-defaults", "invalid");
  values.set(
    "loopcode.provider-settings",
    JSON.stringify({
      codex: { enabled: "yes", name: "  ", models: [{ id: "", name: "Missing ID" }] },
      __proto__: { enabled: false },
    }),
  );
  values.set("loopcode.terminal-font-size", "invalid");
  values.set("loopcode.terminal-scrollback", "20");
  assert.equal(loadAppPreferences(storage).colorMode, "system");
  assert.equal(loadAppPreferences(storage).theme, "violet");
  assert.equal(loadAppPreferences(storage).interfaceZoom, 200);
  assert.equal(loadAppPreferences(storage).contentWidth, 600);
  assert.deepEqual(loadAppPreferences(storage).providerModelDefaults, {});
  assert.deepEqual(loadAppPreferences(storage).providerSettings, {});
  assert.equal(loadAppPreferences(storage).terminalFontSize, 12);
  assert.equal(loadAppPreferences(storage).terminalScrollback, 5_000);
});

void test("every built-in theme has a settings preview palette", () => {
  const styles = readFileSync(new URL("../src/styles/base.css", import.meta.url), "utf8");

  for (const theme of THEME_OPTIONS) {
    assert.match(styles, new RegExp(`\\.theme-preview-${theme.id}\\b`));
  }
});

void test("provider settings preserve ACP arguments without mutating defaults", () => {
  const profiles = [{ id: "codex", label: "Codex", command: "npx", args: ["adapter"] }];
  const configured = configuredProviderProfiles(profiles, { codex: { command: "/opt/npx" } });

  assert.equal(configured[0].label, "Codex");
  assert.equal(configured[0].command, "/opt/npx");
  assert.deepEqual(configured[0].args, ["adapter"]);
  assert.equal(profiles[0].label, "Codex");
});

void test("provider versions are read from common CLI output", () => {
  assert.equal(providerVersionFromOutput("codex-cli 0.149.0"), "0.149.0");
  assert.equal(providerVersionFromOutput("claude 2.1.240 (Claude Code)"), "2.1.240");
  assert.equal(providerVersionFromOutput("unknown"), undefined);
});

void test("reset removes preferences without touching workspace history", () => {
  const removed = [];
  resetAppSettings({ removeItem: (key) => removed.push(key) });

  assert.ok(removed.includes("loopcode.color-mode"));
  assert.ok(removed.includes("loopcode.theme"));
  assert.ok(removed.includes("loopcode.default-provider"));
  assert.ok(removed.includes("loopcode.title-provider"));
  assert.ok(removed.includes("loopcode.auto-follow-output"));
  assert.ok(removed.includes("loopcode.composer-autocomplete"));
  assert.ok(removed.includes("loopcode.permission-mode"));
  assert.ok(removed.includes("loopcode.terminal-height"));
  assert.ok(!removed.includes("loopcode.workspace"));
});

void test("sidebar widths retain CSS defaults until resized and persist valid sizes", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  assert.deepEqual(loadSidebarWidths(storage), { left: null, right: null });
  saveSidebarWidth("left", 318, storage);
  saveSidebarWidth("right", 364, storage);
  assert.deepEqual(loadSidebarWidths(storage), { left: 318, right: 364 });

  values.set("loopcode.left-sidebar-width", "20");
  values.set("loopcode.right-sidebar-width", "2000");
  assert.deepEqual(loadSidebarWidths(storage), { left: 190, right: 520 });
});

void test("terminal height has a useful default and clamps persisted values", () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };

  assert.equal(loadTerminalHeight(storage), 260);
  saveTerminalHeight(340, storage);
  assert.equal(loadTerminalHeight(storage), 340);

  values.set("loopcode.terminal-height", "900");
  assert.equal(loadTerminalHeight(storage), 520);
  values.set("loopcode.terminal-height", "invalid");
  assert.equal(loadTerminalHeight(storage), 260);
});
