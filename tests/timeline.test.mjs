import assert from "node:assert/strict";
import test from "node:test";

import {
  formatElapsedDuration,
  isStreamingMessage,
  timelineEntries,
} from "../src/utils/timeline.ts";

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
      codex: {
        connectionStatus: status === "running" ? "ready" : status,
        turnStatus: status === "running" ? "running" : "idle",
        models: [],
        reasoningOptions: [],
      },
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
  assert.equal(entries[1].startedAt, 1);
  assert.equal(entries[1].durationMs, 5);
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
  assert.equal(work.startedAt, 1);
  assert.equal(work.durationMs, null);
  assert.equal(work.entries.at(-1).message.id, "final");
});

void test("keeps the substantive response visible when a completed turn ends with whitespace", () => {
  const value = thread("running");
  value.messages.push(
    { id: "trailing", role: "agent", text: "\n", createdAt: 7 },
    { id: "follow-up", role: "user", text: "Continue", createdAt: 8 },
  );
  value.updatedAt = 8;

  const entries = timelineEntries(value);
  assert.ok(entries.some((entry) => entry.type === "message" && entry.message.id === "final"));
});

void test("formats work duration labels", () => {
  assert.equal(formatElapsedDuration(8_000), "8s");
  assert.equal(formatElapsedDuration(62_000), "1m 2s");
  assert.equal(formatElapsedDuration(120_000), "2m");
});

void test("does not mark the previous turn's response as streaming while the next turn starts", () => {
  const value = thread("running");
  const previousResponse = value.messages.find((message) => message.id === "final");
  value.messages.push({ id: "follow-up", role: "user", text: "Continue", createdAt: 7 });
  value.updatedAt = 7;

  assert.equal(isStreamingMessage(value, previousResponse), false);

  const currentResponse = { id: "current", role: "agent", text: "Continuing", createdAt: 8 };
  value.messages.push(currentResponse);
  value.updatedAt = 8;

  assert.equal(isStreamingMessage(value, previousResponse), false);
  assert.equal(isStreamingMessage(value, currentResponse), true);
});
