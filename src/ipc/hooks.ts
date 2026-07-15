/**
 * TanStack Query hooks over the Core IPC commands. In browser preview (no Tauri
 * host) a few read hooks fall back to static data so the UI is still reviewable.
 */

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as api from "./commands";
import { isTauri } from "./client";
import type {
	CatalogModel,
	CatalogProvider,
	DiffScope,
	HeroCard,
	Mode,
} from "./types";

/** Honest browser-preview heroes: none pretends to be authenticated. */
const STATIC_HEROES: HeroCard[] = [
	{
		id: "openai",
		name: "OpenAI",
		firstParty: true,
		hasSavedKey: false,
		defaultModel: "gpt-5.4",
	},
	{
		id: "anthropic",
		name: "Anthropic",
		firstParty: true,
		hasSavedKey: false,
		defaultModel: "claude-sonnet-4-5",
	},
	{
		id: "openrouter",
		name: "OpenRouter",
		firstParty: true,
		hasSavedKey: false,
		defaultModel: "openai/gpt-5.4",
	},
	{
		id: "opencode",
		name: "OpenCode Zen",
		firstParty: true,
		hasSavedKey: false,
		defaultModel: "gpt-5.4",
	},
];

type ModelsDevProvider = {
	id?: string;
	name?: string;
	doc?: string;
	api?: string;
	npm?: string;
	env?: string[];
	models?: Record<
		string,
		{
			id?: string;
			name?: string;
			family?: string;
			release_date?: string;
			tool_call?: boolean;
			modalities?: { input?: string[]; output?: string[] };
		}
	>;
};

const PREFERRED_DEFAULTS: Record<string, string[]> = {
	openai: ["gpt-5.4", "gpt-5.2", "gpt-5.1", "gpt-5", "gpt-4.1", "gpt-4o"],
	anthropic: [
		"claude-sonnet-4-5",
		"claude-sonnet-4-6",
		"claude-opus-4-6",
		"claude-sonnet-4",
	],
	openrouter: ["openai/gpt-5.4", "openai/gpt-5.2", "openai/gpt-5"],
	opencode: ["gpt-5.4", "gpt-5.2", "claude-sonnet-4-5", "gpt-5"],
};

function isChatModel(
	m: NonNullable<ModelsDevProvider["models"]>[string],
): boolean {
	const out = m.modalities?.output ?? [];
	if (!out.includes("text")) return false;
	const id = (m.id || "").toLowerCase();
	const family = (m.family || "").toLowerCase();
	if (
		/embedding|whisper|tts|transcribe|dall-e|image|realtime|moderation|omni-moderation/.test(
			id,
		)
	) {
		return false;
	}
	if (/embedding|whisper|tts|image/.test(family)) return false;
	return true;
}

function pickDefault(
	providerId: string,
	models: CatalogModel[],
): string | null {
	for (const p of PREFERRED_DEFAULTS[providerId] || []) {
		if (models.some((m) => m.id === p)) return p;
	}
	return models[0]?.id ?? null;
}

/** Map models.dev api.json → CatalogProvider[] (browser preview / refresh path). */
export function mapModelsDevApi(
	api: Record<string, ModelsDevProvider>,
): CatalogProvider[] {
	const providers: CatalogProvider[] = [];
	for (const [id, p] of Object.entries(api)) {
		if (!p?.models) continue;
		const chat = Object.values(p.models).filter(isChatModel);
		if (chat.length === 0) continue;
		chat.sort((a, b) => {
			const rd = (b.release_date || "").localeCompare(a.release_date || "");
			if (rd) return rd;
			return (a.name || a.id || "").localeCompare(b.name || b.id || "");
		});
		const models: CatalogModel[] = chat.map((m) => ({
			id: m.id || "",
			name: m.name || m.id || "",
			default: false,
		}));
		const defaultModel = pickDefault(id, models);
		if (defaultModel) {
			const hit = models.find((m) => m.id === defaultModel);
			if (hit) hit.default = true;
		}
		providers.push({
			id,
			name: p.name || id,
			docsUrl: p.doc ?? null,
			api: p.api ?? null,
			npm: p.npm
				? `DO_NOT_EXECUTE_${String(p.npm).replace(/^@/, "")}`
				: "DO_NOT_EXECUTE",
			env: Array.isArray(p.env) ? p.env : null,
			defaultModel,
			models,
		});
	}
	return providers;
}

