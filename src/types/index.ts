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
}

export interface ToolActivity {
  id: string;
  title: string;
  kind: string;
  status: string;
  detail?: string;
  locations: string[];
  createdAt: number;
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

export type PersistedWorkspace =
  | {
      version: 1;
      selectedThreadId: string;
      threads: PersistedThreadState[];
    }
  | {
      version: 2;
      selectedThreadId: string;
      selectedProjectId?: string | null;
      threads: PersistedThreadState[];
      projects: PersistedProjectState[];
    };

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
  error?: string;
  errorDetails?: AcpErrorDetails;
}

export type DesktopPlatform = "linux" | "macos" | "windows";

export interface HarnessProfile {
  id: string;
  label: string;
  icon: string;
  iconMode: "brand" | "theme";
  command: string;
  args: string[];
  versionCommand: string;
  versionArgs: string[];
  authCommand?: string;
  authArgs?: string[];
  platforms: DesktopPlatform[];
  supportsImages: boolean;
  titleGeneration: boolean;
  probeModelOptions: boolean;
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

export interface PermissionDecisionRequest extends InteractionRequest {
  type: "permission";
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

export type ProviderUnavailableReason =
  | "authentication"
  | "discovery"
  | "missing-executable"
  | "unsupported-platform";

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
