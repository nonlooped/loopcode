import assert from "node:assert/strict";
import test from "node:test";

import { documentTypeForPath } from "../src/utils/file-preview.ts";

void test("recognizes HTML and Markdown document extensions", () => {
  assert.equal(documentTypeForPath("index.html"), "html");
  assert.equal(documentTypeForPath("ARCHIVE.HTM"), "html");
  assert.equal(documentTypeForPath("README.md"), "markdown");
  assert.equal(documentTypeForPath("CONTRIBUTING.markdown"), "markdown");
  assert.equal(documentTypeForPath("notes.txt"), null);
});
