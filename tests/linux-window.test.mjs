import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const config = JSON.parse(await readFile("src-tauri/tauri.linux.conf.json", "utf8"));
const mainWindow = config.app.windows.find((window) => window.label === "main");

void test("Linux uses an opaque shell background", () => {
  assert.equal(mainWindow.transparent, false);
  assert.equal(mainWindow.backgroundColor.toLowerCase(), "#17181b");
});
