export type ConnectionStatus = "disconnected" | "connecting" | "ready" | "stopped" | "error";

export type TurnStatus = "idle" | "running" | "failed" | "blocked";

export type ThreadStatus = ConnectionStatus | "running";

export interface AcpErrorDetails {
  scope: "connection" | "turn" | "transport";
  method?: string;
  code?: number;
  message: string;
  data?: unknown;
}

export type MessageRole = "user" | "agent" | "thought" | "notice" | "error";

export type PermissionMode = "restricted" | "full";

export interface MessageImage {
  data: string;
  mimeType: string;
  name: string;
}

export type ComposerReferenceKind = "file" | "folder" | "skill";

export interface ComposerReference {
  id: string;
  kind: ComposerReferenceKind;
  name: string;
  path: string;
  relativePath: string;
  uri: string;
}

export type PromptPart =
  | { type: "text"; text: string }
  | { type: "reference"; reference: ComposerReference };

export interface TimelineMessage {
  id: string;
  role: MessageRole;
  text: string;
  content?: PromptPart[];
  images?: MessageImage[];
  createdAt: number;
  failure?: SessionFailure;
  /** Set on a user message sent as a mid-turn follow-up, so timelineEntries keeps it in the
   * still-running turn's segment instead of treating it as the start of a new turn. */
  followUp?: boolean;
}

export type SessionFailureAction = "retry" | "login" | "new_session";

export interface SessionFailure {
  id: string;
  revision: number;
  category: "connection" | "access" | "limit" | "request" | "service" | "unknown";
  severity: "warning" | "error";
  title: string;
  details?: string;
  actions: SessionFailureAction[];
}

export type GoalAction = "set" | "pause" | "resume" | "clear";

export interface GoalState {
  objective: string;
  status: "active" | "paused" | "blocked" | "limited" | "complete";
  iterations?: number;
  lastReason?: string | null;
  createdAt?: number;
  tokenBudget?: number | null;
  tokensUsed?: number;
  timeBudgetSeconds?: number | null;
  timeUsedSeconds?: number;
}

export interface QuotaState {
  totalTokens?: number;
}

export interface RateLimitState {
  id: string;
  name: string;
  primary?: {
    usedPercent: number;
    resetsAt?: number | null;
  } | null;
}

export type ToolDiffKind = "add" | "update" | "delete";

export interface ToolDiff {
  path: string;
  oldText: string | null;
  newText: string | null;
  kind?: ToolDiffKind;
}

export interface ToolTerminal {
  output: string;
  exitCode?: number | null;
}

export type PlanEntryStatus = "pending" | "in_progress" | "completed";

export interface PlanEntry {
  content: string;
  status: PlanEntryStatus;
}

export interface ToolActivity {
  id: string;
  title: string;
  kind: string;
  status: string;
  detail?: string;
  diffs?: ToolDiff[];
  terminal?: ToolTerminal;
  plan?: PlanEntry[];
  locations: string[];
  createdAt: number;
  media?: MessageImage[];
  presentation?: "image" | "review" | "compaction" | "subagent" | "background";
  subagent?: {
    threadId?: string;
    path?: string;
    activity?: string;
    parentToolUseId?: string;
    senderThreadId?: string;
    receiverThreadIds?: string[];
  };
  children?: Array<TimelineMessage | ToolActivity>;
}

export interface ProjectState {
  id: string;
  name: string;
  path: string;
  createdAt: number;
}

export interface ThreadState {
  id: string;
  title: string;
  profileId: string;
  cwd: string;
  messages: TimelineMessage[];
  tools: ToolActivity[];
  draft: string;
  draftReferences: ComposerReference[];
  providers: Record<string, ProviderSessionState>;
  updatedAt: number;
  settled: boolean;
  projectId?: string | null;
  managedWorktree: boolean;
}

export interface PersistedThreadState {
  id: string;
  title: string;
  profileId: string;
  cwd: string;
  messages: TimelineMessage[];
  tools: ToolActivity[];
  draft: string;
  draftReferences?: ComposerReference[];
  updatedAt: number;
  settled?: boolean;
  projectId?: string | null;
  managedWorktree?: boolean;
  providerSessionIds?: Record<string, string>;
}

export interface PersistedProjectState {
  id: string;
  name: string;
  path: string;
  createdAt: number;
}

export interface PersistedWorkspace {
  version: 2;
  selectedThreadId: string;
  selectedProjectId?: string | null;
  threads: PersistedThreadState[];
  projects: PersistedProjectState[];
}

