import type { DesktopPlatform, HarnessProfile } from "../types/index.ts";

export type ProviderDefinition = Omit<HarnessProfile, "icon" | "iconMode">;

export const GROK_VERSION = "1.0.5";
export const PI_VERSION = "0.84.2";
export const PI_ACP_VERSION = "0.17.1";

export const currentPlatform: DesktopPlatform =
  import.meta.env?.TAURI_ENV_PLATFORM === "windows" ? "windows" : "linux";
const isWindows = currentPlatform === "windows";
const npx = isWindows ? "npx.cmd" : "npx";

export const providerDefinitions: ProviderDefinition[] = [
  {
    id: "codex",
    label: "Codex",
    command: npx,
    args: ["--yes", "@agentclientprotocol/codex-acp@1.4.0"],
    versionCommand: isWindows ? "codex.cmd" : "codex",
    versionArgs: ["--version"],
    platforms: ["linux", "windows"],
    supportsImages: true,
    titleGeneration: true,
    probeModelOptions: true,
    installCommand: "npm install --global @openai/codex",
    loginCommand: "codex login",
  },
  {
    id: "claude",
    label: "Claude",
    command: npx,
    args: ["--yes", "@agentclientprotocol/claude-agent-acp@0.69.0"],
    versionCommand: isWindows ? "claude.cmd" : "claude",
    versionArgs: ["--version"],
    authCommand: isWindows ? "claude.cmd" : "claude",
    authArgs: ["auth", "status"],
    platforms: ["linux", "windows"],
    supportsImages: true,
    titleGeneration: true,
    probeModelOptions: true,
    installCommand: "npm install --global @anthropic-ai/claude-code",
    loginCommand: "claude auth login",
  },
  {
    id: "opencode",
    label: "OpenCode",
    command: "opencode",
    args: ["acp"],
    versionCommand: "opencode",
    versionArgs: ["--version"],
    platforms: ["linux", "windows"],
    supportsImages: true,
    titleGeneration: true,
    probeModelOptions: true,
    installCommand: "npm install --global opencode-ai",
    loginCommand: "opencode auth login",
  },
  {
    id: "cursor",
    label: "Cursor",
    command: isWindows ? "agent.cmd" : "agent",
    args: ["acp"],
    versionCommand: isWindows ? "agent.cmd" : "agent",
    versionArgs: ["--version"],
    platforms: ["linux", "windows"],
    supportsImages: true,
    titleGeneration: true,
    probeModelOptions: true,
    installCommand: isWindows
      ? "irm 'https://cursor.com/install?win32=true' | iex"
      : "curl https://cursor.com/install -fsS | bash",
    loginCommand: "agent login",
  },
  {
    id: "grok",
    label: "Grok",
    command: isWindows ? "grok.exe" : "grok",
    args: ["agent", "stdio"],
    versionCommand: isWindows ? "grok.exe" : "grok",
    versionArgs: ["version"],
    platforms: ["linux", "windows"],
    supportsImages: false,
    titleGeneration: false,
    probeModelOptions: false,
    installCommand: isWindows
      ? `irm https://x.ai/cli/install.ps1 | iex; grok update --version ${GROK_VERSION}`
      : `curl -fsSL https://x.ai/cli/install.sh | bash -s ${GROK_VERSION}`,
    loginCommand: "grok login",
  },
  {
    id: "pi",
    label: "Pi",
    command: npx,
    args: ["--yes", `@victor-software-house/pi-acp@${PI_ACP_VERSION}`],
    versionCommand: isWindows ? "pi.cmd" : "pi",
    versionArgs: ["--version"],
    platforms: ["linux", "windows"],
    supportsImages: true,
    titleGeneration: true,
    probeModelOptions: false,
    installCommand: `npm install --global @earendil-works/pi-coding-agent@${PI_VERSION}`,
    loginCommand: "Run pi, then enter /login",
  },
  {
    id: "fx",
    label: "fx",
    command: "fx",
    args: ["acp"],
    versionCommand: "fx",
    versionArgs: ["--version"],
    platforms: ["linux"],
    supportsImages: false,
    titleGeneration: false,
    probeModelOptions: false,
    installCommand: "curl -fsSL https://fx.sh/setup.sh | bash",
    loginCommand: "fx login",
  },
];

export function providerSupportsPlatform(
  profile: Pick<ProviderDefinition, "platforms">,
  platform = currentPlatform,
) {
  return profile.platforms.includes(platform);
}

export function providerDefinitionById(profileId: string) {
  return providerDefinitions.find((profile) => profile.id === profileId);
}
