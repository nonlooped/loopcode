import { motion, useReducedMotion } from "motion/react";
import { useDiffs } from "../../ipc/hooks";
import { useSession } from "../../store/session";
import { useUi } from "../../store/ui";
import { cn, pressable } from "../../design";

/**
 * Session change summary — a small floating pill above the composer
 * ("3 files changed +120 −14"). Click opens the Diffs panel.
 */
export function ChangesPill() {
	const reduce = useReducedMotion();
	const { activeChatId, activeRunId, activeProjectId } = useSession();
	const setRightTab = useUi((s) => s.setRightTab);
	const rightCollapsed = useUi((s) => s.rightCollapsed);
	const toggleRight = useUi((s) => s.toggleRight);

	const { data } = useDiffs({
		chatId: activeChatId,
		scope: "since_chat_start",
		runId: activeRunId,
		projectId: activeProjectId,
	});
	const changes = data?.changes ?? [];

	const { added, removed, hasCounts } = (() => {
		let added = 0;
		let removed = 0;
		let hasCounts = false;
		for (const c of changes) {
			if (typeof c.linesAdded === "number") {
				added += c.linesAdded;
				hasCounts = true;
			}
			if (typeof c.linesRemoved === "number") {
				removed += c.linesRemoved;
				hasCounts = true;
			}
		}
		return { added, removed, hasCounts };
	})();

	if (changes.length === 0) return null;

	const fileLabel =
		changes.length === 1 ? "1 file changed" : `${changes.length} files changed`;

	function openDiffs() {
		setRightTab("diffs");
		if (rightCollapsed) toggleRight();
	}

	return (
		<div className="pointer-events-none absolute inset-x-0 -top-10 flex justify-center">
			<motion.button
				type="button"
				onClick={openDiffs}
				aria-label={`${fileLabel}${hasCounts ? `, ${added} lines added, ${removed} lines removed` : ""}. Open diffs panel`}
				initial={reduce ? false : { opacity: 0, transform: "translateY(4px)" }}
				animate={{ opacity: 1, transform: "translateY(0px)" }}
				transition={{ duration: 0.18, ease: [0.23, 1, 0.32, 1] }}
				className={cn(
					pressable,
					"pointer-events-auto inline-flex items-center gap-1.5 rounded-full border border-line bg-surface",
					"px-3 py-1 text-[12px] font-medium text-ink-2 shadow-soft-1",
					"fine-hover:border-ink-3 fine-hover:text-ink",
					"focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
				)}
			>
				<span>{fileLabel}</span>
				{hasCounts && (
					<>
						<span className="font-semibold text-accent">+{added}</span>
						<span className="font-semibold text-clay">−{removed}</span>
					</>
				)}
			</motion.button>
		</div>
	);
}
