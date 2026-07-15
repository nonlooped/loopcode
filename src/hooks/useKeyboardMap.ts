import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import * as api from "../ipc/commands";
import { useSession } from "../store/session";
import { useUi } from "../store/ui";
import type { Mode } from "../ipc/types";

const MODE_CYCLE: Mode[] = ["ask", "plan", "build", "debug"];

/**
 * Global keyboard map mirroring the shipped Slice 6/10 registry. Reads live
 * store state via getState() so the listener binds once. Base UI dialogs close
 * on Escape themselves; Escape only stops a run when no overlay is open.
 */
export function useKeyboardMap() {
	const qc = useQueryClient();

	useEffect(() => {
		async function newChat() {
			const s = useSession.getState();
			if (!s.activeProjectId) return;
			try {
				const chat = await api.createChat(s.activeProjectId);
				s.setActiveChat(chat.id);
				qc.invalidateQueries({ queryKey: ["chats", s.activeProjectId] });
			} catch {
				/* preview or Core error — ignore */
			}
		}

		async function stopRun() {
			const s = useSession.getState();
			if (!s.activeRunId) return;
			try {
				await api.cancelRun(s.activeRunId);
				qc.invalidateQueries({ queryKey: ["timeline", s.activeChatId] });
			} catch {
				/* ignore */
			}
		}

		function onKey(e: KeyboardEvent) {
			const ui = useUi.getState();
			const meta = e.ctrlKey || e.metaKey;
			const k = e.key.toLowerCase();

			if (meta && !e.altKey && k === "n")
				return e.preventDefault(), void newChat();
			if (meta && k === "l") {
				e.preventDefault();
				document.getElementById("composer-input")?.focus();
				return;
			}
			if (meta && e.altKey && k === "b")
				return e.preventDefault(), ui.toggleRight();
			if (meta && k === "b") return e.preventDefault(), ui.toggleLeft();
			if (meta && k === "1") return e.preventDefault(), ui.setRightTab("files");
			if (meta && k === "2") return e.preventDefault(), ui.setRightTab("diffs");
			if (meta && e.shiftKey && k === "m") {
				e.preventDefault();
				const s = useSession.getState();
				s.setMode(
					MODE_CYCLE[(MODE_CYCLE.indexOf(s.mode) + 1) % MODE_CYCLE.length],
				);
				return;
			}
			if (k === "escape" && ui.settingsOpen) {
				e.preventDefault();
				ui.setSettingsOpen(false);
				return;
			}
			if (k === "escape" && !ui.settingsOpen) {
				void stopRun();
			}
			// F6 cycles major regions (composer ↔ timeline ↔ right pane / settings).
			if (k === "f6") {
				e.preventDefault();
				const regionIds = ui.settingsOpen
					? ["region-settings"]
					: ["composer-input", "region-timeline", "region-surfaces"];
				const regions = regionIds
					.map((id) => document.getElementById(id))
					.filter((el): el is HTMLElement => Boolean(el));
				if (regions.length === 0) return;
				const active = document.activeElement;
				const idx = regions.findIndex(
					(el) => el === active || el.contains(active),
				);
				regions[(idx + 1) % regions.length]?.focus();
			}
		}

		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	}, [qc]);
}
