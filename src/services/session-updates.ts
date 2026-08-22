import type {
  ContentBlock,
  SessionUpdate,
  ToolCall,
  ToolCallUpdate,
} from "@agentclientprotocol/sdk";

import type { ProviderSessionState, ThreadState, ToolActivity } from "../types/index.ts";
import { formattedValue, jsonValueSchema } from "../utils/json.ts";
import { appendMessage, nextTimestamp } from "../utils/messages.ts";
import type { AcpModelState } from "./acp.ts";

export class SessionUpdateHandler {
  #streamIds = new Map<string, { agent: string; thought: string }>();
  #applyConfig: (provider: ProviderSessionState, state: AcpModelState) => void;

  constructor(applyConfig: (provider: ProviderSessionState, state: AcpModelState) => void) {
    this.#applyConfig = applyConfig;
  }

  startTurn(threadId: string, profileId: string) {
    this.#resetStreams(threadId, profileId);
  }

  clear(threadId: string, profileId: string) {
    this.#streamIds.delete(connectionKey(threadId, profileId));
  }

  handle(
    thread: ThreadState,
    profileId: string,
    update: SessionUpdate,
    modelState?: AcpModelState,
  ) {
    if (update.sessionUpdate === "config_option_update") {
      if (modelState) this.#applyConfig(thread.providers[profileId], modelState);
      return;
    }
    if (
      update.sessionUpdate === "agent_message_chunk" ||
      update.sessionUpdate === "agent_thought_chunk"
    ) {
      const role = update.sessionUpdate === "agent_message_chunk" ? "agent" : "thought";
      const streams =
        this.#streamIds.get(connectionKey(thread.id, profileId)) ??
        this.#resetStreams(thread.id, profileId);
      const messageId = update.messageId ?? streams[role];
      const text = contentText(update.content);
      if (text) appendMessage(thread, messageId, role, text);
      return;
    }
    if (update.sessionUpdate === "tool_call" || update.sessionUpdate === "tool_call_update") {
      this.#upsertTool(thread, profileId, update);
      return;
    }
    if (update.sessionUpdate === "plan") {
      const text = update.entries
        .map((entry) => entry.content)
        .filter(Boolean)
        .join("\n");
      if (text) appendMessage(thread, `plan-${thread.id}`, "thought", text);
    }
  }

  #upsertTool(
    thread: ThreadState,
    profileId: string,
    update: (ToolCall | ToolCallUpdate) & { sessionUpdate: "tool_call" | "tool_call_update" },
  ) {
    const existing = thread.tools.find((tool) => tool.id === update.toolCallId);
    const content = update.content?.map(toolContentText).filter(Boolean).join("\n");
    const rawOutput = jsonValueSchema.safeParse(update.rawOutput).data;
    const rawInput = jsonValueSchema.safeParse(update.rawInput).data;
    const detail =
      content ||
      (rawOutput === undefined ? undefined : formattedValue(rawOutput)) ||
      (rawInput === undefined ? undefined : formattedValue(rawInput));
    const locations = update.locations?.map((location) => location.path) ?? [];
    const next: ToolActivity = {
      id: update.toolCallId,
      title: update.title ?? existing?.title ?? "Agent tool",
      kind: update.kind ?? existing?.kind ?? "other",
      status: update.status ?? existing?.status ?? "pending",
      detail: detail ?? existing?.detail,
      locations: locations.length > 0 ? locations : (existing?.locations ?? []),
      createdAt: existing?.createdAt ?? nextTimestamp(thread),
    };
    if (existing) Object.assign(existing, next);
    else {
      thread.tools.push(next);
      this.#resetStreams(thread.id, profileId);
    }
    thread.updatedAt = nextTimestamp(thread);
  }

  #resetStreams(threadId: string, profileId: string) {
    const boundary = crypto.randomUUID();
    const streams = {
      agent: `agent-${boundary}`,
      thought: `thought-${boundary}`,
    };
    this.#streamIds.set(connectionKey(threadId, profileId), streams);
    return streams;
  }
}

function connectionKey(threadId: string, profileId: string) {
  return `${threadId}:${profileId}`;
}

function contentText(content: ContentBlock) {
  if (content.type === "text") return content.text;
  if (content.type === "resource" && "text" in content.resource) return content.resource.text;
  return "";
}

function toolContentText(content: NonNullable<ToolCallUpdate["content"]>[number]) {
  if (content.type !== "content") return "";
  return contentText(content.content);
}
