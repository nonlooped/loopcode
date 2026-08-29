import { z } from "zod";

import { providerDefinitionById, providerDefinitions } from "../config/provider-definitions.ts";
import type {
  PersistedWorkspace,
  ProjectState,
  ProviderModelCatalog,
  ThreadState,
  ToolActivity,
} from "../types/index.ts";
import type { JsonValue } from "./json.ts";
import { copyPromptPart } from "./messages.ts";
import { createThread } from "./threads.ts";

export interface RestoredWorkspace {
  threads: ThreadState[];
  selectedThreadId: string;
  projects: ProjectState[];
  selectedProjectId: string | null;
  providerRepairs: { threadId: string; persistedProfileId: string; profileId: string }[];
}

const nonEmptyString = z.string().min(1);
const referenceSchema = z.object({
  id: nonEmptyString,
  kind: z.enum(["file", "folder", "skill"]),
  name: nonEmptyString,
  path: nonEmptyString,
  relativePath: nonEmptyString,
  uri: nonEmptyString,
});
const promptPartSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("text"), text: z.string() }),
  z.object({ type: z.literal("reference"), reference: referenceSchema }),
]);
const failureSchema = z.object({
  id: nonEmptyString,
  revision: z.number().int().positive(),
  category: z.enum(["connection", "access", "limit", "request", "service", "unknown"]),
  severity: z.enum(["warning", "error"]),
  title: z.string(),
  details: z.string().optional(),
  actions: z.array(z.enum(["retry", "login", "new_session"])),
});
const messageSchema = z.object({
  id: nonEmptyString,
  role: z.enum(["user", "agent", "thought", "notice", "error"]),
  text: z.string(),
  content: z.array(promptPartSchema).optional(),
  images: z
    .array(
      z.object({
        data: nonEmptyString,
        mimeType: z.string().regex(/^image\/[a-z0-9.+-]+$/i),
        name: z
          .string()
          .optional()
          .transform((name) => name || "Attached image"),
      }),
    )
    .optional(),
  createdAt: z.number().finite(),
  failure: failureSchema.optional(),
  followUp: z.boolean().optional(),
});
const childToolSchema = z
  .object({
    id: nonEmptyString,
    title: nonEmptyString,
    kind: nonEmptyString,
    status: nonEmptyString,
    detail: z.string().optional(),
    locations: z.array(z.string()),
    createdAt: z.number().finite(),
  })
  .passthrough();
const toolSchema = z.object({
  id: nonEmptyString,
  title: nonEmptyString,
  kind: nonEmptyString,
  status: nonEmptyString,
  detail: z.string().optional(),
  diffs: z
    .array(
      z.object({
        path: z.string(),
        oldText: z.string().nullable(),
        newText: z.string().nullable(),
        kind: z.enum(["add", "update", "delete"]).optional(),
      }),
    )
    .optional()
    .catch(undefined),
  terminal: z
    .object({
      output: z.string(),
      exitCode: z.number().nullable().optional(),
    })
    .optional()
    .catch(undefined),
  plan: z
    .array(
      z.object({
        content: nonEmptyString,
        status: z.enum(["pending", "in_progress", "completed"]).catch("pending"),
      }),
    )
    .optional()
    .catch(undefined),
  media: z
    .array(z.object({ data: nonEmptyString, mimeType: nonEmptyString, name: nonEmptyString }))
    .optional()
    .catch(undefined),
  presentation: z
    .enum(["image", "review", "compaction", "subagent", "background"])
    .optional()
    .catch(undefined),
  subagent: z
    .object({
      threadId: z.string().optional(),
      path: z.string().optional(),
      activity: z.string().optional(),
      senderThreadId: z.string().optional(),
      receiverThreadIds: z.array(z.string()).optional(),
    })
    .optional()
    .catch(undefined),
  children: z
    .array(z.union([messageSchema, childToolSchema]))
    .optional()
    .transform((children) => children as ToolActivity["children"]),
  locations: z.array(z.string()).catch([]),
  createdAt: z.number().finite(),
});
const projectSchema = z.object({
  id: nonEmptyString,
  name: nonEmptyString,
  path: nonEmptyString,
  createdAt: z.number().finite(),
});
const threadSchema = z.object({
  id: nonEmptyString,
  title: nonEmptyString,
  profileId: z.string().catch(""),
  cwd: nonEmptyString.catch(""),
  messages: z.array(messageSchema).catch([]),
  tools: z.array(toolSchema).catch([]),
  draft: z.string().catch(""),
  draftReferences: z.array(referenceSchema).catch([]),
  updatedAt: z.number().finite(),
  settled: z.boolean().optional().catch(undefined),
  projectId: nonEmptyString.nullable().optional().catch(undefined),
  managedWorktree: z.boolean().optional().catch(undefined),
  providerSessionIds: z.record(z.string(), nonEmptyString).catch({}),
});
const workspaceSchema = z.object({
  version: z.literal(2),
  selectedThreadId: nonEmptyString.optional(),
  selectedProjectId: nonEmptyString.nullable().optional(),
  threads: z.array(z.unknown()),
  projects: z.array(z.unknown()),
});

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
        content: message.content?.map(copyPromptPart),
        images: message.images?.map((image) => ({ ...image })),
      })),
      tools: thread.tools.map((tool) => ({
        ...tool,
        diffs: tool.diffs?.map((diff) => ({ ...diff })),
        terminal: tool.terminal ? { ...tool.terminal } : undefined,
        plan: tool.plan?.map((entry) => ({ ...entry })),
        media: tool.media?.map((image) => ({ ...image })),
        subagent: tool.subagent
          ? {
              ...tool.subagent,
              receiverThreadIds: tool.subagent.receiverThreadIds
                ? [...tool.subagent.receiverThreadIds]
                : undefined,
            }
          : undefined,
        children: tool.children?.map((entry) => ({ ...entry })),
        locations: [...tool.locations],
      })),
      draft: thread.draft,
      draftReferences: thread.draftReferences.map((reference) => ({ ...reference })),
      updatedAt: thread.updatedAt,
      settled: thread.settled,
      projectId: thread.projectId ?? null,
      managedWorktree: thread.managedWorktree || undefined,
      providerSessionIds: providerSessionIds(thread),
    })),
    projects: projects.map((project) => ({ ...project })),
  };
}

