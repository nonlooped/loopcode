import { providerDefinitionById, providerDefinitions } from "../config/provider-definitions.ts";
import { applyFastModeForSelectedModel } from "./fast-mode.ts";
import { applyReasoningForSelectedModel } from "./reasoning-options.ts";
import type {
  HarnessProfile,
  ProviderModelCatalog,
  PermissionRequest,
  ProviderSessionState,
  ThreadState,
  ThreadStatus,
} from "../types/index.ts";

export function createProviderState(
  profileId: string,
  catalogs: Record<string, ProviderModelCatalog>,
): ProviderSessionState {
  const catalog = catalogs[profileId];
  const provider: ProviderSessionState = {
    connectionStatus: "disconnected",
    turnStatus: "idle",
    models: catalog?.models ?? [],
    selectedModelId: catalog?.selectedModelId,
    reasoningOptions: catalog?.reasoningOptions ?? [],
    selectedReasoningId: catalog?.selectedReasoningId,
    reasoningOptionsByModel: catalog?.reasoningOptionsByModel,
    fastModeOptionsByModel: catalog?.fastModeOptionsByModel,
    selects: catalog?.selects,
    supportsFollowups: catalog?.supportsFollowups,
    commands: catalog?.commands,
  };
  applyReasoningForSelectedModel(provider);
  applyFastModeForSelectedModel(provider);
  return provider;
}

export function createThread(
  cwd: string,
  projectId: string | null,
  catalogs: Record<string, ProviderModelCatalog>,
): ThreadState {
  const now = Date.now();
  return {
    id: crypto.randomUUID(),
    title: "New Thread",
    profileId: providerDefinitions[0].id,
    cwd,
    messages: [],
    tools: [],
    draft: "",
    draftReferences: [],
    providers: Object.fromEntries(
      providerDefinitions.map((profile) => [profile.id, createProviderState(profile.id, catalogs)]),
    ),
    updatedAt: now,
    settled: false,
    projectId,
    managedWorktree: false,
  };
}

export function activeProvider(thread: ThreadState) {
  return thread.providers[thread.profileId];
}

export function threadReservesCheckout(thread: ThreadState) {
  return (
    thread.messages.length > 0 ||
    thread.tools.length > 0 ||
    Object.values(thread.providers).some(
      (provider) =>
        provider.connectionStatus === "connecting" || provider.connectionStatus === "ready",
    )
  );
}

export function threadStatus(thread: ThreadState): ThreadStatus {
  const provider = activeProvider(thread);
  if (provider.turnStatus === "running") return "running";
  if (provider.turnStatus === "blocked") return "error";
  return provider.connectionStatus;
}

export type ThreadAttention =
  | { kind: "needs-approval"; label: "Needs approval"; reason: string }
  | { kind: "needs-answer"; label: "Needs answer"; reason: string }
  | { kind: "failed"; label: "Failed"; reason: string }
  | { kind: "working"; label: "Working"; reason: string }
  | { kind: "recent" };

export function threadAttention(thread: ThreadState, request?: PermissionRequest): ThreadAttention {
  if (request?.type === "permission") {
    return {
      kind: "needs-approval",
      label: "Needs approval",
      reason: request.title.trim() || "Agent action",
    };
  }
  if (request?.type === "question") {
    return {
      kind: "needs-answer",
      label: "Needs answer",
      reason: request.title.trim() || "Agent question",
    };
  }

  const provider = activeProvider(thread);
  if (
    provider.turnStatus === "failed" ||
    provider.turnStatus === "blocked" ||
    provider.connectionStatus === "error" ||
    provider.connectionStatus === "stopped"
  ) {
    return {
      kind: "failed",
      label: "Failed",
      reason:
        provider.error?.trim() ||
        (provider.connectionStatus === "stopped" ? "Provider stopped" : "Provider failed"),
    };
  }
  if (provider.turnStatus === "running") {
    const tool = [...thread.tools]
      .reverse()
      .find((candidate) => candidate.status === "pending" || candidate.status === "in_progress");
    return {
      kind: "working",
      label: "Working",
      reason: tool?.title || "Agent is working",
    };
  }
  if (provider.connectionStatus === "connecting") {
    return { kind: "working", label: "Working", reason: "Connecting to provider" };
  }
  return { kind: "recent" };
}

export function threadHarness(
  thread: ThreadState,
  profiles: Pick<HarnessProfile, "id" | "label">[] = providerDefinitions,
) {
  return (
    profiles.find((profile) => profile.id === thread.profileId)?.label ??
    providerDefinitionById(thread.profileId)?.label ??
    "Unknown provider"
  );
}

export function compareSidebarThreads(
  left: ThreadState,
  right: ThreadState,
  leftRequest?: PermissionRequest,
  rightRequest?: PermissionRequest,
) {
  const rank = (attention: ThreadAttention) => {
    switch (attention.kind) {
      case "needs-approval":
      case "needs-answer":
        return 0;
      case "failed":
        return 1;
      case "working":
        return 2;
      case "recent":
        return 3;
    }
  };
  return (
    rank(threadAttention(left, leftRequest)) - rank(threadAttention(right, rightRequest)) ||
    right.updatedAt - left.updatedAt
  );
}

export function folderName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

export function relativeTime(timestamp: number) {
  const minutes = Math.max(0, Math.floor((Date.now() - timestamp) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}
