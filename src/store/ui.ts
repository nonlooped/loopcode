import { create } from "zustand";

export type Theme = "light" | "dark";
export type RightTab = "files" | "diffs";
export type SettingsCategory =
	| "general"
	| "providers"
	| "skills"
	| "mcp"
	| "data"
	| "about";

/** Shared default so left rail and right pane start equal. */
export const SIDEBAR_DEFAULT_WIDTH = 280;
export const SIDEBAR_MIN_WIDTH = 200;
export const SIDEBAR_MAX_WIDTH = 480;

function clampSidebarWidth(width: number): number {
	return Math.min(
		SIDEBAR_MAX_WIDTH,
		Math.max(SIDEBAR_MIN_WIDTH, Math.round(width)),
	);
}

interface UiState {
	theme: Theme;
	settingsOpen: boolean;
	settingsCategory: SettingsCategory;
	rightTab: RightTab;
	leftCollapsed: boolean;
	rightCollapsed: boolean;
	leftWidth: number;
	rightWidth: number;

	setTheme: (theme: Theme) => void;
	toggleTheme: () => void;
	setSettingsOpen: (open: boolean) => void;
	setSettingsCategory: (category: SettingsCategory) => void;
	setRightTab: (tab: RightTab) => void;
	toggleLeft: () => void;
	toggleRight: () => void;
	setLeftWidth: (width: number) => void;
	setRightWidth: (width: number) => void;
}

/** Global UI state: theme, overlays, and panel layout. */
export const useUi = create<UiState>((set) => ({
	theme: "light",
	settingsOpen: false,
	settingsCategory: "general",
	rightTab: "files",
	leftCollapsed: false,
	rightCollapsed: false,
	leftWidth: SIDEBAR_DEFAULT_WIDTH,
	rightWidth: SIDEBAR_DEFAULT_WIDTH,

	setTheme: (theme) => set({ theme }),
	toggleTheme: () =>
		set((s) => ({ theme: s.theme === "dark" ? "light" : "dark" })),
	setSettingsOpen: (settingsOpen) =>
		set({
			settingsOpen,
			// Categories need the left rail; expand it when entering settings.
			...(settingsOpen ? { leftCollapsed: false as const } : {}),
		}),
	setSettingsCategory: (settingsCategory) => set({ settingsCategory }),
	setRightTab: (rightTab) => set({ rightTab }),
	// Don't collapse the category rail while settings is open.
	toggleLeft: () =>
		set((s) => (s.settingsOpen ? s : { leftCollapsed: !s.leftCollapsed })),
	// Right pane is hidden in settings mode; ignore toggles until back in chat.
	toggleRight: () =>
		set((s) => (s.settingsOpen ? s : { rightCollapsed: !s.rightCollapsed })),
	setLeftWidth: (width) => set({ leftWidth: clampSidebarWidth(width) }),
	setRightWidth: (width) => set({ rightWidth: clampSidebarWidth(width) }),
}));
