import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const overlay = JSON.parse(await readFile("src-tauri/tauri.macos.conf.json", "utf8"));
const bundle = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
const mainWindow = overlay.app.windows.find((window) => window.label === "main");

void test("macOS keeps a transparent native window without Windows acrylic", () => {
  assert.equal(mainWindow.transparent, true);
  assert.equal(mainWindow.backgroundColor.toLowerCase(), "#00000000");
  assert.equal(mainWindow.windowEffects, undefined);
  assert.equal(bundle.bundle.macOS.signingIdentity, "-");
});
