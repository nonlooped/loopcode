import type { HarnessProfile } from "../types/index.ts";

export type ProviderDefinition = Omit<HarnessProfile, "icon" | "iconMode">;

const isWindows = import.meta.env?.TAURI_ENV_PLATFORM === "windows";
const npx = isWindows ? "npx.cmd" : "npx";

export const providerDefinitions: ProviderDefinition[] = [
  {
    id: "codex",
    label: "Codex",
    command: npx,
    args: ["--yes", "@agentclientprotocol/codex-acp@1.4.0"],
    versionCommand: isWindows ? "codex.cmd" : "codex",
    installCommand: "npm install --global @openai/codex",
    loginCommand: "codex login",
  },
  {
    id: "claude",
    label: "Claude",
    command: npx,
    args: ["--yes", "@agentclientprotocol/claude-agent-acp@0.70.0"],
    versionCommand: isWindows ? "claude.cmd" : "claude",
    installCommand: "npm install --global @anthropic-ai/claude-code",
    loginCommand: "claude auth login",
  },
];

export function providerDefinitionById(profileId: string) {
  return providerDefinitions.find((profile) => profile.id === profileId);
}
