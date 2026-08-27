import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import test from "node:test";
import { classifyChangedPaths } from "../scripts/ci-paths.mjs";

void test("path filters include app, native, dependency, and root files", () => {
  assert.deepEqual(classifyChangedPaths(".github/workflows/ci.yml\n"), {
    native: false,
    app: false,
  });
  assert.deepEqual(classifyChangedPaths("CONTRIBUTING.md\n"), { native: false, app: true });
  assert.deepEqual(classifyChangedPaths("src/app.css\n"), { native: false, app: true });
  assert.deepEqual(classifyChangedPaths("package-lock.json\n"), { native: false, app: true });
  assert.deepEqual(classifyChangedPaths("src-tauri/src/lib.rs\n"), { native: true, app: true });
  assert.deepEqual(classifyChangedPaths("rust-toolchain.toml\n"), { native: true, app: true });
});

void test("ci-paths writes GitHub output on stdin", () => {
  const output = execFileSync(process.execPath, ["scripts/ci-paths.mjs"], {
    input: "src/app.css\n.github/workflows/ci.yml\n",
    encoding: "utf8",
  });
  assert.equal(output, "native=false\napp=true\n");
});
