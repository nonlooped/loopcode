import assert from "node:assert/strict";
import test from "node:test";

import { appendMessage } from "../src/utils/messages.ts";

void test("creates an appended message with one timestamp", (context) => {
  let now = 10;
  context.mock.method(Date, "now", () => now++);
  const thread = { messages: [], updatedAt: 0 };

  appendMessage(thread, "message-1", "agent", "Hello");

  assert.equal(thread.messages[0].createdAt, thread.updatedAt);
});

void test("rejects empty ids and role changes for an existing message", () => {
  const thread = {
    messages: [{ id: "message-1", role: "agent", text: "Hello", createdAt: 1 }],
    updatedAt: 1,
  };

  assert.throws(() => appendMessage(thread, "", "agent", " world"), /message id/i);
  assert.throws(() => appendMessage(thread, "message-1", "thought", " world"), /role/i);
  assert.equal(thread.messages[0].text, "Hello");
});
