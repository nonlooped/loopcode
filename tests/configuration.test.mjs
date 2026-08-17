import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const packageManifest = JSON.parse(await readFile(new URL("../package.json", import.meta.url)));
const tauriConfig = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url)),
);
const mainCapability = JSON.parse(
  await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url)),
);
const appSource = await readFile(new URL("../src/App.svelte", import.meta.url), "utf8");

void test("the desktop window uses the intended native acrylic shell", () => {
  const mainWindow = tauriConfig.app.windows.find((window) => window.label === "main");
  assert.equal(mainWindow.decorations, false);
  assert.equal(mainWindow.transparent, true);
  assert.equal(mainWindow.shadow, true);
  assert.deepEqual(mainWindow.windowEffects.effects, ["acrylic"]);
});

void test("the intercepted close request force-destroys the window after cleanup", () => {
  const closeApp = appSource.match(/async function closeApp\(\) \{(?<body>[\s\S]*?)\n  \}/);

  assert.ok(closeApp, "App.svelte should define the close cleanup flow");
  assert.match(closeApp.groups.body, /await appWindow\.destroy\(\)/);
  assert.doesNotMatch(closeApp.groups.body, /await appWindow\.close\(\)/);
  assert.ok(mainCapability.permissions.includes("core:window:allow-destroy"));
});

void test("protocol dependencies are pinned and Mermaid is not installed", () => {
  assert.equal(packageManifest.dependencies["@agentclientprotocol/sdk"], "1.3.0");
  assert.equal(packageManifest.dependencies.zod, "4.4.3");
  assert.equal("mermaid" in packageManifest.dependencies, false);
});
