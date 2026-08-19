import assert from "node:assert/strict";
import test from "node:test";

import { AcpConnection } from "../src/services/acp.ts";

function fakeTransport({ loadSession = false, resumeSession = false, promptMode = "normal" } = {}) {
  let onEvent;
  let session = 0;
  const sent = [];
  const configOptions = [
    {
      id: "model",
      name: "Model",
      category: "model",
      type: "select",
      currentValue: "model-1",
      options: [{ value: "model-1", name: "Model 1" }],
    },
  ];

  const transport = {
    async launch(_request, receive) {
      onEvent = receive;
      return "harness-1";
    },
    async send(_harnessId, message) {
      sent.push(message);
      if (message.method === "initialize") {
        reply(message.id, {
          protocolVersion: 1,
          agentCapabilities: {
            loadSession,
            sessionCapabilities: resumeSession ? { resume: {} } : {},
          },
          authMethods: [],
        });
      } else if (message.method === "session/new") {
        session += 1;
        reply(message.id, { sessionId: `session-${session}`, configOptions });
      } else if (message.method === "cursor/list_available_models") {
        reply(message.id, { models: [{ value: "claude-opus-5", name: "Claude Opus 5" }] });
      } else if (message.method === "session/load") {
        onEvent({
          event: "rpc",
          data: {
            message: {
              jsonrpc: "2.0",
              method: "session/update",
              params: {
                sessionId: message.params.sessionId,
                update: {
                  sessionUpdate: "agent_message_chunk",
                  content: { type: "text", text: "Private agent history" },
                },
              },
            },
          },
        });
        reply(message.id, { configOptions });
      } else if (message.method === "session/resume") {
        reply(message.id, { configOptions });
      } else if (message.method === "session/set_config_option") {
        reply(message.id, { configOptions });
      } else if (message.method === "session/prompt") {
        if (promptMode === "pending") return;
        if (promptMode === "error") {
          replyError(message.id, -32603, "Internal error", { detail: "turn already active" });
          return;
        }
        onEvent({
          event: "rpc",
          data: {
            message: {
              jsonrpc: "2.0",
              method: "session/update",
              params: {
                sessionId: message.params.sessionId,
                update:
                  promptMode === "active-tool"
                    ? {
                        sessionUpdate: "tool_call",
                        toolCallId: "tool-1",
                        title: "Editing files",
                        kind: "edit",
                        status: "in_progress",
                      }
                    : {
                        sessionUpdate: "agent_message_chunk",
                        content: { type: "text", text: "Done" },
                      },
              },
            },
          },
        });
        reply(message.id, { stopReason: "end_turn" });
      }
    },
    async stop() {},
  };

  function reply(id, result) {
    queueMicrotask(() => {
      onEvent({ event: "rpc", data: { message: { jsonrpc: "2.0", id, result } } });
    });
  }

  function replyError(id, code, message, data) {
    queueMicrotask(() => {
      onEvent({
        event: "rpc",
        data: { message: { jsonrpc: "2.0", id, error: { code, message, data } } },
      });
    });
  }

  return {
    transport,
    sent,
    requestPermission() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 99,
            method: "session/request_permission",
            params: {
              sessionId: "session-1",
              toolCall: {
                toolCallId: "tool-1",
                title: "Run tests",
                rawInput: { command: "npm test" },
              },
              options: [{ optionId: "allow", name: "Allow", kind: "allow_once" }],
            },
          },
        },
      });
    },
    requestElicitation() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 101,
            method: "elicitation/create",
            params: {
              sessionId: "session-1",
              mode: "form",
              message: "Which database should we use?",
              requestedSchema: {
                type: "object",
                properties: {
                  database: {
                    type: "string",
                    title: "Storage",
                    description: "Which database should we use?",
                    oneOf: [
                      {
                        const: "sqlite",
                        title: "SQLite",
                        description: "A local file-backed database.",
                      },
                      {
                        const: "postgres",
                        title: "Postgres",
                        description: "A networked database service.",
                      },
                    ],
                  },
                },
                required: ["database"],
              },
            },
          },
        },
      });
    },
    requestQuestion() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 100,
            method: "session/request_permission",
            params: {
              sessionId: "session-1",
              toolCall: {
                toolCallId: "tool-2",
                title: "AskUserQuestion",
                rawInput: {
                  questions: [
                    {
                      header: "Storage",
                      question: "Which database should we use?",
                      options: [
                        { label: "SQLite", description: "A local file-backed database." },
                        { label: "Postgres", description: "A networked database service." },
                      ],
                    },
                  ],
                },
              },
              options: [
                { optionId: "sqlite", name: "SQLite", kind: "allow_once" },
                { optionId: "postgres", name: "Postgres", kind: "allow_once" },
              ],
            },
          },
        },
      });
    },
    requestCursorQuestion() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 102,
            method: "cursor/ask_question",
            params: {
              toolCallId: "tool-3",
              title: "Storage",
              questions: [
                {
                  id: "database",
                  prompt: "Which database should we use?",
                  options: [
                    { id: "sqlite", label: "SQLite" },
                    { id: "postgres", label: "Postgres" },
                  ],
                },
                {
                  id: "region",
                  prompt: "Which region should we use?",
                  options: [
                    { id: "us", label: "US" },
                    { id: "eu", label: "EU" },
                  ],
                },
              ],
            },
          },
        },
      });
    },
    requestCursorPlan() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 103,
            method: "cursor/create_plan",
            params: { toolCallId: "tool-4", plan: "Do the work", todos: [] },
          },
        },
      });
    },
  };
}

