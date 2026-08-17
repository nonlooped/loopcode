import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const packageManifest = JSON.parse(await readFile(new URL("../package.json", import.meta.url)));
const tauriConfig = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url)),
);

void test("the desktop window uses the intended native acrylic shell", () => {
  const mainWindow = tauriConfig.app.windows.find((window) => window.label === "main");
  assert.equal(mainWindow.decorations, false);
  assert.equal(mainWindow.transparent, true);
  assert.equal(mainWindow.shadow, true);
  assert.deepEqual(mainWindow.windowEffects.effects, ["acrylic"]);
});

void test("protocol dependencies are pinned and Mermaid is not installed", () => {
  assert.equal(packageManifest.dependencies["@agentclientprotocol/sdk"], "1.3.0");
  assert.equal(packageManifest.dependencies.zod, "4.4.3");
  assert.equal("mermaid" in packageManifest.dependencies, false);
});
