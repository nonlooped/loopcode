import assert from "node:assert/strict";
import test from "node:test";

import { timelineEntries } from "../src/utils/timeline.ts";

function thread(status = "ready") {
  return {
    id: "thread-1",
    title: "Thread",
    profileId: "codex",
    cwd: "C:\\workspace",
    messages: [
      { id: "user", role: "user", text: "Do work", createdAt: 1 },
      { id: "thought", role: "thought", text: "Planning", createdAt: 2 },
      { id: "intermediate", role: "agent", text: "Checking", createdAt: 4 },
      { id: "final", role: "agent", text: "Done", createdAt: 6 },
    ],
    tools: [
      {
        id: "tool",
        title: "Read file",
        kind: "read",
        status: "completed",
        locations: [],
        createdAt: 3,
      },
    ],
    draft: "",
    providers: {
      codex: { status, models: [], reasoningOptions: [] },
    },
    updatedAt: 6,
    settled: false,
    projectId: null,
  };
}

void test("collapses turn work while leaving the terminal agent response visible", () => {
  const entries = timelineEntries(thread());
  assert.equal(entries.length, 3);
  assert.equal(entries[0].type, "message");
  assert.equal(entries[0].message.id, "user");
  assert.equal(entries[1].type, "work");
  assert.deepEqual(
    entries[1].entries.map((entry) => (entry.type === "tool" ? entry.tool.id : entry.message.id)),
    ["thought", "tool", "intermediate"],
  );
  assert.equal(entries[2].type, "message");
  assert.equal(entries[2].message.id, "final");
});

void test("keeps all agent output in the active work group while running", () => {
  const entries = timelineEntries(thread("running"));
  const work = entries.find((entry) => entry.type === "work");
  assert.equal(work.active, true);
  assert.equal(work.entries.at(-1).message.id, "final");
});
