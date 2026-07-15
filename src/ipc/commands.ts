/**
 * Typed wrappers for every Core IPC command the UI depends on. Grouped by
 * domain. Features consume these via TanStack Query hooks — never invoke()
 * directly. Argument names match the Rust command signatures exactly.
 */

import { invoke } from "./client";
import type {
	ApprovalScope,
	CatalogProvider,
	Chat,
	ConnectResult,
	DiffChange,
	DiffScope,
	HeroCard,
	McpServer,
	Mode,
	Project,
	ReadyProvider,
	ReliabilitySettings,
	ReasoningConfig,
	RunDto,
	SkillMeta,
	TimelineProjection,
	TreeEntry,
} from "./types";

/* ---------- health ---------- */

export const health = () => invoke<{ name: string; version: string }>("health");

/* ---------- onboarding ---------- */

export const listHeroes = () => invoke<HeroCard[]>("onboarding_list_heroes");
export const listCatalog = () =>
	invoke<CatalogProvider[]>("onboarding_list_catalog");
export const getReadyProvider = () =>
	invoke<ReadyProvider | null>("onboarding_get_ready_provider");

/** Persist selected provider + model for restarts and send resolution. */
export const setProviderSelection = (providerId: string, model: string) =>
	invoke<ReadyProvider>("cockpit_set_provider_selection", {
		providerId,
		model,
	});

export const connectProvider = (providerId: string, apiKey: string) =>
	invoke<ConnectResult>("onboarding_connect_provider", {
		providerId,
		apiKey,
	});

export const createCustomProfile = (args: {
	displayName: string;
	protocol: string;
	baseUrl: string;
	model: string | null;
}) =>
	invoke<{
		value: {
			id: string;
			displayName: string;
			protocol: string;
			baseUrl: string;
			defaultModel: string | null;
		};
		message: string;
	}>("onboarding_create_custom_profile", args);

/* ---------- projects & chats ---------- */

export const listProjects = () => invoke<Project[]>("list_projects");
export const openProject = (path: string) =>
	invoke<Project>("open_project", { path });

export const listChats = (projectId: string, includeArchived = false) =>
	invoke<Chat[]>("list_chats", { projectId, includeArchived });

export const createChat = (projectId: string, title?: string) =>
	invoke<Chat>("create_chat", { projectId, title });

export const updateChat = (args: {
	chatId: string;
	title?: string;
	archived?: boolean;
}) => invoke<Chat>("update_chat", args);

export const deleteChat = (chatId: string) =>
	invoke<void>("delete_chat", { chatId });

export const deleteProjectRecord = (projectId: string) =>
	invoke<void>("delete_project_record", { projectId });

/* ---------- runtime & timeline ---------- */

export const cockpitTimeline = (chatId: string | null) =>
	invoke<TimelineProjection>("cockpit_timeline", { chatId });

export const cockpitSend = (args: {
	chatId: string;
	text: string;
	mode: Mode;
	model: string | null;
	reasoning?: ReasoningConfig | null;
}) => invoke<RunDto>("cockpit_send", args);

export const cancelRun = (runId: string) =>
	invoke<void>("runtime_cancel_run", { runId });

export const grantApproval = (args: {
	chatId: string;
	runId: string | null;
	actionClass: string;
	scope: ApprovalScope;
}) => invoke<void>("cockpit_grant_approval", args);

/* ---------- surfaces: files, diffs ---------- */

export const listTree = (projectId: string, showExcluded: boolean) =>
	invoke<TreeEntry[]>("surfaces_list_tree", { projectId, showExcluded });

/** Open a workspace file in the OS default application. */
export const openExternal = (projectId: string, path: string) =>
	invoke<void>("surfaces_open_external", { projectId, path });

/** Open an http(s) URL in the OS default browser. */
export const openExternalUrl = (url: string) =>
	invoke<void>("open_external_url", { url });

/* ---------- clipboard ---------- */

/** Clipboard access is Core-owned so the WebView has no direct OS surface. */
export const clipboardWriteText = (text: string) =>
	invoke<void>("clipboard_write_text", { text });

export const clipboardReadText = () =>
	invoke<string>("clipboard_read_text");

export const projectDiffs = (args: {
	chatId: string;
	scope: DiffScope;
	runId: string | null;
	dirtyBuffers: { path: string; content: string; dirty: boolean }[];
	projectId: string | null;
	checkpointId: string | null;
}) => invoke<{ changes: DiffChange[] }>("surfaces_project_diffs", args);

/* ---------- extensibility ---------- */

export const listSkills = (projectId: string) =>
	invoke<{ skills: SkillMeta[] }>("ext_list_skills", { projectId });

export const loadSkill = (projectId: string, path: string) =>
	invoke<{
		status: string;
		path: string;
		name?: string | null;
		description?: string | null;
		body: string;
	}>("ext_load_skill", { projectId, path });

export const listMcp = () => invoke<McpServer[]>("ext_mcp_list", {});

export const registerMcp = (config: {
	name: string;
	transportType: string;
	command?: string | null;
	args?: string[] | null;
	envKeys?: string[] | null;
	url?: string | null;
	headerKeys?: string[] | null;
	projectId?: string | null;
	id?: string | null;
}) => invoke<McpServer>("ext_mcp_register", { config });

export const setMcpEnabled = (serverId: string, enabled: boolean) =>
	invoke<McpServer>("ext_mcp_set_enabled", { serverId, enabled });

export const removeMcp = (serverId: string) =>
	invoke<void>("ext_mcp_remove", { serverId });

export const grantMcpTrust = (
	serverId: string,
	projectId: string | null = null,
) => invoke<void>("ext_mcp_grant_trust", { serverId, projectId });

/* ---------- reliability & a11y ---------- */

export const reliabilitySettings = () =>
	invoke<ReliabilitySettings>("reliability_settings");

export const updateCheck = (enabled: boolean) =>
	invoke<string>("reliability_update_check", { enabled });

export const backupDb = () => invoke<string>("reliability_backup_db");

export const applyTheme = (theme: string) =>
	invoke<void>("a11y_apply_theme", { theme });

export const menuActions = () =>
	invoke<
		{ menu: string; label: string; command: string; chord?: string | null }[]
	>("a11y_menu_actions");
