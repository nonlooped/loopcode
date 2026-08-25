import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { nightlyVersion } from "../scripts/nightly-version.mjs";
import { nightlyNotes, releaseNotes } from "../scripts/release-notes.mjs";

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

void test("nightly version is the stable base plus date and run", () => {
  assert.equal(nightlyVersion("0.8.0", "20260825", "12"), "0.8.0-nightly.20260825.12");
  assert.throws(() => nightlyVersion("0.8.0-nightly.1", "20260825", "12"));
});

void test("nightly notes list commits since the previous tag", () => {
  assert.equal(
    nightlyNotes({
      tag: "v0.8.0-nightly.20260825.12",
      shortSha: "abc1234",
      previousTag: "v0.8.0",
      commits: ["feat(ui): restore acrylic chrome", "fix(ui): keep web preview shell opaque"],
    }),
    "Nightly `v0.8.0-nightly.20260825.12` (abc1234). Not a stable release.\n\n## Changes since v0.8.0\n\n- feat(ui): restore acrylic chrome\n- fix(ui): keep web preview shell opaque\n",
  );
});
