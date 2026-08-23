import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

const WORKSPACE_KEY = "loopcode.web-preview.workspace";
let previewGitBranch = "web-preview";

export function setupWebPreview() {
  mockWindows("main");
  mockIPC(
    (command, payload) => {
      switch (command) {
        case "register_frontend":
          return 1;
        case "initial_working_directory":
          return "/workspace/loopcode";
        case "load_workspace":
          return loadWorkspace();
        case "save_workspace":
          localStorage.setItem(
            WORKSPACE_KEY,
            JSON.stringify(argument(payload, "workspace") ?? null),
          );
          return null;
        case "get_git_branch":
          return previewGitBranch;
        case "list_git_branches":
          return ["main", "web-preview", previewGitBranch];
        case "switch_git_branch": {
          const branch = argument(payload, "branch");
          if (typeof branch === "string") previewGitBranch = branch;
          return null;
        }
        case "create_git_worktree": {
          const branch = argument(payload, "branch");
          if (typeof branch === "string") previewGitBranch = branch;
          return { path: `/workspace/worktrees/${previewGitBranch}`, branch: previewGitBranch };
        }
        case "provider_version":
          return "0.0.0-preview";
        case "provider_auth_status":
          return true;
        case "pick_folder":
          return "/workspace/example";
        case "list_composer_completions":
        case "read_project_directory":
          return [];
        case "read_project_file":
          return new TextEncoder().encode("Web preview file content\n").buffer;
        case "start_project_file_watcher":
          return 1;
        case "start_terminal":
          return { terminalId: "web-preview" };
        case "export_diagnostics":
          return null;
        case "launch_harness":
          throw new Error("Agent processes are unavailable in the web preview.");
        default:
          if (command.startsWith("plugin:window|is_maximized")) return false;
          if (command.startsWith("plugin:")) return null;
          return null;
      }
    },
    { shouldMockEvents: true },
  );
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
