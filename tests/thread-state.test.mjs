import assert from "node:assert/strict";
import test from "node:test";

import { findReusableEmptyThread } from "../src/utils/thread-state.ts";

function candidate(overrides = {}) {
  return {
    id: "empty",
    cwd: "C:\\workspace",
    projectId: "project-1",
    messages: [],
    tools: [],
    draft: "",
    settled: false,
    ...overrides,
  };
}

void test("returns an empty thread in the requested project", () => {
  const empty = candidate();
  assert.equal(
    findReusableEmptyThread(
      [candidate({ id: "other-project", projectId: "project-2" }), empty],
      { cwd: "C:\\workspace", projectId: "project-1" },
      () => false,
    ),
    empty,
  );
});

void test("rejects threads containing user state or belonging to another target", () => {
  const occupied = [
    candidate({ id: "message", messages: [{ id: "m1" }] }),
    candidate({ id: "tool", tools: [{ id: "t1" }] }),
    candidate({ id: "draft", draft: "Keep this draft" }),
    candidate({ id: "attachment" }),
    candidate({ id: "archived", settled: true }),
    candidate({ id: "other-folder", cwd: "C:\\elsewhere" }),
  ];

  assert.equal(
    findReusableEmptyThread(
      occupied,
      { cwd: "C:\\workspace", projectId: "project-1" },
      (threadId) => threadId === "attachment",
    ),
    undefined,
  );
});
