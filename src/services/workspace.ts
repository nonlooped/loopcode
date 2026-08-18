import type {
  PersistedWorkspace,
  ProjectState,
  ProviderModelCatalog,
  ThreadState,
} from "../types/index.ts";
import { findReusableEmptyThread } from "../utils/thread-state.ts";
import { createThread, folderName } from "../utils/threads.ts";
import { restoreWorkspace, workspaceSnapshot } from "../utils/workspace.ts";
import { WorkspacePersistence } from "./workspace-persistence.ts";

export interface WorkspaceState {
  threads: ThreadState[];
  selectedThreadId: string;
  projects: ProjectState[];
  selectedProjectId: string | null;
}

type Persistence = Pick<WorkspacePersistence, "setReady" | "queue" | "flush">;

export function createWorkspaceState(
  cwd: string,
  catalogs: Record<string, ProviderModelCatalog>,
): WorkspaceState {
  const thread = createThread(cwd, null, catalogs);
  return {
    threads: [thread],
    selectedThreadId: thread.id,
    projects: [],
    selectedProjectId: null,
  };
}

export class Workspace {
  private readonly state: WorkspaceState;
  private readonly catalogs: Record<string, ProviderModelCatalog>;
  private readonly persistence: Persistence;

  constructor(
    state: WorkspaceState,
    catalogs: Record<string, ProviderModelCatalog>,
    persistence: Persistence = new WorkspacePersistence(),
  ) {
    this.state = state;
    this.catalogs = catalogs;
    this.persistence = persistence;
  }

  get activeProject() {
    return this.state.selectedProjectId
      ? (this.state.projects.find((project) => project.id === this.state.selectedProjectId) ?? null)
      : null;
  }

  initialize(saved: unknown, defaultWorkingFolder: string) {
    if (saved !== null) {
      const restored = restoreWorkspace(saved, defaultWorkingFolder, this.catalogs);
      if (!restored) return false;
      this.state.threads = restored.threads;
      this.state.selectedThreadId = restored.selectedThreadId;
      this.state.projects = restored.projects;
      this.state.selectedProjectId = restored.selectedProjectId;
    } else {
      for (const thread of this.state.threads) {
        if (!thread.cwd) thread.cwd = defaultWorkingFolder;
      }
    }
    this.persistence.setReady();
    return true;
  }

  addThread(
    defaultWorkingFolder: string,
    hasAttachments: (threadId: string) => boolean,
    projectId = this.state.selectedProjectId,
  ) {
    const project = projectId ? this.state.projects.find((item) => item.id === projectId) : null;
    if (projectId && !project) return undefined;

    this.state.selectedProjectId = project?.id ?? null;
    const target = { cwd: project?.path ?? defaultWorkingFolder, projectId: project?.id ?? null };
    const reusable = findReusableEmptyThread(this.state.threads, target, hasAttachments);
    const thread = reusable ?? createThread(target.cwd, target.projectId, this.catalogs);
    if (!reusable) this.state.threads.unshift(thread);
    this.state.selectedThreadId = thread.id;
    return thread;
  }

  selectThread(threadId: string) {
    if (!this.state.threads.some((thread) => thread.id === threadId)) return false;
    this.state.selectedThreadId = threadId;
    return true;
  }

  removeThread(threadId: string, defaultWorkingFolder: string) {
    this.state.threads = this.state.threads.filter((thread) => thread.id !== threadId);
    if (this.state.threads.length === 0) {
      const project = this.activeProject;
      this.state.threads = [
        createThread(project?.path ?? defaultWorkingFolder, project?.id ?? null, this.catalogs),
      ];
    }
    if (!this.state.threads.some((thread) => thread.id === this.state.selectedThreadId)) {
      this.state.selectedThreadId = this.state.threads[0].id;
    }
  }

  renameThread(threadId: string, title: string) {
    const thread = this.state.threads.find((item) => item.id === threadId);
    const trimmed = title.trim();
    if (!thread || !trimmed || trimmed === thread.title) return false;
    thread.title = trimmed;
    thread.updatedAt = Date.now();
    return true;
  }

  toggleSettled(threadId: string) {
    const thread = this.state.threads.find((item) => item.id === threadId);
    if (!thread) return;
    thread.settled = !thread.settled;
    thread.updatedAt = Date.now();
  }

  ensureProject(path: string) {
    const trimmed = path.trim();
    const existing = this.state.projects.find((project) => project.path === trimmed);
    if (existing) return existing;
    const project: ProjectState = {
      id: crypto.randomUUID(),
      name: folderName(trimmed) || "Untitled project",
      path: trimmed,
      createdAt: Date.now(),
    };
    this.state.projects = [...this.state.projects, project];
    return project;
  }

  selectProject(projectId: string | null) {
    if (projectId && !this.state.projects.some((project) => project.id === projectId)) return false;
    this.state.selectedProjectId = projectId;
    return true;
  }

  removeProject(projectId: string) {
    this.state.projects = this.state.projects.filter((project) => project.id !== projectId);
    for (const thread of this.state.threads) {
      if (thread.projectId === projectId) thread.projectId = null;
    }
    if (this.state.selectedProjectId === projectId) this.state.selectedProjectId = null;
  }

  projectNameForThread(thread: ThreadState) {
    const project = thread.projectId
      ? this.state.projects.find((item) => item.id === thread.projectId)
      : null;
    return project?.name ?? folderName(thread.cwd) ?? "No project";
  }

  snapshot(): PersistedWorkspace {
    return workspaceSnapshot(
      this.state.threads,
      this.state.selectedThreadId,
      this.state.projects,
      this.state.selectedProjectId,
    );
  }

  queuePersistence() {
    this.persistence.queue(this.snapshot());
  }

  flush() {
    return this.persistence.flush();
  }
}
