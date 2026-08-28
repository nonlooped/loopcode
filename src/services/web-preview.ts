import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { InvokeArgs } from "@tauri-apps/api/core";

const WORKSPACE_KEY = "loopcode.web-preview.workspace";
let previewGitBranch = "web-preview";

export function setupWebPreview() {
  mockWindows("main");
  mockIPC(handlePreviewCommand, { shouldMockEvents: true });
}

const previewCommands: Record<string, (payload?: InvokeArgs) => unknown> = {
  register_frontend: () => 1,
  initial_working_directory: () => "/workspace/loopcode",
  load_workspace: () => loadWorkspace(),
  save_workspace: (payload) => {
    localStorage.setItem(WORKSPACE_KEY, JSON.stringify(argument(payload, "workspace") ?? null));
    return null;
  },
  get_git_branch: () => previewGitBranch,
  list_git_branches: () => ["main", "web-preview", previewGitBranch],
  list_git_changes: () => ({
    changes: [
      {
        path: "src/App.svelte",
        oldPath: null,
        status: "modified",
        staged: false,
        unstaged: true,
      },
      {
        path: "src/components/ChangesPanel.svelte",
        oldPath: null,
        status: "untracked",
        staged: false,
        unstaged: true,
      },
    ],
    additions: 114,
    deletions: 7,
  }),
  get_git_file_diff: () => ({
    hunks: [
      "@@ -12,3 +12,3 @@\n import Sidebar from './components/Sidebar.svelte';\n-import Transcript from './components/Transcript.svelte';\n+import ChangesPanel from './components/ChangesPanel.svelte';\n import Titlebar from './components/Titlebar.svelte';\n",
    ],
    binary: false,
    tooLarge: false,
  }),
  switch_git_branch: (payload) => {
    const branch = argument(payload, "branch");
    if (typeof branch === "string") previewGitBranch = branch;
    return null;
  },
  create_git_worktree: (payload) => {
    const branch = argument(payload, "branch");
    if (typeof branch === "string") previewGitBranch = branch;
    return { path: `/workspace/worktrees/${previewGitBranch}`, branch: previewGitBranch };
  },
  provider_version: () => "0.0.0-preview",
  provider_auth_status: () => true,
  pick_folder: () => "/workspace/example",
  list_composer_completions: () => [],
  read_project_directory: () => [],
  read_project_file: () => new TextEncoder().encode("Web preview file content\n").buffer,
  start_project_file_watcher: () => 1,
  start_terminal: () => ({ terminalId: "web-preview" }),
  export_diagnostics: () => null,
  launch_harness: () => {
    throw new Error("Agent processes are unavailable in the web preview.");
  },
};

function handlePreviewCommand(command: string, payload?: InvokeArgs) {
  if (Object.hasOwn(previewCommands, command)) return previewCommands[command](payload);
  if (command.startsWith("plugin:window|is_maximized")) return false;
  return null;
}

function argument(
  payload: Record<string, unknown> | number[] | ArrayBuffer | Uint8Array | undefined,
  name: string,
) {
  if (
    !payload ||
    Array.isArray(payload) ||
    payload instanceof ArrayBuffer ||
    payload instanceof Uint8Array
  ) {
    return undefined;
  }
  return payload[name];
}

function loadWorkspace(): unknown {
  const saved = localStorage.getItem(WORKSPACE_KEY);
  if (!saved) return null;
  try {
    return JSON.parse(saved);
  } catch {
    localStorage.removeItem(WORKSPACE_KEY);
    return null;
  }
}