void test("uses the official SDK to initialize, create a session, and route updates", async () => {
  const fake = fakeTransport();
  const updates = [];
  let ready;
  const connection = new AcpConnection(
    {
      status: () => {},
      ready: (session) => {
        ready = session;
      },
      update: (update) => {
        updates.push(update);
      },
      permission: () => {},
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  await connection.setConfigOption("fast_mode", true);
  await connection.prompt("Hello");

  assert.deepEqual(
    fake.sent.find((message) => message.method === "initialize")?.params.clientCapabilities,
    { session: { configOptions: { boolean: {} } }, elicitation: { form: {} } },
  );
  assert.equal(ready.harnessId, "harness-1");
  assert.equal(ready.sessionId, "session-1");
  assert.equal(ready.selectedModelId, "model-1");
  assert.deepEqual(
    fake.sent.find((message) => message.method === "session/set_config_option")?.params,
    { sessionId: "session-1", configId: "fast_mode", value: true, type: "boolean" },
  );
  assert.deepEqual(
    updates.map((update) => update.sessionUpdate),
    ["agent_message_chunk"],
  );
});

void test("enables Cursor's parameterized model picker", async () => {
  const fake = fakeTransport();
  const connection = new AcpConnection(
    {
      ready: () => {},
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: () => {},
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({
    cwd: "C:\\workspace",
    command: "agent.cmd",
    args: ["acp"],
    profileId: "cursor",
  });

  assert.deepEqual(
    fake.sent.find((message) => message.method === "initialize")?.params.clientCapabilities._meta,
    { parameterizedModelPicker: true },
  );
  assert.deepEqual(
    (await connection.listCursorModels()).map(({ model }) => model),
    [{ id: "claude-opus-5", name: "Claude Opus 5" }],
  );
  await connection.setFastModeConfigOption("fast", true, "string");
  assert.deepEqual(
    fake.sent.find(
      (message) =>
        message.method === "session/set_config_option" && message.params.configId === "fast",
    )?.params,
    { sessionId: "session-1", configId: "fast", value: "true" },
  );
});

void test("blocks another prompt when ACP completes with a tool still active", async () => {
  const fake = fakeTransport({ promptMode: "active-tool" });
  const turnStatuses = [];
  const errors = [];
  const connection = new AcpConnection(
    {
      connectionStatus: () => {},
      turnStatus: (status) => turnStatuses.push(status),
      ready: () => {},
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: (error) => errors.push(error),
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  await assert.rejects(connection.prompt("First"), /tool call\(s\) still active/);
  await assert.rejects(connection.prompt("Second"), /already has an active turn/);

  assert.equal(fake.sent.filter((message) => message.method === "session/prompt").length, 1);
  assert.deepEqual(turnStatuses, ["running", "blocked"]);
  assert.equal(errors[0].scope, "turn");
  assert.equal(errors[0].method, "session/prompt");
});

void test("cancellation does not release the active-turn guard before prompt completion", async () => {
  const fake = fakeTransport({ promptMode: "pending" });
  const connection = new AcpConnection(
    {
      connectionStatus: () => {},
      turnStatus: () => {},
      ready: () => {},
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: () => {},
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  void connection.prompt("First");
  await connection.cancel();
  await assert.rejects(connection.prompt("Second"), /already has an active turn/);
  assert.equal(fake.sent.filter((message) => message.method === "session/prompt").length, 1);
  assert.equal(
    fake.sent.some((message) => message.method === "session/cancel"),
    true,
  );
});

void test("preserves structured prompt errors without marking the connection unavailable", async () => {
  const fake = fakeTransport({ promptMode: "error" });
  const connectionStatuses = [];
  const turnStatuses = [];
  const errors = [];
  const connection = new AcpConnection(
    {
      connectionStatus: (status) => connectionStatuses.push(status),
      turnStatus: (status) => turnStatuses.push(status),
      ready: () => {},
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: (error) => errors.push(error),
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  await assert.rejects(connection.prompt("Hello"), /Internal error/);

  assert.deepEqual(connectionStatuses, ["connecting", "ready"]);
  assert.deepEqual(turnStatuses, ["running", "failed"]);
  assert.equal(errors[0].code, -32603);
  assert.deepEqual(errors[0].data, { detail: "turn already active" });
});

void test("prefers resuming an existing session without replaying history", async () => {
  const fake = fakeTransport({ loadSession: true, resumeSession: true });
  let ready;
  const connection = new AcpConnection(
    {
      status: () => {},
      ready: (session) => {
        ready = session;
      },
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({
    cwd: "C:\\workspace",
    command: "agent",
    args: [],
    sessionId: "existing-session",
  });

  assert.equal(
    fake.sent.some((message) => message.method === "session/load"),
    false,
  );
  assert.deepEqual(fake.sent.find((message) => message.method === "session/resume")?.params, {
    sessionId: "existing-session",
    cwd: "C:\\workspace",
    mcpServers: [],
  });
  assert.equal(ready.sessionId, "existing-session");
});

void test("loads an existing session without forwarding private agent history", async () => {
  const fake = fakeTransport({ loadSession: true });
  const updates = [];
  let ready;
  const connection = new AcpConnection(
    {
      status: () => {},
      ready: (session) => {
        ready = session;
      },
      update: (update) => {
        updates.push(update);
      },
      permission: () => {},
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({
    cwd: "C:\\workspace",
    command: "agent",
    args: [],
    sessionId: "existing-session",
  });

  assert.equal(
    fake.sent.some((message) => message.method === "session/new"),
    false,
  );
  assert.deepEqual(fake.sent.find((message) => message.method === "session/load")?.params, {
    sessionId: "existing-session",
    cwd: "C:\\workspace",
    mcpServers: [],
  });
  assert.equal(ready.sessionId, "existing-session");
  assert.deepEqual(updates, []);
});

void test("presents ACP form elicitations as agent questions", async () => {
  const fake = fakeTransport();
  let request;
  const connection = new AcpConnection(
    {
      status: () => {},
      ready: () => {},
      update: () => {},
      permission: (value) => {
        request = value;
        connection.answerPermission(value.requestId, "sqlite");
      },
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestElicitation();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.deepEqual(request, {
    requestId: 101,
    type: "question",
    title: "Storage",
    detail: "Which database should we use?",
    options: [
      {
        optionId: "sqlite",
        name: "SQLite",
        description: "A local file-backed database.",
      },
      {
        optionId: "postgres",
        name: "Postgres",
        description: "A networked database service.",
      },
    ],
  });
  const response = fake.sent.find((message) => message.id === 101 && "result" in message);
  assert.deepEqual(response.result, { action: "accept", content: { database: "sqlite" } });
});

void test("presents question tool input as an agent question", async () => {
  const fake = fakeTransport();
  let request;
  const connection = new AcpConnection(
    {
      status: () => {},
      ready: () => {},
      update: () => {},
      permission: (value) => {
        request = value;
        connection.answerPermission(value.requestId, "sqlite");
      },
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestQuestion();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.deepEqual(request, {
    requestId: 100,
    type: "question",
    title: "Storage",
    detail: "Which database should we use?",
    options: [
      {
        optionId: "sqlite",
        name: "SQLite",
        description: "A local file-backed database.",
        kind: "allow_once",
      },
      {
        optionId: "postgres",
        name: "Postgres",
        description: "A networked database service.",
        kind: "allow_once",
      },
    ],
  });
  const response = fake.sent.find((message) => message.id === 100 && "result" in message);
  assert.deepEqual(response.result, { outcome: { outcome: "selected", optionId: "sqlite" } });
});

void test("presents every Cursor question and declines Cursor plans", async () => {
  const fake = fakeTransport();
  const requests = [];
  const connection = new AcpConnection(
    {
      ready: () => {},
      update: () => {},
      permission: (value) => {
        requests.push(value);
        connection.answerPermission(value.requestId, requests.length === 1 ? "sqlite" : "eu");
      },
      stderr: () => {},
      error: () => {},
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestCursorQuestion();
  fake.requestCursorPlan();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.deepEqual(
    requests.map(({ detail, options }) => ({ detail, options })),
    [
      {
        detail: "Which database should we use?",
        options: [
          { optionId: "sqlite", name: "SQLite" },
          { optionId: "postgres", name: "Postgres" },
        ],
      },
      {
        detail: "Which region should we use?",
        options: [
          { optionId: "us", name: "US" },
          { optionId: "eu", name: "EU" },
        ],
      },
    ],
  );
  assert.deepEqual(fake.sent.find((message) => message.id === 102 && "result" in message).result, {
    outcome: {
      outcome: "answered",
      answers: [
        { questionId: "database", selectedOptionIds: ["sqlite"] },
        { questionId: "region", selectedOptionIds: ["eu"] },
      ],
    },
  });
  assert.deepEqual(fake.sent.find((message) => message.id === 103 && "result" in message).result, {
    outcome: { outcome: "rejected", reason: "LoopCode does not support plan approval." },
  });
});

void test("automatically approves permissions in full access mode without answering questions", async () => {
  const fake = fakeTransport();
  const requests = [];
  const connection = new AcpConnection(
    {
      status: () => {},
      ready: () => {},
      update: () => {},
      permission: (request) => requests.push(request),
      permissionMode: () => "full",
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestPermission();
  fake.requestQuestion();
  await new Promise((resolve) => setTimeout(resolve, 0));

  const permissionResponse = fake.sent.find((message) => message.id === 99 && "result" in message);
  assert.deepEqual(permissionResponse.result, {
    outcome: { outcome: "selected", optionId: "allow" },
  });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].type, "question");
  connection.cancelPermission(requests[0].requestId);
});

void test("returns permission decisions through the SDK request handler", async () => {
  const fake = fakeTransport();
  let connection;
  connection = new AcpConnection(
    {
      status: () => {},
      ready: () => {},
      update: () => {},
      permission: (request) => {
        connection.answerPermission(request.requestId, "allow");
      },
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestPermission();
  await new Promise((resolve) => setTimeout(resolve, 0));

  const response = fake.sent.find((message) => message.id === 99 && "result" in message);
  assert.deepEqual(response.result, { outcome: { outcome: "selected", optionId: "allow" } });
});
