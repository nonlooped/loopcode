import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { releaseNotes } from "../scripts/release-notes.mjs";

const version = JSON.parse(readFileSync("package.json", "utf8")).version;
const releaseWorkflow = readFileSync(".github/workflows/release.yml", "utf8");
const versionSetter = readFileSync("scripts/set-version.mjs", "utf8");

void test("release version check accepts the matching tag and rejects another tag", () => {
  const options = { stdio: "pipe" };
  assert.doesNotThrow(() =>
    execFileSync(process.execPath, ["scripts/check-versions.mjs", `v${version}`], options),
  );
  assert.throws(() =>
    execFileSync(process.execPath, ["scripts/check-versions.mjs", "v0.0.0-invalid"], options),
  );
});

void test("version setter updates stable package metadata without platform wrappers", () => {
  assert.match(versionSetter, /\["package\.json", "package-lock\.json"\]/);
  assert.match(versionSetter, /^if \(!version \|\| !\/\^\\d\+\\.\\d\+\\.\\d\+\$\//m);
  assert.doesNotMatch(versionSetter, /npm\.cmd|execFileSync\(npm|nightly/);
});

void test("release notes use the matching top changelog section", () => {
  const changelog = "## [1.2.3] - 2026-08-25\n\n- Current\n\n## [1.2.2] - 2026-08-24\n\n- Old\n";
  assert.equal(releaseNotes(changelog, "v1.2.3"), "## [1.2.3] - 2026-08-25\n\n- Current\n");
  assert.throws(() => releaseNotes(changelog, "v1.2.2"));
});

void test("release workflow is stable-only and reports failures", () => {
  assert.match(releaseWorkflow, /RELEASE_SHA: \$\{\{ github\.sha \}\}/);
  assert.match(releaseWorkflow, /\[\[ "\$RELEASE_SHA" =~ \^\[0-9a-f\]\{40\}\$ \]\]/);
  assert.match(releaseWorkflow, /git merge-base --is-ancestor "\$RELEASE_SHA" origin\/master/);
  assert.match(
    releaseWorkflow,
    /gh run list --workflow ci\.yml --branch master --commit "\$RELEASE_SHA"[\s\S]*--event push --status success/,
  );
  assert.match(releaseWorkflow, /group: release/);
  assert.match(releaseWorkflow, /^  report-failure:/m);
  assert.match(releaseWorkflow, /gh issue create[\s\S]*--assignee "\$GITHUB_REPOSITORY_OWNER"/);
  assert.doesNotMatch(releaseWorkflow, /nightly|RELEASE_CHANNEL/);
  assert.doesNotMatch(releaseWorkflow.split("  bundle:")[0], /rustup toolchain install/);
});