export function restoreWorkspace(
  value: JsonValue,
  defaultWorkingFolder: string,
  catalogs: Record<string, ProviderModelCatalog>,
): RestoredWorkspace | undefined {
  const workspace = workspaceSchema.safeParse(value).data;
  if (!workspace) return undefined;

  const restoredThreads: ThreadState[] = [];
  const providerRepairs: RestoredWorkspace["providerRepairs"] = [];
  const seenIds = new Set<string>();
  for (const item of workspace.threads) {
    const thread = restoreThread(item, defaultWorkingFolder, catalogs, providerRepairs);
    if (!thread || seenIds.has(thread.id)) continue;
    seenIds.add(thread.id);
    restoredThreads.push(thread);
  }
  if (restoredThreads.length === 0) return undefined;

  const restoredProjects: ProjectState[] = [];
  const seenProjectIds = new Set<string>();
  for (const item of workspace.projects) {
    const project = projectSchema.safeParse(item).data;
    if (!project || seenProjectIds.has(project.id)) continue;
    seenProjectIds.add(project.id);
    restoredProjects.push(project);
  }
  const restoredSelectedProjectId =
    workspace.selectedProjectId &&
    restoredProjects.some((project) => project.id === workspace.selectedProjectId)
      ? workspace.selectedProjectId
      : null;
  for (const thread of restoredThreads) {
    if (thread.projectId && !restoredProjects.some((project) => project.id === thread.projectId)) {
      thread.projectId = null;
    }
  }

  return {
    threads: restoredThreads,
    selectedThreadId: restoredThreads.some((thread) => thread.id === workspace.selectedThreadId)
      ? workspace.selectedThreadId!
      : restoredThreads[0].id,
    projects: restoredProjects,
    selectedProjectId: restoredSelectedProjectId,
    providerRepairs,
  };
}

function restoreThread(
  value: unknown,
  defaultWorkingFolder: string,
  catalogs: Record<string, ProviderModelCatalog>,
  providerRepairs: RestoredWorkspace["providerRepairs"],
): ThreadState | undefined {
  const persisted = threadSchema.safeParse(value).data;
  if (!persisted) return undefined;

  const profile =
    providerDefinitionById(persisted.profileId) ??
    providerDefinitions.find((candidate) => catalogs[candidate.id]?.status === "ready") ??
    providerDefinitions[0];
  if (profile.id !== persisted.profileId) {
    providerRepairs.push({
      threadId: persisted.id,
      persistedProfileId: persisted.profileId,
      profileId: profile.id,
    });
  }
  const thread = {
    ...createThread(defaultWorkingFolder, null, catalogs),
    id: persisted.id,
    title: persisted.title,
    profileId: profile.id,
    cwd: persisted.cwd || defaultWorkingFolder,
    messages: persisted.messages,
    tools: persisted.tools,
    draft: persisted.draft,
    draftReferences: persisted.draftReferences,
    updatedAt: persisted.updatedAt,
    settled: persisted.settled === true,
    projectId: persisted.projectId ?? null,
    managedWorktree: persisted.managedWorktree === true,
  };
  for (const [profileId, sessionId] of Object.entries(persisted.providerSessionIds)) {
    if (thread.providers[profileId]) thread.providers[profileId].sessionId = sessionId;
  }
  return thread;
}

function providerSessionIds(thread: ThreadState) {
  const entries = Object.entries(thread.providers).flatMap(([profileId, provider]) =>
    provider.sessionId ? [[profileId, provider.sessionId] as const] : [],
  );
  return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}
