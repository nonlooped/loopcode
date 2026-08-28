import assert from "node:assert/strict";
import test from "node:test";

import { workspaceMcpServers } from "../src/utils/mcp-servers.ts";

void test("maps workspace HTTP, SSE, and stdio MCP servers to ACP", () => {
  assert.deepEqual(
    workspaceMcpServers({
      mcpServers: {
        docs: { type: "http", url: "https://example.com/mcp", headers: { Authorization: "token" } },
        events: { type: "sse", url: "https://example.com/sse" },
        local: { command: "server", args: ["--stdio"], env: { TOKEN: "secret" } },
      },
    }),
    [
      {
        type: "http",
        name: "docs",
        url: "https://example.com/mcp",
        headers: [{ name: "Authorization", value: "token" }],
      },
      { type: "sse", name: "events", url: "https://example.com/sse", headers: [] },
      {
        name: "local",
        command: "server",
        args: ["--stdio"],
        env: [{ name: "TOKEN", value: "secret" }],
      },
    ],
  );
});
