import assert from "node:assert/strict";
import test from "node:test";

import { AcpConnection } from "../src/services/acp.ts";

function fakeTransport({ loadSession = false, resumeSession = false } = {}) {
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
  await connection.setBooleanConfigOption("fast_mode", true);
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
