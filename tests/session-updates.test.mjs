import assert from "node:assert/strict";
import test from "node:test";

import { SessionUpdateHandler } from "../src/services/session-updates.ts";

void test("agent chunks share a stream when startTurn was missed", (context) => {
  let now = 1;
  context.mock.method(Date, "now", () => now++);
  const thread = {
    id: "thread-1",
    messages: [],
    tools: [],
    providers: { codex: {} },
    updatedAt: 0,
  };
  const updates = new SessionUpdateHandler(() => {});

  for (const text of ["Hello", " world"]) {
    updates.handle(thread, "codex", {
      sessionUpdate: "agent_message_chunk",
      content: { type: "text", text },
    });
  }

  assert.equal(thread.messages.length, 1);
  assert.equal(thread.messages[0].text, "Hello world");
});
