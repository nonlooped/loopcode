import type { DesktopPlatform, HarnessProfile } from "../types/index.ts";

export type ProviderDefinition = Omit<HarnessProfile, "icon" | "iconMode">;

export function desktopPlatform(
  tauriPlatform = import.meta.env?.TAURI_ENV_PLATFORM,
): DesktopPlatform {
  if (tauriPlatform === "windows") return "windows";
  if (tauriPlatform === "darwin" || tauriPlatform === "macos") return "macos";
  return "linux";
}

export const currentPlatform: DesktopPlatform = desktopPlatform();
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
    platforms: ["linux", "macos", "windows"],
    supportsImages: true,
    titleGeneration: true,
    probeModelOptions: true,
    installCommand: "npm install --global @openai/codex",
    loginCommand: "codex login",
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
