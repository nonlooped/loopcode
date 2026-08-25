import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { classifyChangedPaths } from "../scripts/ci-paths.mjs";

const ciWorkflow = readFileSync(".github/workflows/ci.yml", "utf8");
const releaseWorkflow = readFileSync(".github/workflows/release.yml", "utf8");
const dependabot = readFileSync(".github/dependabot.yml", "utf8");
const toolchain = readFileSync("rust-toolchain.toml", "utf8");

void test("path filters treat workflow YAML as non-native and skip nightlies", () => {
  assert.deepEqual(classifyChangedPaths(".github/workflows/ci.yml\nCONTRIBUTING.md\n"), {
    native: false,
    app: false,
  });
  assert.deepEqual(classifyChangedPaths("src/app.css\n"), { native: false, app: true });
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

void test("CI compiles rust only for native paths and tests Windows", () => {
  assert.match(ciWorkflow, /node scripts\/ci-paths\.mjs/);
  assert.match(ciWorkflow, /shared-key: linux-rust-ci/);
  assert.match(ciWorkflow, /shared-key: windows-rust-ci/);
  assert.match(ciWorkflow, /cargo clippy --locked --all-targets --manifest-path src-tauri\/Cargo\.toml -- -D warnings/);
  assert.match(ciWorkflow, /cargo test --locked --manifest-path src-tauri\/Cargo\.toml/);
  assert.doesNotMatch(ciWorkflow, /cargo check --locked/);
  assert.equal([...ciWorkflow.matchAll(/rustup toolchain install --no-self-update/g)].length, 2);
  assert.doesNotMatch(ciWorkflow, /dtolnay\/rust-toolchain/);
  assert.match(ciWorkflow, /needs\.changes\.outputs\.app == 'true'/);
  assert.match(ciWorkflow, /gh workflow run release\.yml --ref "\$source_tag"/);
  assert.doesNotMatch(ciWorkflow, /--ref master -f channel=nightly/);
});

void test("release workflow isolates nightly concurrency and cache from CI", () => {
  assert.match(releaseWorkflow, /group: release-\$\{\{ inputs\.channel \|\| 'stable' \}\}/);
  assert.match(releaseWorkflow, /cancel-in-progress: \$\{\{ inputs\.channel == 'nightly' \}\}/);
  assert.match(releaseWorkflow, /cache-key: linux-rust-release/);
  assert.match(releaseWorkflow, /cache-key: windows-rust-release/);
  assert.equal([...releaseWorkflow.matchAll(/rustup toolchain install --no-self-update/g)].length, 1);
  assert.doesNotMatch(releaseWorkflow, /dtolnay\/rust-toolchain/);
  assert.match(dependabot, /package-ecosystem: github-actions/);
  assert.match(toolchain, /channel = "1\.97\.1"/);
  assert.match(toolchain, /clippy/);
  assert.match(toolchain, /profile = "minimal"/);
});
