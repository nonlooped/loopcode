export type ThreadStatus =
  | "disconnected"
  | "connecting"
  | "ready"
  | "running"
  | "stopped"
  | "error";

export type MessageRole = "user" | "agent" | "thought" | "notice" | "error";

export interface MessageImage {
  data: string;
  mimeType: string;
  name: string;
}

export interface TimelineMessage {
  id: string;
  role: MessageRole;
  text: string;
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
  providers: Record<string, ProviderSessionState>;
  updatedAt: number;
  settled: boolean;
  projectId?: string | null;
}

export interface PersistedThreadState {
  id: string;
  title: string;
  profileId: string;
  cwd: string;
  messages: TimelineMessage[];
  tools: ToolActivity[];
  draft: string;
  updatedAt: number;
  settled?: boolean;
  projectId?: string | null;
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

export interface FastModeOption {
  configId: string;
  enabled: boolean;
  description?: string;
}

export interface ProviderSessionState {
  status: ThreadStatus;
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
  fastModeDescription?: string;
  error?: string;
}

export interface HarnessProfile {
  id: string;
  label: string;
  icon: string;
  command: string;
  args: string[];
}

export interface ConnectRequest {
  cwd: string;
  command: string;
  args: string[];
  sessionId?: string;
}

export interface PermissionOption {
  optionId: string;
  name: string;
  description?: string;
  kind?: string;
}

export interface PermissionRequest {
  requestId: string | number | null;
  type: "permission" | "question";
  title: string;
  detail: string;
  options: PermissionOption[];
}

export interface ProviderModelCatalog {
  status: "loading" | "ready" | "error";
  models: ModelOption[];
  selectedModelId?: string;
  reasoningOptions: ReasoningOption[];
  selectedReasoningId?: string;
  reasoningOptionsByModel?: Record<string, ReasoningModelOption>;
  fastModeConfigId?: string;
  fastModeModelId?: string;
  fastModeOptionsByModel?: Record<string, FastModeOption>;
  fastModeEnabled?: boolean;
  fastModeDescription?: string;
  error?: string;
}

export interface ComposerImage {
  id: string;
  type: "image";
  data: string;
  mimeType: string;
  name: string;
  previewUrl: string;
}
