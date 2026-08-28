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

function newThread() {
  return {
    id: "thread-1",
    messages: [],
    tools: [],
    providers: { codex: {} },
    updatedAt: 0,
  };
}

void test("edit tool calls keep their diffs instead of a raw payload dump", () => {
  const thread = newThread();
  const updates = new SessionUpdateHandler(() => {});

  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call",
    toolCallId: "edit-1",
    title: "Editing files",
    kind: "edit",
    status: "in_progress",
    content: [
      {
        type: "diff",
        path: "/repo/src/app.ts",
        oldText: "const a = 1;\n",
        newText: "const a = 2;\n",
        _meta: { kind: "update" },
      },
    ],
  });

  const [tool] = thread.tools;
  assert.deepEqual(tool.diffs, [
    {
      path: "/repo/src/app.ts",
      oldText: "const a = 1;\n",
      newText: "const a = 2;\n",
      kind: "update",
    },
  ]);
  assert.equal(tool.detail, undefined);
});

void test("command output accumulates across streamed terminal deltas", () => {
  const thread = newThread();
  const updates = new SessionUpdateHandler(() => {});

  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call",
    toolCallId: "cmd-1",
    title: "npm test",
    kind: "execute",
    status: "in_progress",
    content: [{ type: "terminal", terminalId: "cmd-1" }],
    _meta: { terminal_info: { cwd: "/repo", terminal_id: "cmd-1" } },
  });
  for (const data of ["running…\n", "done\n"]) {
    updates.handle(thread, "codex", {
      sessionUpdate: "tool_call_update",
      toolCallId: "cmd-1",
      _meta: { terminal_output_delta: { data, terminal_id: "cmd-1" } },
    });
  }
  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call_update",
    toolCallId: "cmd-1",
    status: "completed",
    rawOutput: { formatted_output: "running…\ndone\n", exit_code: 0 },
    _meta: { terminal_exit: { exit_code: 0, signal: null, terminal_id: "cmd-1" } },
  });

  const [tool] = thread.tools;
  assert.equal(tool.terminal.output, "running…\ndone\n");
  assert.equal(tool.terminal.exitCode, 0);
  assert.equal(tool.status, "completed");
  // The escaped JSON blob must not come back as a fallback once output is structured.
  assert.equal(tool.detail, undefined);
});

void test("command output falls back to the aggregate when no deltas arrived", () => {
  const thread = newThread();
  const updates = new SessionUpdateHandler(() => {});

  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call",
    toolCallId: "cmd-2",
    title: "ls",
    kind: "execute",
    status: "in_progress",
    content: [{ type: "terminal", terminalId: "cmd-2" }],
  });
  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call_update",
    toolCallId: "cmd-2",
    status: "failed",
    rawOutput: { formatted_output: "no such file\n", exit_code: 2 },
  });

  const [tool] = thread.tools;
  assert.equal(tool.terminal.output, "no such file\n");
  assert.equal(tool.terminal.exitCode, 2);
});

void test("a command without terminal content shows its output unescaped", () => {
  const thread = newThread();
  const updates = new SessionUpdateHandler(() => {});

  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call",
    toolCallId: "cmd-3",
    title: "git status",
    kind: "execute",
    status: "completed",
    rawOutput: { formatted_output: "line one\nline two", exit_code: 0 },
  });

  assert.equal(thread.tools[0].detail, "line one\nline two");
  assert.equal(thread.tools[0].terminal, undefined);
});

void test("plan revisions replace the checklist rather than appending copies", () => {
  const thread = newThread();
  const updates = new SessionUpdateHandler(() => {});
  updates.startTurn(thread.id, "codex");

  updates.handle(thread, "codex", {
    sessionUpdate: "plan",
    entries: [
      { content: "Read the code", status: "in_progress", priority: "medium" },
      { content: "Write the fix", status: "pending", priority: "medium" },
    ],
  });
  // A tool call between revisions starts a new message stream; the plan must survive it.
  updates.handle(thread, "codex", {
    sessionUpdate: "tool_call",
    toolCallId: "read-1",
    title: "Read file",
    kind: "read",
    status: "completed",
  });
  updates.handle(thread, "codex", {
    sessionUpdate: "plan",
    entries: [
      { content: "Read the code", status: "completed", priority: "medium" },
      { content: "Write the fix", status: "in_progress", priority: "medium" },
    ],
  });

  assert.equal(thread.tools.filter((tool) => tool.plan).length, 1);
  assert.deepEqual(thread.tools.find((tool) => tool.plan).plan, [
    { content: "Read the code", status: "completed" },
    { content: "Write the fix", status: "in_progress" },
  ]);
  assert.equal(thread.tools.find((tool) => tool.plan).status, "in_progress");
  assert.equal(thread.messages.length, 0);
});

void test("context usage and available commands land on the provider", () => {
  const thread = newThread();
  const updates = new SessionUpdateHandler(() => {});

  updates.handle(thread, "codex", { sessionUpdate: "usage_update", used: 30_000, size: 200_000 });
  updates.handle(thread, "codex", {
    sessionUpdate: "available_commands_update",
    availableCommands: [
      {
        name: "review",
        description: "Review uncommitted changes.",
        input: { hint: "instructions" },
      },
      { name: "compact", description: "Summarize the conversation.", input: null },
    ],
  });

  assert.equal(thread.providers.codex.contextUsed, 30_000);
  assert.equal(thread.providers.codex.contextSize, 200_000);
  assert.deepEqual(thread.providers.codex.commands, [
    { name: "review", description: "Review uncommitted changes.", hint: "instructions" },
    { name: "compact", description: "Summarize the conversation.", hint: undefined },
  ]);
});
