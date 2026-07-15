import { create } from "zustand";
import type { Mode } from "../ipc/types";

export type View = "onboarding" | "cockpit";

interface SessionState {
	view: View;
	activeProjectId: string | null;
	activeChatId: string | null;
	activeRunId: string | null;
	/**
	 * Most recent run id for the active chat, kept after the run completes
	 * (activeRunId is nulled then). Lets "This turn" diffs show the last turn.
	 */
	lastRunId: string | null;
	mode: Mode;
	/** Selected first-party provider id (models.dev catalog). */
	providerId: string | null;
	/** Selected model id for the active provider. */
	model: string | null;
	/**
	 * Composer reasoning selection: an effort id ("low"…"max"), "on"
	 * (toggle-only models), "off", or null (model default / not applicable).
	 */
	reasoning: string | null;

	enterCockpit: () => void;
	setActiveProject: (id: string | null) => void;
	setActiveChat: (id: string | null) => void;
	setActiveRun: (id: string | null) => void;
	setMode: (mode: Mode) => void;
	setProviderId: (providerId: string | null) => void;
	setModel: (model: string | null) => void;
	setReasoning: (reasoning: string | null) => void;
	/** Switch provider and optionally pair a model in one update. */
	setProviderAndModel: (
		providerId: string | null,
		model: string | null,
	) => void;
}

/** Global session state: which view is showing and the active project/chat/run. */
export const useSession = create<SessionState>((set) => ({
	view: "onboarding",
	activeProjectId: null,
	activeChatId: null,
	activeRunId: null,
	lastRunId: null,
	mode: "build",
	providerId: null,
	model: null,
	reasoning: null,

	enterCockpit: () => set({ view: "cockpit" }),
	setActiveProject: (activeProjectId) =>
		set({ activeProjectId, activeChatId: null, activeRunId: null, lastRunId: null }),
	setActiveChat: (activeChatId) =>
		set({ activeChatId, activeRunId: null, lastRunId: null }),
	setActiveRun: (activeRunId) =>
		set((s) => ({ activeRunId, lastRunId: activeRunId ?? s.lastRunId })),
	setMode: (mode) => set({ mode }),
	setProviderId: (providerId) => set({ providerId }),
	setModel: (model) => set({ model }),
	setReasoning: (reasoning) => set({ reasoning }),
	setProviderAndModel: (providerId, model) => set({ providerId, model }),
}));
