import {
	IconLayoutSidebarLeftExpand,
	IconLayoutSidebarRightExpand,
} from "@tabler/icons-react";
import { Tooltip } from "@base-ui/react/tooltip";
import { Button, cn } from "../../design";

type Side = "left" | "right";

/**
 * Slim edge strip shown when a sidebar is collapsed — the only way back in,
 * since the collapse control now lives on the sidebar itself and disappears
 * with it. Mirrors the VS Code activity-bar pattern.
 */
export function SidebarCollapsedRail({
	side,
	onExpand,
}: {
	side: Side;
	onExpand: () => void;
}) {
	const Icon =
		side === "left" ? IconLayoutSidebarLeftExpand : IconLayoutSidebarRightExpand;
	const label = side === "left" ? "Show left sidebar" : "Show right sidebar";
	const shortcut = side === "left" ? "Ctrl/⌘ B" : "Ctrl/⌘ Alt B";

	return (
		<div
			className={cn(
				"flex h-full w-9 shrink-0 items-start justify-center bg-surface-2 pt-2",
				side === "left" ? "border-r border-line" : "border-l border-line",
			)}
		>
			<Tooltip.Root>
				<Tooltip.Trigger
					render={
						<Button
							variant="ghost"
							aria-label={`${label} (${shortcut})`}
							onClick={onExpand}
							className={cn(
								"inline-flex size-7 items-center justify-center rounded-md p-0! text-ink-3 outline-none",
								"transition-[background-color,color] duration-[var(--duration-fast)] ease-[var(--ease-out)]",
								"fine-hover:bg-surface fine-hover:text-ink",
								"focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent",
							)}
						>
							<Icon size={16} stroke={1.75} aria-hidden />
						</Button>
					}
				/>
				<Tooltip.Portal>
					<Tooltip.Positioner sideOffset={6} align="start">
						<Tooltip.Popup className="surface-popover rounded-card border border-line bg-surface px-2.5 py-1.5 text-[12px] text-ink-2 shadow-soft-2 outline-none">
							{label} ({shortcut})
						</Tooltip.Popup>
					</Tooltip.Positioner>
				</Tooltip.Portal>
			</Tooltip.Root>
		</div>
	);
}
