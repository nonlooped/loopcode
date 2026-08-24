import type { AnyMessage } from "@agentclientprotocol/sdk";
import { Channel, invoke } from "@tauri-apps/api/core";
import { z } from "zod";

import type { PersistedWorkspace } from "../types/index.ts";
import { jsonValueSchema, type JsonValue } from "../utils/json.ts";

export interface LaunchRequest {
  command: string;
  args: string[];
  cwd: string;
  profileId?: string;
  threadId?: string;
}

let frontendGeneration: number | undefined;

export type BrokerEvent =
  | { event: "rpc"; data: { message: AnyMessage } }
  | { event: "stderr"; data: { line: string } }
  | { event: "exited"; data: { code: number | null; success: boolean } }
  | { event: "error"; data: { message: string } };

export interface ProjectFileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  isSymlink: boolean;
}

export interface ProjectFileChange {
  paths: string[];
}

export interface StartTerminalRequest {
  threadId: string;
  cwd: string;
  cols: number;
  rows: number;
}

const terminalEventSchema = z.discriminatedUnion("event", [
  z.object({
    event: z.literal("output"),
    data: z.object({ bytes: z.array(z.number().int().min(0).max(255)).max(8192) }),
  }),
  z.object({
    event: z.literal("exited"),
    data: z.object({ code: z.number().int().nonnegative(), success: z.boolean() }),
  }),
  z.object({ event: z.literal("error"), data: z.object({ message: z.string() }) }),
]);

const startTerminalResultSchema = z.object({ terminalId: z.string().min(1) });
const gitBranchSchema = z.string().min(1).max(1024);
const gitBranchesSchema = z.array(gitBranchSchema).max(10_000);
const gitWorktreeSchema = z.object({ path: z.string().min(1), branch: gitBranchSchema });
const gitChangeSchema = z.object({
  path: z.string().min(1).max(4096),
  oldPath: z.string().min(1).max(4096).nullable(),
  status: z.enum([
    "added",
    "modified",
    "deleted",
    "renamed",
    "copied",
    "untracked",
    "conflicted",
    "typeChanged",
  ]),
  staged: z.boolean(),
  unstaged: z.boolean(),
});
const gitChangesSchema = z.object({
  changes: z.array(gitChangeSchema).max(10_000),
  additions: z.number().int().nonnegative(),
  deletions: z.number().int().nonnegative(),
});
const gitFileDiffSchema = z.object({
  hunks: z.array(z.string().max(2 * 1024 * 1024)).max(10_000),
  binary: z.boolean(),
  tooLarge: z.boolean(),
});

export type GitChange = z.infer<typeof gitChangeSchema>;
export type GitChanges = z.infer<typeof gitChangesSchema>;
export type GitFileDiff = z.infer<typeof gitFileDiffSchema>;
export type TerminalEvent = z.infer<typeof terminalEventSchema>;

export interface ComposerCompletionEntry {
  kind: "file" | "folder" | "skill";
  name: string;
  path: string;
  relativePath: string;
  description?: string;
}

export async function launchHarness(
  request: LaunchRequest,
  onMessage: (event: BrokerEvent) => void,
): Promise<string> {
  if (frontendGeneration === undefined) throw new Error("The frontend is not registered");
  const onEvent = new Channel<BrokerEvent>();
  onEvent.onmessage = onMessage;
  const result = await invoke<{ harnessId: string }>("launch_harness", {
    request: { ...request, frontendGeneration },
    onEvent,
  });
  return result.harnessId;
}

export async function registerFrontend() {
  frontendGeneration = await invoke<number>("register_frontend");
}

export function sendRpc(harnessId: string, message: AnyMessage): Promise<void> {
  return invoke("send_rpc", { harnessId, message });
}

export function stopHarness(harnessId: string): Promise<void> {
  return invoke("stop_harness", { harnessId });
}

export function stopAllHarnesses(): Promise<void> {
  return invoke("stop_all_harnesses");
}

export async function startTerminal(
  request: StartTerminalRequest,
  onMessage: (event: TerminalEvent) => void,
): Promise<string> {
  if (frontendGeneration === undefined) throw new Error("The frontend is not registered");
  const onEvent = new Channel<unknown>();
  onEvent.onmessage = (value) => {
    const event = terminalEventSchema.safeParse(value);
    onMessage(
      event.success
        ? event.data
        : { event: "error", data: { message: "The terminal returned an invalid event" } },
    );
  };
  const result = startTerminalResultSchema.parse(
    await invoke("start_terminal", {
      request: { ...request, frontendGeneration },
      onEvent,
    }),
  );
  return result.terminalId;
}

