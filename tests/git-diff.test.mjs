import assert from "node:assert/strict";
import test from "node:test";

import { DiffFile } from "@git-diff-view/core";

import { gitDiffViewData } from "../src/utils/git-diff.ts";

void test("wraps Git hunks in the unified headers required by git-diff-view", () => {
  const data = gitDiffViewData(
    {
      path: "src/example.ts",
      oldPath: null,
      status: "modified",
      staged: false,
      unstaged: true,
    },
    {
      hunks: ["@@ -1 +1 @@\n-old\n+new\n"],
      binary: false,
      tooLarge: false,
    },
  );
  const file = new DiffFile(data.oldFile.fileName, "", data.newFile.fileName, "", data.hunks);
  file.initRaw();
  file.buildUnifiedDiffLines();

  assert.ok(file.unifiedLineLength > 0);
  assert.equal(file.additionLength, 1);
  assert.equal(file.deletionLength, 1);
});
