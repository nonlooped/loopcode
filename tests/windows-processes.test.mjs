import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const nativeSource = await readFile(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");

void test("Git metadata commands do not create a console window on Windows", () => {
  const gitCommand = nativeSource.match(/fn git_command\(\) -> Command \{(?<body>[\s\S]*?)\n\}/);

  assert.ok(gitCommand, "get_git_branch should use the shared Git command builder");
  assert.match(gitCommand.groups.body, /cfg\(target_os = "windows"\)/);
  assert.match(gitCommand.groups.body, /creation_flags\(0x0800_0000\)/);
});
