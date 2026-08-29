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
  SessionFailure,
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
const toolMetaSchema = z.object({
  contextCompaction: z
    .object({ version: z.literal(1) })
    .passthrough()
    .optional(),
  codex: z
    .object({
      subagent: z
        .object({ threadId: z.string(), path: z.string(), activity: z.string() })
        .optional(),
      collaboration: z
        .object({
          tool: z.string(),
          senderThreadId: z.string(),
          receiverThreadIds: z.array(z.string()),
        })
        .optional(),
    })
    .optional(),
  claudeCode: z
    .object({
      parentToolUseId: z.string().optional(),
      subagent: z.literal(true).optional(),
      toolName: z.string().optional(),
    })
    .optional(),
});
/**
 * Hard caps on persisted per-tool-call state, which the provider otherwise grows unbounded.
 * The workspace file has a fixed 10 MB load limit (persistence.rs MAX_WORKSPACE_BYTES).
 */
const MAX_DIFF_TEXT_LENGTH = 200_000;
const MAX_TERMINAL_OUTPUT_LENGTH = 200_000;
/** Images persist as base64, so the cap is on the encoded length that actually gets written. */
const MAX_MEDIA_BASE64_LENGTH = 4_000_000;

function truncateHead(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return `…(truncated, ${text.length - maxLength} chars omitted)…\n${text.slice(text.length - maxLength)}`;
}

