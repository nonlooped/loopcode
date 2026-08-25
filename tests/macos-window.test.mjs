import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const overlay = JSON.parse(await readFile("src-tauri/tauri.macos.conf.json", "utf8"));
const bundle = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
const cargo = await readFile("src-tauri/Cargo.toml", "utf8");
const mainWindow = overlay.app.windows.find((window) => window.label === "main");

void test("macOS uses vibrancy instead of Windows acrylic", () => {
  assert.equal(overlay.app.macOSPrivateApi, true);
  assert.equal(bundle.app.macOSPrivateApi, true);
  assert.match(cargo, /macos-private-api/);
  assert.equal(mainWindow.transparent, true);
  assert.equal(mainWindow.backgroundColor.toLowerCase(), "#00000000");
  assert.deepEqual(mainWindow.windowEffects?.effects, ["underWindowBackground"]);
  assert.equal(bundle.bundle.macOS.signingIdentity, "-");
});
