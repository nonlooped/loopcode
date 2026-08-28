import openAiIcon from "@lobehub/icons-static-svg/icons/openai.svg?url";

import { providerDefinitions } from "./provider-definitions.ts";
import type { HarnessProfile } from "../types/index.ts";

const icons = new Map(Object.entries({ codex: openAiIcon }));

export const profiles: HarnessProfile[] = providerDefinitions.map((profile) => {
  const icon = icons.get(profile.id);
  if (!icon) throw new Error(`Missing icon for provider ${profile.id}`);
  return { ...profile, icon, iconMode: "theme" };
});

export function profileById(profileId: string) {
  return profiles.find((profile) => profile.id === profileId);
}