async function loadCatalog(): Promise<CatalogProvider[]> {
	// Catalog refresh and network access are Core-owned. Browser preview intentionally
	// shows an empty advisory catalog instead of making an unrestricted web request.
	return isTauri() ? api.listCatalog() : [];
}

/* ---------- onboarding ---------- */

export function useHealth() {
	return useQuery({
		queryKey: ["health"],
		queryFn: api.health,
		enabled: isTauri(),
	});
}

export function useReadyProvider() {
	return useQuery({
		queryKey: ["readyProvider"],
		queryFn: async () => (isTauri() ? api.getReadyProvider() : null),
	});
}

/** Persist cockpit provider/model selection into Core (`provider.ready`). */
export function useSetProviderSelection() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (vars: { providerId: string; model: string }) =>
			api.setProviderSelection(vars.providerId, vars.model),
		onSuccess: () => {
			void qc.invalidateQueries({ queryKey: ["readyProvider"] });
			// Heroes carry per-provider last model for switch-back restore.
			void qc.invalidateQueries({ queryKey: ["heroes"] });
		},
	});
}

export function useHeroes() {
	return useQuery({
		queryKey: ["heroes"],
		// No initialData: a fake "OpenAI connected" seed was selecting the wrong provider.
		queryFn: async () => (isTauri() ? api.listHeroes() : STATIC_HEROES),
	});
}

/** Full models.dev catalog (providers + models). Advisory; never executes npm recipes. */
export function useCatalog() {
	return useQuery({
		queryKey: ["catalog"],
		queryFn: loadCatalog,
		staleTime: 5 * 60_000,
	});
}

export function useConnectProvider() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (vars: { providerId: string; apiKey: string }) =>
			api.connectProvider(vars.providerId, vars.apiKey),
		onSuccess: () => {
			// Refresh saved-key badges and the "ready provider" skip-onboarding path.
			void qc.invalidateQueries({ queryKey: ["heroes"] });
			void qc.invalidateQueries({ queryKey: ["readyProvider"] });
		},
	});
}

/* ---------- projects & chats ---------- */

export function useProjects() {
	return useQuery({
		queryKey: ["projects"],
		queryFn: async () => (isTauri() ? api.listProjects() : []),
	});
}

export function useOpenProject() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (path: string) => api.openProject(path),
		onSuccess: () => qc.invalidateQueries({ queryKey: ["projects"] }),
	});
}

export function useChats(projectId: string | null) {
	return useQuery({
		queryKey: ["chats", projectId],
		queryFn: async () => (projectId ? api.listChats(projectId) : []),
		enabled: Boolean(projectId),
	});
}

export function useCreateChat() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (vars: { projectId: string; title?: string }) =>
			api.createChat(vars.projectId, vars.title),
		onSuccess: (_data, vars) =>
			qc.invalidateQueries({ queryKey: ["chats", vars.projectId] }),
	});
}

export function useUpdateChat() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: api.updateChat,
		onSuccess: () =>
			qc.invalidateQueries({ queryKey: ["chats"] }),
	});
}

export function useDeleteChat() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: api.deleteChat,
		onSuccess: () => qc.invalidateQueries({ queryKey: ["chats"] }),
	});
}

export function useDeleteProject() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: api.deleteProjectRecord,
		onSuccess: () => qc.invalidateQueries({ queryKey: ["projects"] }),
	});
}

/* ---------- timeline & runtime ---------- */