export interface ModelOption {
  id: string;
  name: string;
  description?: string;
}

export type ReasoningOption = ModelOption;

export interface ReasoningModelOption {
  options: ReasoningOption[];
  selectedId?: string;
}

export type FastModeValueType = "boolean" | "string";

export interface FastModeOption {
  configId: string;
  enabled: boolean;
  valueType?: FastModeValueType;
  description?: string;
}

export interface SlashCommand {
  name: string;
  description: string;
  hint?: string;
}

export type SessionSelectId = "mode" | "collaboration" | "agent";

export interface SessionSelect {
  configId: string;
  options: ModelOption[];
  selectedId?: string;
}

export type SessionSelects = Partial<Record<SessionSelectId, SessionSelect>>;

export interface ProviderSessionState {
  connectionStatus: ConnectionStatus;
  turnStatus: TurnStatus;
  harnessId?: string;
  sessionId?: string;
  modelConfigId?: string;
  models: ModelOption[];
  selectedModelId?: string;
  reasoningConfigId?: string;
  reasoningOptions: ReasoningOption[];
  selectedReasoningId?: string;
  reasoningOptionsByModel?: Record<string, ReasoningModelOption>;
  fastModeConfigId?: string;
  fastModeModelId?: string;
  fastModeOptionsByModel?: Record<string, FastModeOption>;
  fastModeEnabled?: boolean;
  fastModeValueType?: FastModeValueType;
  fastModeDescription?: string;
  selects?: SessionSelects;
  supportsFollowups?: boolean;
  contextUsed?: number;
  contextSize?: number;
  commands?: SlashCommand[];
  error?: string;
  errorDetails?: AcpErrorDetails;
  goal?: GoalState | null;
  goalActions?: GoalAction[];
  goalControlMethod?: string;
  quota?: QuotaState;
  rateLimits?: RateLimitState[];
}

export interface HarnessProfile {
  id: string;
  label: string;
  icon: string;
  iconMode: "brand" | "theme";
  command: string;
  args: string[];
  versionCommand: string;
  installCommand: string;
  loginCommand: string;
}

export interface ConnectRequest {
  cwd: string;
  command: string;
  args: string[];
  profileId?: string;
  threadId?: string;
  sessionId?: string;
}

export interface PermissionOption {
  optionId: string;
  name: string;
  description?: string;
  preview?: string;
  kind?: string;
}

interface InteractionRequest {
  requestId: string | number | null;
  title: string;
  detail: string;
  options: PermissionOption[];
}

export interface PermissionFileChange {
  path: string;
  kind: "add" | "update" | "delete";
  diff: string;
}

export interface PermissionDecisionRequest extends InteractionRequest {
  type: "permission";
  description?: string;
  /** Populated for edit approvals so the prompt shows the change rather than a raw payload. */
  fileChanges?: PermissionFileChange[];
  /** Populated for plan approvals, which carry the proposed plan as Markdown. */
  planMarkdown?: string;
}

export interface QuestionRequest extends InteractionRequest {
  type: "question";
  allowMultiple: boolean;
  allowCustomAnswer: boolean;
  required: boolean;
}

export type PermissionRequest = PermissionDecisionRequest | QuestionRequest;

export interface QuestionAnswer {
  selectedOptionIds: string[];
  customAnswer?: string;
}

export type ProviderUnavailableReason = "authentication" | "discovery" | "missing-executable";

interface ProviderModelCatalogState {
  agentVersion?: string;
  models: ModelOption[];
  selectedModelId?: string;
  reasoningOptions: ReasoningOption[];
  selectedReasoningId?: string;
  reasoningOptionsByModel?: Record<string, ReasoningModelOption>;
  fastModeConfigId?: string;
  fastModeModelId?: string;
  fastModeOptionsByModel?: Record<string, FastModeOption>;
  fastModeEnabled?: boolean;
  fastModeValueType?: FastModeValueType;
  fastModeDescription?: string;
  selects?: SessionSelects;
  supportsFollowups?: boolean;
  commands?: SlashCommand[];
  needsAuth?: boolean;
}

export type ProviderModelCatalog = ProviderModelCatalogState &
  (
    | { status: "loading" | "ready"; error?: never; unavailableReason?: never }
    | { status: "unavailable"; error: string; unavailableReason: ProviderUnavailableReason }
  );

export interface ComposerImage {
  id: string;
  type: "image";
  data: string;
  mimeType: string;
  name: string;
  previewUrl: string;
}
