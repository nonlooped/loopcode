import assert from "node:assert/strict";
import test from "node:test";

import { gitDiffLines } from "../src/utils/git-diff.ts";

void test("classifies unified diff lines for the changes panel", () => {
  assert.deepEqual(
    gitDiffLines({
      hunks: ["@@ -1 +1 @@\n-old\n+new\n"],
      binary: false,
      tooLarge: false,
    }),
    [
      { kind: "hunk", text: "@@ -1 +1 @@" },
      { kind: "del", text: "-old" },
      { kind: "add", text: "+new" },
    ],
  );
});
