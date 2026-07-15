/**
 * DTOs for the Core IPC contract. These mirror the Rust command return shapes
 * and are the preserved interface between the rewritten UI and the untouched
 * Core process. Ported from the legacy vanilla frontend.
 */

export interface HeroCard {
  id: string;
  name: string;
  docsUrl?: string | null;
  firstParty: boolean;
  defaultModel?: string | null;
  /** True when Core already has a key for this provider (never the key itself). */
  hasSavedKey?: boolean;
}

/** Model entry from the bundled models.dev-style catalog. */
export interface CatalogModel {
  id: string;
  name: string;
  default?: boolean;
  /** Model can reason/think at all. */
  reasoning?: boolean;
  /** Named effort levels the model accepts, low → high. */
  reasoningEfforts?: string[];
  /** Model supports a plain on/off reasoning switch. */
  reasoningToggle?: boolean;
}

/** Composer reasoning selection sent with a run (mirrors Rust ReasoningConfig). */
export interface ReasoningConfig {
  enabled: boolean;
  effort?: string | null;
}

export interface CatalogProvider {
  id: string;
  name: string;
  docsUrl?: string | null;
  api?: string | null;
  npm?: string | null;
  env?: string[] | null;
  defaultModel?: string | null;
  models: CatalogModel[];
}

/** User's currently ready provider (no secrets) — from `provider.ready` settings. */
export interface ReadyProvider {
  providerId: string;
  model?: string | null;
  connectionId?: string | null;
}

export interface ConnectResult {
  connection: {
    providerId: string;
    ready: boolean;
    defaultModel?: string | null;
    lastProbeError?: string | null;
  };
  probe: {
    ready: boolean;
    model?: string | null;
    notice?: string | null;
    error?: { message: string } | null;
    usedFixture: boolean;
  };
}

export interface Project {
  id: string;
  displayPath: string;
  rootPath: string;
}

export interface Chat {
  id: string;
  projectId: string;
  title: string;
}

export type Mode = "ask" | "plan" | "build" | "debug";

/** Mirrors Core `TimelineItemKind` (serde snake_case). */
export type TimelineItemKind =
  | "user_message"
  | "assistant_message"
  | "tool_proposal"
  | "tool_result"
  | "tool_rejected"
  | "approval"
  | "error"
  | "usage"
  | "lifecycle"
  | "system"
  | "unknown";

export interface TimelineItem {
  id: string;
  seq: number;
  kind: TimelineItemKind | string;
  title: string;
  body: string;
  runId?: string | null;
  /** Raw Core event kind (e.g. run.lifecycle.completed). */
  eventKind?: string;
  actionClass?: string | null;
  toolName?: string | null;
  needsApproval: boolean;
  /** Full Core event payload (tool args / results). */
  payload?: unknown;
}

export interface TimelineProjection {
  state: string;
  items: TimelineItem[];
  statusLabel: string;
  costDisplay: string;
  tokensDisplay: string;
  activeRunId?: string | null;
  activeRunStatus?: string | null;
}

export interface RunDto {
  id: string;
  mode: string;
  status: string;
  model?: string | null;
}

export interface TreeEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  excluded: boolean;
  children: TreeEntry[];
}

export interface DiffChange {
  path: string;
  kind: string;
  attribution: string;
  runId?: string | null;
  toolId?: string | null;
  /** Line stats when computable (absent for legacy events). */
  linesAdded?: number | null;
  linesRemoved?: number | null;
}

export type DiffScope =
  | "this_run"
  | "since_chat_start"
  | "buffer_vs_disk"
  | "vs_checkpoint";

export interface SkillMeta {
  name: string;
  description: string;
  path: string;
  /** Discovery root: project | loopcode | agents */
  origin: string;
  /** Script filenames under the skill dir (paths only — not executed on list). */
  scripts: string[];
}

export type McpTransport =
  | { type: "stdio"; command: string; args: string[]; env_keys: string[] }
  | { type: "http"; url: string; header_keys: string[] };

export interface McpServer {
  id: string;
  name: string;
  transport: McpTransport;
  projectId: string | null;
  enabled: boolean;
  fingerprint: string;
}

export interface ReliabilitySettings {
  updateCheckDefault: boolean;
  labels: string[];
}

export type ApprovalScope = "deny" | "once" | "this_chat";
