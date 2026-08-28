import type { McpServer } from "@agentclientprotocol/sdk";
import { z } from "zod";

import { readProjectFile } from "../services/native.ts";

const stringRecord = z.record(z.string(), z.string());
const serverSchema = z.union([
  z.object({
    type: z.enum(["http", "sse"]).default("http"),
    url: z.url(),
    headers: stringRecord.optional(),
  }),
  z.object({
    command: z.string().min(1),
    args: z.array(z.string()).optional(),
    env: stringRecord.optional(),
  }),
]);
const workspaceMcpSchema = z.object({ mcpServers: z.record(z.string(), serverSchema) });

export function workspaceMcpServers(value: unknown): McpServer[] {
  const parsed = workspaceMcpSchema.parse(value);
  return Object.entries(parsed.mcpServers).map(([name, server]) => {
    if ("url" in server) {
      return {
        type: server.type,
        name,
        url: server.url,
        headers: Object.entries(server.headers ?? {}).map(([headerName, value]) => ({
          name: headerName,
          value,
        })),
      };
    }
    return {
      name,
      command: server.command,
      args: server.args ?? [],
      env: Object.entries(server.env ?? {}).map(([envName, value]) => ({ name: envName, value })),
    };
  });
}

export async function loadWorkspaceMcpServers(cwd: string): Promise<McpServer[]> {
  try {
    const bytes = await readProjectFile(cwd, `${cwd}/.mcp.json`);
    return workspaceMcpServers(JSON.parse(new TextDecoder().decode(bytes)));
  } catch {
    return [];
  }
}
