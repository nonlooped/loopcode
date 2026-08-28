import type {
  ContentBlock,
  SessionUpdate,
  ToolCall,
  ToolCallUpdate,
} from "@agentclientprotocol/sdk";
import { z } from "zod";

import type {
  PlanEntry,
  ProviderSessionState,
  SlashCommand,
  ThreadState,
  ToolActivity,
  ToolDiff,
  ToolTerminal,
} from "../types/index.ts";
import { formattedValue, jsonValueSchema } from "../utils/json.ts";
import { appendMessage, nextTimestamp } from "../utils/messages.ts";
import type { AcpModelState } from "./acp.ts";

/** Codex reports command output through `_meta` rather than terminal content blocks. */
const terminalOutputSchema = z.object({ data: z.string(), terminal_id: z.string() });
const terminalExitSchema = z.object({
  exit_code: z.number().nullable().optional(),
  terminal_id: z.string(),
});
const terminalMetaSchema = z.object({
  terminal_output: terminalOutputSchema.optional(),
  terminal_output_delta: terminalOutputSchema.optional(),
  terminal_exit: terminalExitSchema.optional(),
});
const diffMetaSchema = z.object({ kind: z.enum(["add", "update", "delete"]).optional() });
const commandOutputSchema = z.object({
  formatted_output: z.string().min(1).optional(),
  exit_code: z.number().nullable().optional(),
});

type TerminalMeta = z.infer<typeof terminalMetaSchema> | undefined;

export class SessionUpdateHandler {
  #streamIds = new Map<string, { agent: string; thought: string }>();
  #planIds = new Map<string, string>();
  #applyConfig: (provider: ProviderSessionState, state: AcpModelState) => void;

  constructor(applyConfig: (provider: ProviderSessionState, state: AcpModelState) => void) {
    this.#applyConfig = applyConfig;
  }

  startTurn(threadId: string, profileId: string) {
    this.#resetStreams(threadId, profileId);
    this.#planIds.set(connectionKey(threadId, profileId), `plan-${crypto.randomUUID()}`);
  }

  clear(threadId: string, profileId: string) {
    this.#streamIds.delete(connectionKey(threadId, profileId));
    this.#planIds.delete(connectionKey(threadId, profileId));
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
    if (update.sessionUpdate === "available_commands_update") {
      const provider = thread.providers[profileId];
      if (provider) provider.commands = slashCommands(update.availableCommands);
      return;
    }
    if (update.sessionUpdate === "usage_update") {
      const provider = thread.providers[profileId];
      if (provider && update.size > 0) {
        provider.contextUsed = update.used;
        provider.contextSize = update.size;
      }
      return;
    }
    if (update.sessionUpdate === "plan") {
      this.#upsertPlan(thread, profileId, planEntries(update.entries));
    }
  }

