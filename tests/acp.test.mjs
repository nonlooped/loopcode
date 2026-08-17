import assert from "node:assert/strict";
import test from "node:test";

import { AcpConnection } from "../src/services/acp.ts";

function fakeTransport() {
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
        reply(message.id, { protocolVersion: 1, agentCapabilities: {}, authMethods: [] });
      } else if (message.method === "session/new") {
        session += 1;
        reply(message.id, { sessionId: `session-${session}`, configOptions });
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
  await connection.prompt("Hello");

  assert.equal(ready.harnessId, "harness-1");
  assert.equal(ready.sessionId, "session-1");
  assert.equal(ready.selectedModelId, "model-1");
  assert.deepEqual(
    updates.map((update) => update.sessionUpdate),
    ["agent_message_chunk"],
  );
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
