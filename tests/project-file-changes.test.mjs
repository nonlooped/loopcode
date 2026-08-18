import assert from "node:assert/strict";
import test from "node:test";

import { changedParentDirectories } from "../src/utils/project-file-changes.ts";

void test("maps changes to their immediate parent without widening nested noise", () => {
  assert.deepEqual(
    changedParentDirectories("C:\\work", [
      "C:\\work\\README.md",
      "C:\\work\\target\\debug\\build.log",
      "C:\\elsewhere\\ignored.txt",
    ]),
    ["c:/work", "c:/work/target/debug"],
  );
});
