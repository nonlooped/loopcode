import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

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

void test("release notes require the target version, summary, and highlights", () => {
  const options = { stdio: "pipe" };
  const path = join(mkdtempSync(join(tmpdir(), "loopcode-release-")), "notes.md");
  const check = (tag) =>
    execFileSync(process.execPath, ["scripts/check-release-notes.mjs", tag, path], options);

  writeFileSync(
    path,
    "# LoopCode v1.2.3: Useful title\n\nA useful summary.\n\n## Highlights\n\n- Useful change.\n",
  );
  assert.doesNotThrow(() => check("v1.2.3"));
  assert.throws(() => check("v1.2.4"));

  writeFileSync(path, "# LoopCode v1.2.3: Useful title\n\n## Highlights\n\n- Useful change.\n");
  assert.throws(() => check("v1.2.3"));

  writeFileSync(
    path,
    "# LoopCode v1.2.3: Useful title\n\nA useful summary.\n\n## Highlights\n\nNothing listed.\n",
  );
  assert.throws(() => check("v1.2.3"));
});
