import claudeIcon from "../assets/providers/claude.svg?url";
import openAiIcon from "../assets/providers/openai.svg?url";

import { providerDefinitions } from "./provider-definitions.ts";
import type { HarnessProfile } from "../types/index.ts";

export const profiles: HarnessProfile[] = providerDefinitions.map((profile) => ({
  ...profile,
  icon: profile.id === "claude" ? claudeIcon : openAiIcon,
  iconMode: profile.id === "claude" ? "brand" : "theme",
}));

export function profileById(profileId: string) {
  return profiles.find((profile) => profile.id === profileId);
}
