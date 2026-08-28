import assert from "node:assert/strict";
import test from "node:test";

import { AcpConnection } from "../src/services/acp.ts";

function fakeTransport({
  loadSession = false,
  resumeSession = false,
  promptMode = "normal",
  steering = false,
  goal = false,
  authMethods = [],
  agentInfo,
} = {}) {
  let onEvent;
  let session = 0;
  let pendingPromptId;
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
          _meta:
            steering || goal
              ? {
                  ...(steering ? { steering: { supported: true } } : {}),
                  ...(goal
                    ? {
                        goal: {
                          version: 1,
                          controlMethod: "_session/goal",
                          actions: ["set", "clear"],
                        },
                      }
                    : {}),
                }
              : undefined,
          authMethods,
          agentInfo,
        });
      } else if (message.method === "authenticate") {
        reply(message.id, {});
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
        if (promptMode === "steerable") {
          pendingPromptId = message.id;
          return;
        }
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
      } else if (message.method === "_session/steering") {
        reply(message.id, { outcome: "injected" });
        if (pendingPromptId !== undefined) {
          reply(pendingPromptId, { stopReason: "end_turn" });
          pendingPromptId = undefined;
        }
      } else if (message.method === "_session/goal") {
        reply(message.id, {});
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
    emit(event) {
      onEvent(event);
    },
    requestPermission(meta) {
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
              _meta: meta,
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
    requestUnsupportedElicitation() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 103,
            method: "elicitation/create",
            params: {
              sessionId: "session-1",
              mode: "form",
              message: "Enter an access token.",
              requestedSchema: {
                type: "object",
                properties: {
                  token: { type: "string", minLength: 20 },
                },
                required: ["token"],
              },
            },
          },
        },
      });
    },
    requestFormQuestions() {
      onEvent({
        event: "rpc",
        data: {
          message: {
            jsonrpc: "2.0",
            id: 104,
            method: "elicitation/create",
            params: {
              sessionId: "session-1",
              mode: "form",
              message: "Please answer the following questions.",
              requestedSchema: {
                type: "object",
                properties: {
                  question_0: {
                    type: "string",
                    title: "Approach",
                    description: "Which approach should I take?",
                    oneOf: [
                      {
                        const: "Balanced",
                        title: "Balanced",
                        description: "Keep the change focused.",
                        _meta: {
                          "_claude/askUserQuestionOption": { preview: "One focused change." },
                        },
                      },
                      { const: "Thorough", title: "Thorough" },
                    ],
                  },
                  question_0_custom: {
                    type: "string",
                    title: "Other",
                    _meta: {
                      _askUserQuestionCustomAnswer: {
                        questionId: "question_0",
                        isCustomAnswer: true,
                      },
                    },
                  },
                  question_1: {
                    type: "array",
                    title: "Checks",
                    description: "Which checks should I run?",
                    items: {
                      anyOf: [
                        { const: "Tests", title: "Tests" },
                        { const: "Docs", title: "Docs" },
                      ],
                    },
                  },
                  question_1_custom: {
                    type: "string",
                    title: "Other",
                    _meta: {
                      codex: { questionId: "question_1", isOtherAnswer: true },
                    },
                  },
                },
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

void test("ignores broker events after the ACP stream has been stopped", async () => {
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

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  await connection.stop();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.doesNotThrow(() => fake.emit({ event: "exited", data: { code: 0, success: true } }));
  assert.doesNotThrow(() =>
    fake.emit({
      event: "rpc",
      data: { message: { jsonrpc: "2.0", method: "late-notification" } },
    }),
  );
});

void test("rejects concurrent connection attempts", async () => {
  const fake = fakeTransport();
  const launch = fake.transport.launch.bind(fake.transport);
  let releaseLaunch;
  const launchGate = new Promise((resolve) => {
    releaseLaunch = resolve;
  });
  let launches = 0;
  fake.transport.launch = async (...args) => {
    launches += 1;
    await launchGate;
    return launch(...args);
  };
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
  const request = { cwd: "C:\\workspace", command: "agent", args: [] };

  const first = connection.connect(request);
  await assert.rejects(connection.connect(request), /already active/);
  assert.equal(launches, 1);
  releaseLaunch();
  await first;
});

void test("uses the official SDK to initialize, create a session, and route updates", async () => {
  const fake = fakeTransport({
    agentInfo: { name: "test-agent", title: "Test agent", version: "1.2.3" },
  });
  const updates = [];
  let initialized;
  let ready;
  const connection = new AcpConnection(
    {
      status: () => {},
      initialized: (agentInfo) => {
        initialized = agentInfo;
      },
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

  const mcpServers = [{ type: "http", name: "docs", url: "https://example.com/mcp", headers: [] }];
  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [], mcpServers });
  await connection.setConfigOption("fast_mode", true);
  await connection.prompt([{ type: "text", text: "Hello" }]);

  assert.deepEqual(
    fake.sent.find((message) => message.method === "initialize")?.params.clientCapabilities,
    {
      plan: {},
      session: { configOptions: { boolean: {} }, compaction: {} },
      elicitation: { form: {} },
      _meta: {
        terminal_output: true,
        "subagent-transcript": true,
        jetbrains: { air: { version: 1, capabilities: ["sessionFailure"] } },
      },
    },
  );
  assert.equal(ready.harnessId, "harness-1");
  assert.equal(ready.sessionId, "session-1");
  assert.equal(ready.selectedModelId, "model-1");
  assert.equal(initialized.version, "1.2.3");
  assert.deepEqual(fake.sent.find((message) => message.method === "session/new")?.params, {
    cwd: "C:\\workspace",
    mcpServers,
  });
  assert.deepEqual(
    fake.sent.find((message) => message.method === "session/set_config_option")?.params,
    { sessionId: "session-1", configId: "fast_mode", value: true, type: "boolean" },
  );
  assert.deepEqual(
    updates.map((update) => update.sessionUpdate),
    ["agent_message_chunk"],
  );
});

void test("sends follow-ups through an advertised steering method", async () => {
  const fake = fakeTransport({ promptMode: "steerable", steering: true });
  let ready;
  const connection = new AcpConnection(
    {
      ready: (session) => {
        ready = session;
      },
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: () => {},
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  const prompt = connection.prompt([{ type: "text", text: "Start" }]);
  await connection.followUp([{ type: "text", text: "Also check the tests" }]);
  await prompt;

  assert.equal(ready.supportsFollowups, true);
  assert.deepEqual(fake.sent.find((message) => message.method === "_session/steering")?.params, {
    sessionId: "session-1",
    prompt: [{ type: "text", text: "Also check the tests" }],
    _meta: { steering: { idleBehavior: "promptRequired" } },
  });
});

void test("uses the advertised shared goal control method", async () => {
  const fake = fakeTransport({ goal: true });
  let ready;
  const connection = new AcpConnection(
    {
      ready: (session) => {
        ready = session;
      },
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: () => {},
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  await connection.goal("set", "Ship the change", ready.goalControlMethod);

  assert.deepEqual(ready.goalActions, ["set", "clear"]);
  assert.deepEqual(fake.sent.find((message) => message.method === "_session/goal")?.params, {
    sessionId: "session-1",
    action: "set",
    objective: "Ship the change",
  });
});

void test("sends select-typed fast mode as a string config value", async () => {
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
    command: "npx",
    args: ["--yes", "@agentclientprotocol/codex-acp"],
    profileId: "codex",
  });
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
  await assert.rejects(
    connection.prompt([{ type: "text", text: "First" }]),
    /tool call\(s\) still active/,
  );
  await assert.rejects(
    connection.prompt([{ type: "text", text: "Second" }]),
    /already has an active turn/,
  );

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
  void connection.prompt([{ type: "text", text: "First" }]);
  await connection.cancel();
  await assert.rejects(
    connection.prompt([{ type: "text", text: "Second" }]),
    /already has an active turn/,
  );
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
  await assert.rejects(connection.prompt([{ type: "text", text: "Hello" }]), /Internal error/);

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
    mcpServers: [{ name: "local", command: "server", args: [], env: [] }],
  });

  assert.equal(
    fake.sent.some((message) => message.method === "session/load"),
    false,
  );
  assert.deepEqual(fake.sent.find((message) => message.method === "session/resume")?.params, {
    sessionId: "existing-session",
    cwd: "C:\\workspace",
    mcpServers: [{ name: "local", command: "server", args: [], env: [] }],
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
    allowMultiple: false,
    allowCustomAnswer: false,
    required: true,
  });
  const response = fake.sent.find((message) => message.id === 101 && "result" in message);
  assert.deepEqual(response.result, { action: "accept", content: { database: "sqlite" } });
});

void test("cancels elicitations with unsupported constraints", async () => {
  const fake = fakeTransport();
  const connection = new AcpConnection(
    {
      ready: () => {},
      update: () => {},
      permission: () => assert.fail("unsupported forms must not be rendered"),
      stderr: () => {},
      error: (message) => {
        throw new Error(message);
      },
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestUnsupportedElicitation();
  await new Promise((resolve) => setTimeout(resolve, 0));

  const response = fake.sent.find((message) => message.id === 103 && "result" in message);
  assert.deepEqual(response.result, { action: "cancel" });
});

void test("supports single-select, multi-select, and custom form answers", async () => {
  const fake = fakeTransport();
  const requests = [];
  const connection = new AcpConnection(
    {
      ready: () => {},
      update: () => {},
      permission: (request) => {
        requests.push(request);
        if (requests.length === 1) {
          connection.answerQuestion(request.requestId, {
            selectedOptionIds: ["Balanced"],
            customAnswer: "Keep the public API unchanged",
          });
        } else {
          connection.answerQuestion(request.requestId, {
            selectedOptionIds: ["Tests", "Docs"],
          });
        }
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
  fake.requestFormQuestions();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.deepEqual(
    requests.map((request) => ({
      title: request.title,
      detail: request.detail,
      options: request.options,
      allowMultiple: request.allowMultiple,
      allowCustomAnswer: request.allowCustomAnswer,
      required: request.required,
    })),
    [
      {
        title: "Approach",
        detail: "Which approach should I take?",
        options: [
          {
            optionId: "Balanced",
            name: "Balanced",
            description: "Keep the change focused.",
            preview: "One focused change.",
          },
          { optionId: "Thorough", name: "Thorough", description: undefined },
        ],
        allowMultiple: false,
        allowCustomAnswer: true,
        required: false,
      },
      {
        title: "Checks",
        detail: "Which checks should I run?",
        options: [
          { optionId: "Tests", name: "Tests", description: undefined },
          { optionId: "Docs", name: "Docs", description: undefined },
        ],
        allowMultiple: true,
        allowCustomAnswer: true,
        required: false,
      },
    ],
  );
  const response = fake.sent.find((message) => message.id === 104 && "result" in message);
  assert.deepEqual(response.result, {
    action: "accept",
    content: {
      question_0_custom: "Keep the public API unchanged",
      question_1: ["Tests", "Docs"],
    },
  });
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
        connection.answerQuestion(value.requestId, { selectedOptionIds: ["sqlite"] });
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
    allowMultiple: false,
    allowCustomAnswer: false,
    required: true,
  });
  const response = fake.sent.find((message) => message.id === 100 && "result" in message);
  assert.deepEqual(response.result, { outcome: { outcome: "selected", optionId: "sqlite" } });
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

void test("uses Claude permission presentation metadata", async () => {
  const fake = fakeTransport();
  let request;
  const connection = new AcpConnection(
    {
      ready: () => {},
      update: () => {},
      permission: (value) => {
        request = value;
        connection.cancelPermission(value.requestId);
      },
      stderr: () => {},
      error: () => {},
      exited: () => {},
    },
    fake.transport,
  );

  await connection.connect({ cwd: "C:\\workspace", command: "agent", args: [] });
  fake.requestPermission({
    permission: {
      version: 1,
      title: "Run the test suite",
      description: "Reason: Verify the provider change.",
    },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.equal(request.title, "Run the test suite");
  assert.equal(request.description, "Reason: Verify the provider change.");
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
