import claudeIcon from "@lobehub/icons-static-svg/icons/claude.svg?url";
import cursorIcon from "@lobehub/icons-static-svg/icons/cursor.svg?url";
import openAiIcon from "@lobehub/icons-static-svg/icons/openai.svg?url";
import openCodeIcon from "@lobehub/icons-static-svg/icons/opencode.svg?url";

import { providerDefinitions } from "./provider-definitions.ts";
import type { HarnessProfile } from "../types/index.ts";

const icons = new Map(
  Object.entries({
    codex: openAiIcon,
    claude: claudeIcon,
    opencode: openCodeIcon,
    cursor: cursorIcon,
  }),
);

export const profiles: HarnessProfile[] = providerDefinitions.map((profile) => {
  const icon = icons.get(profile.id);
  if (!icon) throw new Error(`Missing icon for provider ${profile.id}`);
  return { ...profile, icon };
});

export function profileById(profileId: string) {
  return profiles.find((profile) => profile.id === profileId) ?? profiles[0];
}
