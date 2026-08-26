import {
  providerSupportsPlatform,
  type ProviderDefinition,
} from "../config/provider-definitions.ts";
import type {
  HarnessProfile,
  ModelOption,
  ProviderModelCatalog,
  ProviderUnavailableReason,
} from "../types/index.ts";

export interface TitleGenerationPreference {
  profileId: string;
  modelId: string;
}

export function initialProviderCatalog(profile: HarnessProfile): ProviderModelCatalog {
  return providerSupportsPlatform(profile)
    ? { status: "loading", models: [], reasoningOptions: [] }
    : {
        status: "unavailable",
        models: [],
        reasoningOptions: [],
        unavailableReason: "unsupported-platform",
        error: `${profile.label} is not available on this platform.`,
      };
}

export function previewProviderCatalog(profile: HarnessProfile): ProviderModelCatalog {
  const defaultModelId = `${profile.id}-default`;
  return {
    status: "ready",
    models: [
      { id: defaultModelId, name: `${profile.label} default` },
      { id: `${profile.id}-fast`, name: `${profile.label} fast` },
    ],
    selectedModelId: defaultModelId,
    reasoningOptions: [
      { id: "default", name: "Default" },
      { id: "high", name: "High" },
    ],
    selectedReasoningId: "default",
  };
}

export function unavailableReason(error: unknown): ProviderUnavailableReason {
  const message = error instanceof Error ? error.message : String(error);
  if (/auth|credential|api[ _-]?key|log[ -]?in|sign[ -]?in/i.test(message)) {
    return "authentication";
  }
  if (/could not start|not found|no such file|os error 2|cannot find the file/i.test(message)) {
    return "missing-executable";
  }
  return "discovery";
}

export function firstReadyProviderId(
  profiles: Pick<HarnessProfile, "id">[],
  catalogs: Record<string, ProviderModelCatalog>,
) {
  return profiles.find((profile) => catalogs[profile.id]?.status === "ready")?.id;
}

export function readyProviderId(
  preferredProfileId: string,
  profiles: Pick<HarnessProfile, "id">[],
  catalogs: Record<string, ProviderModelCatalog>,
) {
  return catalogs[preferredProfileId]?.status === "ready"
    ? preferredProfileId
    : firstReadyProviderId(profiles, catalogs);
}

export function titleGenerationSelection(
  preference: TitleGenerationPreference,
  profiles: ProviderDefinition[],
  catalogs: Record<string, ProviderModelCatalog>,
): { profile: ProviderDefinition; model: ModelOption } | undefined {
  const profile = profiles.find((item) => item.id === preference.profileId && item.titleGeneration);
  const catalog = profile ? catalogs[profile.id] : undefined;
  if (!profile || catalog?.status !== "ready") return;
  const modelId = preference.modelId || catalog.selectedModelId;
  const model = catalog.models.find((item) => item.id === modelId);
  return model ? { profile, model } : undefined;
}

export type ProviderDisplayStatus =
  | "Authenticated"
  | "Connected"
  | "Disabled"
  | "Not installed"
  | "Not logged in";

export function providerCanToggle(
  profileId: string,
  catalog: ProviderModelCatalog | undefined,
  authenticated?: boolean,
  enabled = true,
) {
  return (
    enabled || (catalog?.status === "ready" && (profileId !== "claude" || authenticated === true))
  );
}

export function providerDisplayStatus(
  profileId: string,
  enabled: boolean,
  catalog: ProviderModelCatalog | undefined,
  authenticated?: boolean,
): ProviderDisplayStatus {
  if (!enabled) return "Disabled";
  if (catalog?.status === "unavailable" && catalog.unavailableReason === "missing-executable") {
    return "Not installed";
  }
  if (catalog?.status !== "ready" || (profileId === "claude" && authenticated !== true)) {
    return "Not logged in";
  }
  return profileId === "fx" || profileId === "opencode" || profileId === "pi"
    ? "Connected"
    : "Authenticated";
}

export function providerVersionLabel(
  catalog: ProviderModelCatalog | undefined,
  detectedVersion?: string,
) {
  if (catalog?.status === "unavailable" && catalog.unavailableReason === "missing-executable") {
    return "";
  }
  const version = detectedVersion ?? catalog?.agentVersion;
  return version ? `v${version}` : "Version unavailable";
}
