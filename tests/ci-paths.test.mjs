import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { classifyChangedPaths } from "../scripts/ci-paths.mjs";

const ciWorkflow = readFileSync(".github/workflows/ci.yml", "utf8");
const releaseWorkflow = readFileSync(".github/workflows/release.yml", "utf8");
const dependabot = readFileSync(".github/dependabot.yml", "utf8");
const checkVersions = readFileSync("scripts/check-versions.mjs", "utf8");
const toolchain = readFileSync("rust-toolchain.toml", "utf8");

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

void test("CI compiles rust only for native paths and tests Windows and macOS", () => {
  assert.match(ciWorkflow, /^  node:/m);
  assert.doesNotMatch(ciWorkflow, /^  frontend:/m);
  assert.match(
    ciWorkflow,
    /group: \$\{\{ github\.workflow \}\}-\$\{\{ github\.event_name == 'push' && github\.sha \|\| github\.ref \}\}/,
  );
  assert.match(ciWorkflow, /cancel-in-progress: \$\{\{ github\.event_name == 'pull_request' \}\}/);
  assert.match(ciWorkflow, /node scripts\/ci-paths\.mjs/);
  assert.match(ciWorkflow, /shared-key: linux-rust-ci/);
  assert.match(ciWorkflow, /shared-key: windows-rust-ci/);
  assert.match(ciWorkflow, /shared-key: macos-rust-ci/);
  assert.match(
    ciWorkflow,
    /cargo clippy --locked --all-targets --manifest-path src-tauri\/Cargo\.toml -- -D warnings/,
  );
  assert.match(ciWorkflow, /cargo test --locked --manifest-path src-tauri\/Cargo\.toml/);
  assert.doesNotMatch(ciWorkflow, /cargo check --locked/);
  assert.equal([...ciWorkflow.matchAll(/rustup toolchain install --no-self-update/g)].length, 3);
  assert.doesNotMatch(ciWorkflow, /dtolnay\/rust-toolchain/);
  assert.match(ciWorkflow, /gh workflow run release\.yml --ref "\$TAG"/);
  assert.doesNotMatch(ciWorkflow, /nightly/);
  assert.doesNotMatch(checkVersions, /execFileSync|"cargo"/);
  assert.equal([...ciWorkflow.matchAll(/npm install --global npm@12\.0\.2/g)].length, 1);
  assert.doesNotMatch(ciWorkflow, /npm install --global npm@12\s/);
});

void test("release workflow uses separate release caches", () => {
  assert.match(
    releaseWorkflow,
    /group: release-\$\{\{ github\.event_name == 'schedule' && 'nightly' \|\| 'stable' \}\}/,
  );
  assert.match(releaseWorkflow, /cancel-in-progress: false/);
  assert.match(releaseWorkflow, /cache-key: linux-rust-release/);
  assert.match(releaseWorkflow, /cache-key: windows-rust-release/);
  assert.match(releaseWorkflow, /cache-key: macos-rust-release/);
  assert.match(releaseWorkflow, /cache-on-failure: true/);
  assert.match(releaseWorkflow, /platform: macos-latest/);
  assert.match(releaseWorkflow, /rustup target add aarch64-apple-darwin x86_64-apple-darwin/);
  assert.match(releaseWorkflow, /--target universal-apple-darwin/);
  assert.match(releaseWorkflow, /target\/universal-apple-darwin\/release\/bundle\/\*\*\/\*\.dmg/);
  assert.match(releaseWorkflow, /codesign --verify --deep --strict/);
  assert.match(releaseWorkflow, /lipo -archs/);
  assert.doesNotMatch(releaseWorkflow, /app\.tar\.gz|\*\.sig/);
  assert.equal(
    [...releaseWorkflow.matchAll(/rustup toolchain install --no-self-update/g)].length,
    1,
  );
  assert.doesNotMatch(releaseWorkflow, /dtolnay\/rust-toolchain/);
  assert.match(dependabot, /package-ecosystem: github-actions/);
  assert.match(dependabot, /package-ecosystem: npm[\s\S]*directory: \//);
  assert.match(dependabot, /package-ecosystem: cargo[\s\S]*directory: \/src-tauri/);
  assert.equal([...releaseWorkflow.matchAll(/npm install --global npm@12\.0\.2/g)].length, 1);
  assert.doesNotMatch(releaseWorkflow, /npm install --global npm@12\s/);
  assert.match(toolchain, /channel = "1\.97\.1"/);
  assert.match(toolchain, /clippy/);
  assert.match(toolchain, /profile = "minimal"/);
});
