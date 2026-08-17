import type { HarnessProfile } from "../types/index.ts";

export type ProviderDefinition = Omit<HarnessProfile, "icon">;

export const providerDefinitions: ProviderDefinition[] = [
  {
    id: "codex",
    label: "Codex",
    command: "npx.cmd",
    args: ["--yes", "@agentclientprotocol/codex-acp@1.4.0"],
  },
  {
    id: "claude",
    label: "Claude",
    command: "npx.cmd",
    args: ["--yes", "@agentclientprotocol/claude-agent-acp@0.69.0"],
  },
  {
    id: "opencode",
    label: "OpenCode",
    command: "opencode",
    args: ["acp"],
  },
];

export function providerDefinitionById(profileId: string) {
  return providerDefinitions.find((profile) => profile.id === profileId) ?? providerDefinitions[0];
}
