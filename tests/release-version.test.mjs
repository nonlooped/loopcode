import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { releaseNotes } from "../scripts/release-notes.mjs";

const version = JSON.parse(readFileSync("package.json", "utf8")).version;

void test("release version check accepts the matching tag and rejects another tag", () => {
  const options = { stdio: "pipe" };
  assert.doesNotThrow(() =>
    execFileSync(process.execPath, ["scripts/check-versions.mjs", `v${version}`], options),
  );
  assert.throws(() =>
    execFileSync(process.execPath, ["scripts/check-versions.mjs", "v0.0.0-invalid"], options),
  );
});

void test("release notes use the matching top changelog section", () => {
  const changelog = "## [1.2.3] - 2026-08-25\n\n- Current\n\n## [1.2.2] - 2026-08-24\n\n- Old\n";
  assert.equal(releaseNotes(changelog, "v1.2.3"), "## [1.2.3] - 2026-08-25\n\n- Current\n");
  assert.throws(() => releaseNotes(changelog, "v1.2.2"));
});
