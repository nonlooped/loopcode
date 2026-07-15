// Minimal newline-delimited JSON-RPC MCP echo server for LoopCode tests.
import readline from "node:readline";

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

for await (const raw of rl) {
  const line = raw.trim();
  if (!line) continue;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    continue;
  }
  const method = msg.method;
  const mid = msg.id;
  if (method === "initialize") {
    const resp = {
      jsonrpc: "2.0",
      id: mid,
      result: {
        protocolVersion: "2024-11-05",
        capabilities: { tools: {} },
        serverInfo: { name: "loopcode-echo", version: "1.0" },
      },
    };
    process.stdout.write(JSON.stringify(resp) + "\n");
  } else if (method === "notifications/initialized") {
    continue;
  } else if (method === "tools/call") {
    const params = msg.params ?? {};
    const resp = {
      jsonrpc: "2.0",
      id: mid,
      result: {
        content: [{ type: "text", text: "echo:" + JSON.stringify(params) }],
        isError: false,
      },
    };
    process.stdout.write(JSON.stringify(resp) + "\n");
    break;
  } else if (mid != null) {
    process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: mid, result: {} }) + "\n");
  }
}
