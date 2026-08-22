import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
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