export function useTimeline(chatId: string | null) {
	return useQuery({
		queryKey: ["timeline", chatId],
		queryFn: async () =>
			isTauri() && chatId ? api.cockpitTimeline(chatId) : null,
		enabled: Boolean(chatId),
		refetchInterval: (query) => {
			// Core state is populated/empty_*; active runs are indicated by activeRunId.
			return query.state.data?.activeRunId ? 1200 : false;
		},
	});
}

export function useSend() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (vars: {
			chatId: string;
			text: string;
			mode: Mode;
			model: string | null;
			reasoning?: import("./types").ReasoningConfig | null;
		}) => api.cockpitSend(vars),
		onSuccess: (_data, vars) => {
			qc.invalidateQueries({ queryKey: ["timeline", vars.chatId] });
			// Core may auto-title the chat from this first message — refresh the sidebar.
			qc.invalidateQueries({ queryKey: ["chats"] });
		},
	});
}

export function useCancelRun() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (vars: { runId: string; chatId: string | null }) =>
			api.cancelRun(vars.runId),
		onSuccess: (_data, vars) =>
			qc.invalidateQueries({ queryKey: ["timeline", vars.chatId] }),
	});
}

export function useGrantApproval() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: api.grantApproval,
		onSuccess: (_data, vars) =>
			qc.invalidateQueries({ queryKey: ["timeline", vars.chatId] }),
	});
}

/* ---------- surfaces ---------- */

export function useTree(projectId: string | null, showExcluded: boolean) {
	return useQuery({
		queryKey: ["tree", projectId, showExcluded],
		queryFn: async () =>
			projectId ? api.listTree(projectId, showExcluded) : [],
		enabled: Boolean(projectId),
		// No FS watcher in Core yet — poll so external/agent edits don't leave the tree stale.
		refetchInterval: 2_000,
	});
}

export function useDiffs(args: {
	chatId: string | null;
	scope: DiffScope;
	runId: string | null;
	projectId: string | null;
}) {
	return useQuery({
		queryKey: ["diffs", args.chatId, args.scope, args.runId],
		queryFn: async () =>
			args.chatId
				? api.projectDiffs({
						chatId: args.chatId,
						scope: args.scope,
						runId: args.runId,
						dirtyBuffers: [],
						projectId: args.projectId,
						checkpointId: null,
					})
				: { changes: [] },
		enabled: Boolean(args.chatId),
		// Diffs come from events + disk — poll so in-flight runs update live.
		refetchInterval: 2_000,
	});
}

/* ---------- settings surfaces ---------- */

export function useSkills(projectId: string | null) {
	return useQuery({
		queryKey: ["skills", projectId],
		queryFn: async () =>
			projectId ? api.listSkills(projectId) : { skills: [] },
		enabled: Boolean(projectId),
	});
}

export function useMcp() {
	return useQuery({
		queryKey: ["mcp"],
		queryFn: api.listMcp,
		enabled: isTauri(),
	});
}

export function useRegisterMcp() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: api.registerMcp,
		onSuccess: () => void qc.invalidateQueries({ queryKey: ["mcp"] }),
	});
}

export function useSetMcpEnabled() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: ({
			serverId,
			enabled,
		}: {
			serverId: string;
			enabled: boolean;
		}) => api.setMcpEnabled(serverId, enabled),
		onSuccess: () => void qc.invalidateQueries({ queryKey: ["mcp"] }),
	});
}

export function useRemoveMcp() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (serverId: string) => api.removeMcp(serverId),
		onSuccess: () => void qc.invalidateQueries({ queryKey: ["mcp"] }),
	});
}

export function useGrantMcpTrust() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: ({
			serverId,
			projectId,
		}: {
			serverId: string;
			projectId: string | null;
		}) => api.grantMcpTrust(serverId, projectId),
		onSuccess: () => void qc.invalidateQueries({ queryKey: ["mcp"] }),
	});
}

export function useReliabilitySettings() {
	return useQuery({
		queryKey: ["reliabilitySettings"],
		queryFn: api.reliabilitySettings,
		enabled: isTauri(),
	});
}