  /**
   * Plans arrive as a whole list on every revision, so the timeline keeps one entry per turn
   * and replaces its steps instead of appending each successive copy.
   */
  #upsertPlan(thread: ThreadState, profileId: string, plan: PlanEntry[]) {
    if (plan.length === 0) return;
    const id = this.#planId(thread.id, profileId);
    const status = plan.every((entry) => entry.status === "completed")
      ? "completed"
      : "in_progress";
    const existing = thread.tools.find((tool) => tool.id === id);
    if (existing) {
      existing.plan = plan;
      existing.status = status;
    } else {
      thread.tools.push({
        id,
        title: "Plan",
        kind: "think",
        status,
        plan,
        locations: [],
        createdAt: nextTimestamp(thread),
      });
    }
    thread.updatedAt = nextTimestamp(thread);
  }

  /** One plan entry per turn: tool calls reset the message streams, but not the plan. */
  #planId(threadId: string, profileId: string) {
    const key = connectionKey(threadId, profileId);
    const existing = this.#planIds.get(key);
    if (existing) return existing;
    const id = `plan-${crypto.randomUUID()}`;
    this.#planIds.set(key, id);
    return id;
  }

  #upsertTool(
    thread: ThreadState,
    profileId: string,
    update: (ToolCall | ToolCallUpdate) & { sessionUpdate: "tool_call" | "tool_call_update" },
  ) {
    const existing = thread.tools.find((tool) => tool.id === update.toolCallId);
    const next = mergedTool(existing, update, () => nextTimestamp(thread));
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

function mergedTool(
  existing: ToolActivity | undefined,
  update: ToolCall | ToolCallUpdate,
  timestamp: () => number,
): ToolActivity {
  const content = toolContent(update);
  const diffs = content.diffs ?? existing?.diffs;
  const terminal = mergeTerminal(existing?.terminal, content.terminal, update);
  // Structured content already says everything a raw payload dump would, and a trailing
  // update carrying the aggregate output would otherwise repeat the terminal panel as text.
  const structured = Boolean(diffs?.length) || terminal !== undefined;
  return {
    id: update.toolCallId,
    ...toolLabels(existing, update),
    detail: structured ? undefined : (content.detail ?? existing?.detail),
    diffs,
    terminal,
    locations: toolLocations(existing, update),
    createdAt: existing?.createdAt ?? timestamp(),
  };
}

function toolLabels(existing: ToolActivity | undefined, update: ToolCall | ToolCallUpdate) {
  return {
    title: update.title ?? existing?.title ?? "Agent tool",
    kind: update.kind ?? existing?.kind ?? "other",
    status: update.status ?? existing?.status ?? "pending",
  };
}

function toolLocations(existing: ToolActivity | undefined, update: ToolCall | ToolCallUpdate) {
  const locations = update.locations?.map((location) => location.path) ?? [];
  return locations.length > 0 ? locations : (existing?.locations ?? []);
}

function slashCommands(
  commands: { name: string; description: string; input?: { hint: string } | null }[],
): SlashCommand[] {
  return commands
    .filter((command) => command.name)
    .map((command) => ({
      name: command.name,
      description: command.description,
      hint: command.input?.hint,
    }));
}

function planEntries(entries: { content: string; status?: string }[]): PlanEntry[] {
  return entries
    .filter((entry) => entry.content)
    .map((entry) => ({
      content: entry.content,
      status:
        entry.status === "in_progress" || entry.status === "completed" ? entry.status : "pending",
    }));
}

function contentText(content: ContentBlock) {
  if (content.type === "text") return content.text;
  if (content.type === "resource_link") return content.name || content.uri;
  if (content.type === "resource" && "text" in content.resource) return content.resource.text;
  return "";
}

/**
 * Splits a tool call's content into the shapes the transcript renders. Anything unrecognised
 * falls back to `rawOutput`/`rawInput` so no update goes on screen empty.
 */
function toolContent(update: ToolCall | ToolCallUpdate) {
  const texts: string[] = [];
  const diffs: ToolDiff[] = [];
  let terminal: ToolTerminal | undefined;

  for (const entry of update.content ?? []) {
    if (entry.type === "diff") {
      diffs.push({
        path: entry.path,
        oldText: entry.oldText ?? null,
        newText: entry.newText ?? null,
        kind: diffMetaSchema.safeParse(entry._meta).data?.kind,
      });
    } else if (entry.type === "terminal") {
      terminal = { terminalId: entry.terminalId, output: "" };
    } else if (entry.type === "content") {
      const text = contentText(entry.content);
      if (text) texts.push(text);
    }
  }

  return {
    diffs: diffs.length > 0 ? diffs : undefined,
    terminal,
    detail: toolDetail(update, texts.join("\n")),
  };
}

function toolDetail(update: ToolCall | ToolCallUpdate, text: string) {
  if (text) return text;
  const formatted = commandOutputSchema.safeParse(update.rawOutput).data?.formatted_output;
  if (formatted) return formatted;
  const rawOutput = jsonValueSchema.safeParse(update.rawOutput).data;
  const rawInput = jsonValueSchema.safeParse(update.rawInput).data;
  return (
    (rawOutput === undefined ? undefined : formattedValue(rawOutput)) ??
    (rawInput === undefined ? undefined : formattedValue(rawInput))
  );
}

/**
 * Command output streams in as `_meta` deltas across many updates, so terminal state has to
 * accumulate rather than be replaced by each one.
 */
function mergeTerminal(
  existing: ToolTerminal | undefined,
  started: ToolTerminal | undefined,
  update: ToolCall | ToolCallUpdate,
): ToolTerminal | undefined {
  const meta = terminalMetaSchema.safeParse(update._meta).data;
  const base = existing ?? started ?? streamedTerminal(meta);
  if (!base) return undefined;
  const terminal = { ...base };
  applyTerminalOutput(terminal, meta, update);
  return terminal;
}

function streamedTerminal(meta: TerminalMeta): ToolTerminal | undefined {
  const terminalId = meta?.terminal_output?.terminal_id ?? meta?.terminal_output_delta?.terminal_id;
  return terminalId ? { terminalId, output: "" } : undefined;
}

function applyTerminalOutput(
  terminal: ToolTerminal,
  meta: TerminalMeta,
  update: ToolCall | ToolCallUpdate,
) {
  if (meta?.terminal_output) terminal.output = meta.terminal_output.data;
  if (meta?.terminal_output_delta) terminal.output += meta.terminal_output_delta.data;
  const command = commandOutputSchema.safeParse(update.rawOutput).data;
  if (!terminal.output && command?.formatted_output) terminal.output = command.formatted_output;
  if (meta?.terminal_exit) terminal.exitCode = meta.terminal_exit.exit_code ?? null;
  else if (command?.exit_code !== undefined) terminal.exitCode = command.exit_code;
}
