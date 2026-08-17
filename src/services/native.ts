import type { AnyMessage } from "@agentclientprotocol/sdk";
import { Channel, invoke } from "@tauri-apps/api/core";

export interface LaunchRequest {
  command: string;
  args: string[];
  cwd: string;
  profileId?: string;
  threadId?: string;
}

export type BrokerEvent =
  | { event: "rpc"; data: { message: AnyMessage } }
  | { event: "stderr"; data: { line: string } }
  | { event: "exited"; data: { code: number | null; success: boolean } }
  | { event: "error"; data: { message: string } };

export async function launchHarness(
  request: LaunchRequest,
  onMessage: (event: BrokerEvent) => void,
): Promise<string> {
  const onEvent = new Channel<BrokerEvent>();
  onEvent.onmessage = onMessage;
  const result = await invoke<{ harnessId: string }>("launch_harness", { request, onEvent });
  return result.harnessId;
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

export function getInitialWorkingDirectory(): Promise<string> {
  return invoke("initial_working_directory");
}

export function loadWorkspace(): Promise<unknown> {
  return invoke("load_workspace");
}

export function saveWorkspace(workspace: unknown): Promise<void> {
  return invoke("save_workspace", { workspace });
}

export function recordDiagnostic(level: string, eventName: string, fields: unknown): Promise<void> {
  return invoke("record_diagnostic", { level, eventName, fields });
}

export function exportDiagnostics(): Promise<string | null> {
  return invoke("export_diagnostics");
}

export function pickFolder(): Promise<string | null> {
  return invoke<string | null>("pick_folder");
}

export function getGitBranch(cwd: string): Promise<string | null> {
  return invoke<string | null>("get_git_branch", { cwd });
}
