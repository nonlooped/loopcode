import type { ComponentType } from "react";
import {
	IconDatabase,
	IconCode,
	IconInfoCircle,
	IconPlug,
	IconPlugConnected,
	IconSettings,
} from "@tabler/icons-react";
import type { SettingsCategory } from "../../store/ui";

export type SettingsIcon = ComponentType<{
	size?: number;
	stroke?: number;
	className?: string;
	"aria-hidden"?: boolean;
}>;

export interface SettingsNavItem {
	id: SettingsCategory;
	label: string;
	description: string;
	icon: SettingsIcon;
}

/** Nav items for the settings left rail — LoopCode’s own surface map. */
export const SETTINGS_CATEGORIES: SettingsNavItem[] = [
	{
		id: "general",
		label: "General",
		description: "How LoopCode looks and feels day to day",
		icon: IconSettings,
	},
	{
		id: "providers",
		label: "Providers",
		description: "Connect AI providers and manage API keys",
		icon: IconPlug,
	},
	{
		id: "skills",
		label: "Skills",
		description: "Agent skills discovered in this project",
		icon: IconCode,
	},
	{
		id: "mcp",
		label: "MCP",
		description: "Register, trust, and enable MCP servers",
		icon: IconPlugConnected,
	},
	{
		id: "data",
		label: "Data",
		description: "Backup and update checks",
		icon: IconDatabase,
	},
	{
		id: "about",
		label: "About",
		description: "Version, license, and product details",
		icon: IconInfoCircle,
	},
];

export function settingsCategoryMeta(id: SettingsCategory): SettingsNavItem {
	return SETTINGS_CATEGORIES.find((c) => c.id === id) ?? SETTINGS_CATEGORIES[0];
}