const backgroundInputSchema = z.object({ run_in_background: z.literal(true) });
const failureSchema = z.object({
  id: z.string().min(1),
  revision: z.number().int().positive(),
  category: z.enum(["connection", "access", "limit", "request", "service", "unknown"]),
  severity: z.enum(["warning", "error"]),
  title: z.string(),
  details: z.string().optional(),
  actions: z.array(z.enum(["retry", "login", "new_session"])),
});
const goalSchema = z.object({
  objective: z.string(),
  status: z.enum(["active", "paused", "blocked", "limited", "complete"]),
  iterations: z.number().int().nonnegative().optional(),
  lastReason: z.string().nullable().optional(),
  createdAt: z.number().optional(),
  updatedAt: z.number().optional(),
  tokenBudget: z.number().nullable().optional(),
  tokensUsed: z.number().optional(),
  timeBudgetSeconds: z.number().nullable().optional(),
  timeUsedSeconds: z.number().optional(),
});
const metadataSchema = z.object({
  goal: z.union([goalSchema, z.null()]).optional(),
  quota: z
    .object({
      token_count: z
        .object({
          totalTokens: z.number().optional(),
          inputTokens: z.number().optional(),
          cachedInputTokens: z.number().optional(),
          outputTokens: z.number().optional(),
          reasoningOutputTokens: z.number().optional(),
        })
        .nullable()
        .optional(),
    })
    .optional(),
  jetbrains: z.object({ air: z.object({ sessionFailure: failureSchema.optional() }) }).optional(),
  "_codex/rateLimits": z
    .array(
      z.object({
        limitId: z.string().nullable().optional(),
        limitName: z.string().nullable().optional(),
        primary: z
          .object({
            usedPercent: z.number(),
            resetsAt: z.number().nullable().optional(),
            windowDurationMins: z.number().nullable().optional(),
          })
          .nullable()
          .optional(),
        secondary: z
          .object({
            usedPercent: z.number(),
            resetsAt: z.number().nullable().optional(),
            windowDurationMins: z.number().nullable().optional(),
          })
          .nullable()
          .optional(),
      }),
    )
    .optional(),
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

  /** Like startTurn, but keeps the plan id: the turn never ended, so the Plan card is the same. */
  startFollowUp(threadId: string, profileId: string) {
    this.#resetStreams(threadId, profileId);
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
    this.handleMetadata(thread, profileId, update._meta);
    if (this.#handleTranscriptUpdate(thread, profileId, update)) return;
    this.#handleStateUpdate(thread, profileId, update, modelState);
  }

  #handleTranscriptUpdate(thread: ThreadState, profileId: string, update: SessionUpdate) {
    if (
      update.sessionUpdate !== "agent_message_chunk" &&
      update.sessionUpdate !== "agent_thought_chunk" &&
      update.sessionUpdate !== "tool_call" &&
      update.sessionUpdate !== "tool_call_update"
    )
      return false;
    if (update.sessionUpdate === "tool_call" || update.sessionUpdate === "tool_call_update") {
      this.#upsertTool(thread, profileId, update);
      return true;
    }
    const role = update.sessionUpdate === "agent_message_chunk" ? "agent" : "thought";
    const streams =
      this.#streamIds.get(connectionKey(thread.id, profileId)) ??
      this.#resetStreams(thread.id, profileId);
    const messageId = update.messageId ?? streams[role];
    const text = contentText(update.content);
    if (!text) return true;
    const parentId = toolMetaSchema.safeParse(update._meta).data?.claudeCode?.parentToolUseId;
    const parent = parentId ? thread.tools.find((tool) => tool.id === parentId) : undefined;
    if (parent) appendChildMessage(parent, messageId, role, text, () => nextTimestamp(thread));
    else appendMessage(thread, messageId, role, text);
    return true;
  }

  #handleStateUpdate(
    thread: ThreadState,
    profileId: string,
    update: SessionUpdate,
    modelState?: AcpModelState,
  ) {
    if (update.sessionUpdate === "config_option_update") {
      if (modelState) this.#applyConfig(thread.providers[profileId], modelState);
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
      return;
    }
    if (update.sessionUpdate === "plan_update") {
      this.#handlePlanUpdate(thread, profileId, update);
      return;
    }
    if (update.sessionUpdate === "plan_removed") {
      thread.tools = thread.tools.filter((tool) => tool.id !== `plan-${update.planId}`);
      return;
    }
    if (update.sessionUpdate === "compaction_update") {
      this.#upsertCompaction(thread, update);
      return;
    }
    if (update.sessionUpdate === "compaction_summary_chunk") {
      this.#appendCompactionSummary(thread, update.compactionId, contentText(update.content));
    }
  }

  #handlePlanUpdate(
    thread: ThreadState,
    profileId: string,
    update: Extract<SessionUpdate, { sessionUpdate: "plan_update" }>,
  ) {
    if (update.plan.type === "items") {
      this.#upsertPlan(thread, profileId, planEntries(update.plan.entries), update.plan.planId);
      return;
    }
    this.#upsertPlanDetail(
      thread,
      update.plan.planId,
      update.plan.type === "markdown" ? update.plan.content : update.plan.uri,
    );
  }

  handleMetadata(thread: ThreadState, profileId: string, metadata: unknown) {
    const parsed = metadataSchema.safeParse(metadata);
    if (!parsed.success) return;
    const provider = thread.providers[profileId];
    if (!provider) return;
    if (parsed.data.goal !== undefined) provider.goal = parsed.data.goal;
    const tokenCount = parsed.data.quota?.token_count;
    if (tokenCount) provider.quota = tokenCount;
    const rateLimits = parsed.data["_codex/rateLimits"];
    if (rateLimits)
      provider.rateLimits = rateLimits.map((limit, index) => ({
        id: limit.limitId ?? limit.limitName ?? String(index),
        name: limit.limitName ?? limit.limitId ?? "Limit",
        primary: limit.primary,
        secondary: limit.secondary,
      }));
    const failure = parsed.data.jetbrains?.air.sessionFailure;
    if (failure) this.#upsertFailure(thread, failure);
  }

  #upsertFailure(thread: ThreadState, failure: SessionFailure) {
    const id = `failure-${failure.id}`;
    const existing = thread.messages.find((message) => message.id === id);
    if (existing?.failure && existing.failure.revision >= failure.revision) return;
    if (existing) {
      existing.role = failure.severity === "warning" ? "notice" : "error";
      existing.text = failure.title;
      existing.failure = failure;
    } else {
      thread.messages.push({
        id,
        role: failure.severity === "warning" ? "notice" : "error",
        text: failure.title,
        failure,
        createdAt: nextTimestamp(thread),
      });
    }
    thread.updatedAt = nextTimestamp(thread);
  }

  /**
   * Plans arrive as a whole list on every revision, so the timeline keeps one entry per turn
   * and replaces its steps instead of appending each successive copy.
   */
  #upsertPlan(thread: ThreadState, profileId: string, plan: PlanEntry[], planId?: string) {
    if (plan.length === 0) return;
    const id = planId ? `plan-${planId}` : this.#planId(thread.id, profileId);
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

  #upsertPlanDetail(thread: ThreadState, planId: string, detail: string) {
    const id = `plan-${planId}`;
    const existing = thread.tools.find((tool) => tool.id === id);
    if (existing) existing.detail = detail;
    else
      thread.tools.push({
        id,
        title: "Plan",
        kind: "think",
        status: "in_progress",
        detail,
        locations: [],
        createdAt: nextTimestamp(thread),
      });
    thread.updatedAt = nextTimestamp(thread);
  }

  #upsertCompaction(
    thread: ThreadState,
    update: Extract<SessionUpdate, { sessionUpdate: "compaction_update" }>,
  ) {
    const id = `compaction-${update.compactionId}`;
    const existing = thread.tools.find((tool) => tool.id === id);
    const summary = update.summary?.map(contentText).filter(Boolean).join("\n");
    const detail = update.error ?? summary ?? existing?.detail;
    if (existing) Object.assign(existing, { status: update.status, detail });
    else
      thread.tools.push({
        id,
        title: "Context compaction",
        kind: "think",
        status: update.status,
        detail,
        presentation: "compaction",
        locations: [],
        createdAt: nextTimestamp(thread),
      });
    thread.updatedAt = nextTimestamp(thread);
  }

  #appendCompactionSummary(thread: ThreadState, compactionId: string, text: string) {
    if (!text) return;
    const tool = thread.tools.find((entry) => entry.id === `compaction-${compactionId}`);
    if (!tool) return;
    tool.detail = (tool.detail ?? "") + text;
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
    const parentId = toolMetaSchema.safeParse(update._meta).data?.claudeCode?.parentToolUseId;
    const parent = parentId ? thread.tools.find((tool) => tool.id === parentId) : undefined;
    if (parent) {
      const existing = parent.children?.find(
        (entry): entry is ToolActivity => "kind" in entry && entry.id === update.toolCallId,
      );
      const next = mergedTool(existing, update, () => nextTimestamp(thread));
      if (existing) Object.assign(existing, next);
      else (parent.children ??= []).push(next);
      thread.updatedAt = nextTimestamp(thread);
      return;
    }
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
  const meta = toolMetaSchema.safeParse(update._meta).data;
  // Structured content already says everything a raw payload dump would, and a trailing
  // update carrying the aggregate output would otherwise repeat the terminal panel as text.
  const structured =
    Boolean(diffs?.length) || terminal !== undefined || Boolean(content.media?.length);
  return {
    id: update.toolCallId,
    ...toolLabels(existing, update),
    detail: structured ? undefined : (content.detail ?? existing?.detail),
    diffs,
    terminal,
    media: content.media ?? existing?.media,
    presentation: toolPresentation(existing, update, meta, content.media),
    subagent: toolSubagent(existing, meta),
    children: existing?.children,
    locations: toolLocations(existing, update),
    createdAt: existing?.createdAt ?? timestamp(),
  };
}