export function writeTerminal(terminalId: string, data: string): Promise<void> {
  return invoke("write_terminal", { terminalId, data });
}

export function resizeTerminal(terminalId: string, cols: number, rows: number): Promise<void> {
  return invoke("resize_terminal", { terminalId, cols, rows });
}

export function stopTerminal(terminalId: string): Promise<void> {
  return invoke("stop_terminal", { terminalId });
}

export function stopTerminalForThread(threadId: string): Promise<void> {
  return invoke("stop_terminal_for_thread", { threadId });
}

export function stopAllTerminals(): Promise<void> {
  return invoke("stop_all_terminals");
}

export function getInitialWorkingDirectory(): Promise<string> {
  return invoke("initial_working_directory");
}

export async function loadWorkspace(): Promise<JsonValue | null> {
  return jsonValueSchema.nullable().parse(await invoke("load_workspace"));
}

export function saveWorkspace(workspace: PersistedWorkspace): Promise<void> {
  return invoke("save_workspace", { workspace });
}

export function recordDiagnostic(
  level: string,
  eventName: string,
  fields: Record<string, JsonValue | undefined>,
): Promise<void> {
  return invoke("record_diagnostic", { level, eventName, fields });
}

export function exportDiagnostics(): Promise<string | null> {
  return invoke("export_diagnostics");
}

export function pickFolder(): Promise<string | null> {
  return invoke<string | null>("pick_folder");
}

export async function getGitBranch(cwd: string): Promise<string | null> {
  return gitBranchSchema.nullable().parse(await invoke("get_git_branch", { cwd }));
}

export async function listGitBranches(cwd: string): Promise<string[]> {
  return gitBranchesSchema.parse(await invoke("list_git_branches", { cwd }));
}

export async function listGitChanges(cwd: string, baseBranch: string | null): Promise<GitChanges> {
  return gitChangesSchema.parse(await invoke("list_git_changes", { cwd, baseBranch }));
}

export async function getGitFileDiff(
  cwd: string,
  baseBranch: string | null,
  path: string,
  oldPath: string | null,
): Promise<GitFileDiff> {
  return gitFileDiffSchema.parse(
    await invoke("get_git_file_diff", {
      cwd,
      baseBranch,
      path,
      oldPath,
    }),
  );
}

export function switchGitBranch(cwd: string, branch: string): Promise<void> {
  return invoke("switch_git_branch", { cwd, branch });
}

export async function createGitWorktree(
  cwd: string,
  baseBranch: string,
  branch: string,
): Promise<{ path: string; branch: string }> {
  return gitWorktreeSchema.parse(await invoke("create_git_worktree", { cwd, baseBranch, branch }));
}

export async function getProviderVersion(command: string, args: string[]): Promise<string | null> {
  return z
    .string()
    .max(512)
    .nullable()
    .parse(await invoke("provider_version", { command, args }));
}

export async function getProviderAuthStatus(
  command: string,
  args: string[],
): Promise<boolean | null> {
  return z
    .boolean()
    .nullable()
    .parse(await invoke("provider_auth_status", { command, args }));
}

export function listComposerCompletions(projectRoot: string): Promise<ComposerCompletionEntry[]> {
  return invoke<ComposerCompletionEntry[]>("list_composer_completions", { projectRoot });
}

export function readProjectDirectory(
  projectRoot: string,
  directory: string,
): Promise<ProjectFileEntry[]> {
  return invoke<ProjectFileEntry[]>("read_project_directory", {
    projectRoot,
    directory,
  });
}

export function readProjectFile(projectRoot: string, path: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("read_project_file", { projectRoot, path });
}

export function openProjectPath(projectRoot: string, path: string): Promise<void> {
  return invoke("open_project_path", { projectRoot, path });
}

export function revealProjectPath(projectRoot: string, path: string): Promise<void> {
  return invoke("reveal_project_path", { projectRoot, path });
}

export async function startProjectFileWatcher(
  projectRoot: string,
  onChange: (change: ProjectFileChange) => void,
): Promise<number> {
  const channel = new Channel<ProjectFileChange>();
  channel.onmessage = onChange;
  return invoke<number>("start_project_file_watcher", {
    projectRoot,
    onChange: channel,
  });
}

export function stopProjectFileWatcher(watcherId: number): Promise<void> {
  return invoke("stop_project_file_watcher", { watcherId });
}
