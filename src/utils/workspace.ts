import { providerDefinitionById } from "../config/provider-definitions.ts";
import type {
  MessageImage,
  PersistedWorkspace,
  ProjectState,
  ProviderModelCatalog,
  ThreadState,
  TimelineMessage,
  ToolActivity,
} from "../types/index.ts";
import { finiteNumber, isObject, stringValue } from "./json.ts";
import { createThread } from "./threads.ts";

export interface RestoredWorkspace {
  threads: ThreadState[];
  selectedThreadId: string;
  projects: ProjectState[];
  selectedProjectId: string | null;
}

export function workspaceSnapshot(
  threads: ThreadState[],
  selectedThreadId: string,
  projects: ProjectState[],
  selectedProjectId: string | null,
): PersistedWorkspace {
  return {
    version: 2,
    selectedThreadId,
    selectedProjectId,
    threads: threads.map((thread) => ({
      id: thread.id,
      title: thread.title,
      profileId: thread.profileId,
      cwd: thread.cwd,
      messages: thread.messages.map((message) => ({
        ...message,
        images: message.images?.map((image) => ({ ...image })),
      })),
      tools: thread.tools.map((tool) => ({ ...tool, locations: [...tool.locations] })),
      draft: thread.draft,
      updatedAt: thread.updatedAt,
      settled: thread.settled,
      projectId: thread.projectId ?? null,
      providerSessionIds: providerSessionIds(thread),
    })),
    projects: projects.map((project) => ({ ...project })),
  };
}

export function restoreWorkspace(
  value: unknown,
  defaultWorkingFolder: string,
  catalogs: Record<string, ProviderModelCatalog>,
): RestoredWorkspace | undefined {
  if (!isObject(value) || !Array.isArray(value.threads)) return undefined;
  const version = value.version === 2 ? 2 : value.version === 1 ? 1 : undefined;
  if (!version) return undefined;

  const restoredThreads: ThreadState[] = [];
  const seenIds = new Set<string>();
  for (const item of value.threads) {
    const thread = restoreThread(item, defaultWorkingFolder, catalogs);
    if (!thread || seenIds.has(thread.id)) continue;
    seenIds.add(thread.id);
    restoredThreads.push(thread);
  }
  if (restoredThreads.length === 0) return undefined;

  const persistedSelection = stringValue(value.selectedThreadId);
  const restoredProjects: ProjectState[] = [];
  let restoredSelectedProjectId: string | null = null;
  if (version === 2 && Array.isArray(value.projects)) {
    const seenProjectIds = new Set<string>();
    for (const item of value.projects) {
      const project = restoreProject(item);
      if (!project || seenProjectIds.has(project.id)) continue;
      seenProjectIds.add(project.id);
      restoredProjects.push(project);
    }
    const selection = stringValue(value.selectedProjectId);
    if (selection && restoredProjects.some((project) => project.id === selection)) {
      restoredSelectedProjectId = selection;
    }
    for (const thread of restoredThreads) {
      if (
        thread.projectId &&
        !restoredProjects.some((project) => project.id === thread.projectId)
      ) {
        thread.projectId = null;
      }
    }
  }

  return {
    threads: restoredThreads,
    selectedThreadId: restoredThreads.some((thread) => thread.id === persistedSelection)
      ? persistedSelection!
      : restoredThreads[0].id,
    projects: restoredProjects,
    selectedProjectId: restoredSelectedProjectId,
  };
}

function restoreProject(value: unknown): ProjectState | undefined {
  if (!isObject(value)) return undefined;
  const id = stringValue(value.id);
  const name = stringValue(value.name);
  const path = typeof value.path === "string" ? value.path : undefined;
  const createdAt = finiteNumber(value.createdAt);
  if (!id || !name || !path || createdAt === undefined) return undefined;
  return { id, name, path, createdAt };
}

function restoreThread(
  value: unknown,
  defaultWorkingFolder: string,
  catalogs: Record<string, ProviderModelCatalog>,
): ThreadState | undefined {
  if (!isObject(value)) return undefined;
  const id = stringValue(value.id);
  const title = stringValue(value.title);
  if (!id || !title) return undefined;

  const profile = providerDefinitionById(
    typeof value.profileId === "string" ? value.profileId : "",
  );
  const messages = Array.isArray(value.messages)
    ? value.messages.map(restoreMessage).filter((message) => message !== undefined)
    : [];
  const tools = Array.isArray(value.tools)
    ? value.tools.map(restoreTool).filter((tool) => tool !== undefined)
    : [];
  const updatedAt =
    finiteNumber(value.updatedAt) ??
    Math.max(
      0,
      ...messages.map((message) => message.createdAt),
      ...tools.map((tool) => tool.createdAt),
    );
  const thread = {
    ...createThread(defaultWorkingFolder, null, catalogs),
    id,
    title,
    profileId: profile.id,
    cwd: typeof value.cwd === "string" && value.cwd ? value.cwd : defaultWorkingFolder,
    messages,
    tools,
    draft: typeof value.draft === "string" ? value.draft : "",
    updatedAt,
    settled: value.settled === true,
    projectId: typeof value.projectId === "string" ? value.projectId : null,
  };
  if (isObject(value.providerSessionIds)) {
    for (const [profileId, sessionId] of Object.entries(value.providerSessionIds)) {
      if (thread.providers[profileId] && typeof sessionId === "string" && sessionId) {
        thread.providers[profileId].sessionId = sessionId;
      }
    }
  }
  return thread;
}

function providerSessionIds(thread: ThreadState) {
  const entries = Object.entries(thread.providers).flatMap(([profileId, provider]) =>
    provider.sessionId ? [[profileId, provider.sessionId] as const] : [],
  );
  return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

function restoreMessage(value: unknown): TimelineMessage | undefined {
  if (!isObject(value)) return undefined;
  const id = stringValue(value.id);
  const text = typeof value.text === "string" ? value.text : undefined;
  const createdAt = finiteNumber(value.createdAt);
  const role = value.role;
  if (
    !id ||
    text === undefined ||
    createdAt === undefined ||
    (role !== "user" &&
      role !== "agent" &&
      role !== "thought" &&
      role !== "notice" &&
      role !== "error")
  ) {
    return undefined;
  }
  return { id, role, text, images: restoreMessageImages(value.images), createdAt };
}

function restoreMessageImages(value: unknown): MessageImage[] | undefined {
  if (!Array.isArray(value)) return undefined;
  const images = value.filter(isObject).flatMap((image): MessageImage[] => {
    const attachmentId = stringValue(image.attachmentId);
    const mimeType = stringValue(image.mimeType);
    const name = stringValue(image.name) ?? "Attached image";
    if (!mimeType?.match(/^image\/[a-z0-9.+-]+$/i)) return [];
    if (attachmentId?.match(/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i)) {
      return [{ attachmentId, mimeType, name }];
    }
    const data = stringValue(image.data);
    return data ? [{ data, mimeType, name }] : [];
  });
  return images.length > 0 ? images : undefined;
}

function restoreTool(value: unknown): ToolActivity | undefined {
  if (!isObject(value)) return undefined;
  const id = stringValue(value.id);
  const title = stringValue(value.title);
  const kind = stringValue(value.kind);
  const status = stringValue(value.status);
  const createdAt = finiteNumber(value.createdAt);
  if (!id || !title || !kind || !status || createdAt === undefined) return undefined;
  return {
    id,
    title,
    kind,
    status,
    detail: typeof value.detail === "string" ? value.detail : undefined,
    locations: Array.isArray(value.locations)
      ? value.locations.filter((location): location is string => typeof location === "string")
      : [],
    createdAt,
  };
}