function toolPresentation(
  existing: ToolActivity | undefined,
  update: ToolCall | ToolCallUpdate,
  meta: z.infer<typeof toolMetaSchema> | undefined,
  media: ToolActivity["media"],
): ToolActivity["presentation"] {
  if (meta?.contextCompaction) return "compaction";
  if (meta?.codex?.subagent || meta?.codex?.collaboration || meta?.claudeCode?.subagent)
    return "subagent";
  if (backgroundInputSchema.safeParse(update.rawInput).success) return "background";
  if (update.toolCallId.startsWith("guardian_assessment:") || update.title === "Guardian Review")
    return "review";
  if (media?.length || update.title?.startsWith("View Image")) return "image";
  return existing?.presentation;
}

function toolSubagent(
  existing: ToolActivity | undefined,
  meta: z.infer<typeof toolMetaSchema> | undefined,
) {
  const codex = meta?.codex?.subagent;
  const collaboration = meta?.codex?.collaboration;
  const claude = meta?.claudeCode;
  if (codex)
    return {
      ...existing?.subagent,
      threadId: codex.threadId,
      path: codex.path,
      activity: codex.activity,
    };
  if (collaboration)
    return {
      ...existing?.subagent,
      activity: collaboration.tool,
      senderThreadId: collaboration.senderThreadId,
      receiverThreadIds: collaboration.receiverThreadIds,
    };
  if (claude?.subagent || claude?.parentToolUseId)
    return { ...existing?.subagent, parentToolUseId: claude.parentToolUseId };
  return existing?.subagent;
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

export function slashCommands(
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
/**
 * Truncating each side of a diff independently can hide the edit completely: a change past the
 * cap leaves two identical prefixes, which renders as no change at all. An oversized diff drops
 * both sides and says so instead.
 */
function boundedDiff(
  entry: { path: string; oldText?: string | null; newText?: string | null },
  kind: ToolDiff["kind"],
): ToolDiff {
  const oldText = entry.oldText ?? null;
  const newText = entry.newText ?? null;
  const longest = Math.max(oldText?.length ?? 0, newText?.length ?? 0);
  if (longest > MAX_DIFF_TEXT_LENGTH) {
    return {
      path: entry.path,
      oldText: null,
      newText: `…(diff omitted: ${longest} chars exceeds the ${MAX_DIFF_TEXT_LENGTH} char display limit)…`,
      kind,
    };
  }
  return { path: entry.path, oldText, newText, kind };
}

function toolContent(update: ToolCall | ToolCallUpdate) {
  const texts: string[] = [];
  const diffs: ToolDiff[] = [];
  let terminal: ToolTerminal | undefined;
  const media: ToolActivity["media"] = [];

  for (const entry of update.content ?? []) {
    if (entry.type === "diff") {
      diffs.push(boundedDiff(entry, diffMetaSchema.safeParse(entry._meta).data?.kind));
    } else if (entry.type === "terminal") {
      terminal = { output: "" };
    } else if (entry.type === "content") {
      if (entry.content.type === "image") {
        if (entry.content.data.length > MAX_MEDIA_BASE64_LENGTH) {
          texts.push("[image omitted: exceeds the persisted size limit]");
          continue;
        }
        media.push({
          data: entry.content.data,
          mimeType: entry.content.mimeType,
          name: "Generated image",
        });
        continue;
      }
      const text = contentText(entry.content);
      if (text) texts.push(text);
    }
  }

  return {
    diffs: diffs.length > 0 ? diffs : undefined,
    terminal,
    media: media.length > 0 ? media : undefined,
    detail: toolDetail(update, texts.join("\n")),
  };
}

function appendChildMessage(
  parent: ToolActivity,
  id: string,
  role: "agent" | "thought",
  text: string,
  timestamp: () => number,
) {
  const existing = parent.children?.find(
    (entry): entry is import("../types/index.ts").TimelineMessage =>
      "role" in entry && entry.id === id,
  );
  if (existing) existing.text += text;
  else (parent.children ??= []).push({ id, role, text, createdAt: timestamp() });
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

function mergeTerminal(
  existing: ToolTerminal | undefined,
  started: ToolTerminal | undefined,
  update: ToolCall | ToolCallUpdate,
): ToolTerminal | undefined {
  const meta = terminalMetaSchema.safeParse(update._meta).data;
  const base = existing ?? started ?? streamedTerminal(meta);
  if (!base) return;
  const terminal = { ...base };
  if (meta?.terminal_output)
    terminal.output = truncateHead(meta.terminal_output.data, MAX_TERMINAL_OUTPUT_LENGTH);
  if (meta?.terminal_output_delta)
    terminal.output = truncateHead(
      terminal.output + meta.terminal_output_delta.data,
      MAX_TERMINAL_OUTPUT_LENGTH,
    );
  const command = commandOutputSchema.safeParse(update.rawOutput).data;
  if (!terminal.output && command?.formatted_output) terminal.output = command.formatted_output;
  if (meta?.terminal_exit) terminal.exitCode = meta.terminal_exit.exit_code ?? null;
  else if (command?.exit_code !== undefined) terminal.exitCode = command.exit_code;
  return terminal;
}

function streamedTerminal(
  meta: z.infer<typeof terminalMetaSchema> | undefined,
): ToolTerminal | undefined {
  if (meta?.terminal_output || meta?.terminal_output_delta) return { output: "" };
}
