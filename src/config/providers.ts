import claudeIcon from "@lobehub/icons-static-svg/icons/claude.svg?url";
import openAiIcon from "@lobehub/icons-static-svg/icons/openai.svg?url";
import openCodeIcon from "@lobehub/icons-static-svg/icons/opencode.svg?url";

import { providerDefinitions } from "./provider-definitions.ts";
import type { HarnessProfile } from "../types/index.ts";

const icons = {
  codex: openAiIcon,
  claude: claudeIcon,
  opencode: openCodeIcon,
};

export const profiles: HarnessProfile[] = providerDefinitions.map((profile) => ({
  ...profile,
  icon: icons[profile.id as keyof typeof icons],
}));

export function profileById(profileId: string) {
  return profiles.find((profile) => profile.id === profileId) ?? profiles[0];
}
