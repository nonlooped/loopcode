import { providerDefinitionById, providerDefinitions } from "../config/provider-definitions.ts";
import { applyFastModeForSelectedModel } from "./fast-mode.ts";
import { applyReasoningForSelectedModel } from "./reasoning-options.ts";
import { newThreadTitle } from "./thread-title.ts";
import type {
  HarnessProfile,
  ProviderModelCatalog,
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
    title: newThreadTitle(now),
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

export function compareSidebarThreads(left: ThreadState, right: ThreadState) {
  const rank = (thread: ThreadState) => {
    const status = threadStatus(thread);
    if (status === "running") return 0;
    if (status === "error") return 1;
    if (status === "connecting") return 2;
    return 3;
  };
  return rank(left) - rank(right) || right.updatedAt - left.updatedAt;
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
