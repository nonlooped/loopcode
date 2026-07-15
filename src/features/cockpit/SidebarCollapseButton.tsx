import {
	IconLayoutSidebarLeftCollapse,
	IconLayoutSidebarRightCollapse,
} from "@tabler/icons-react";
import { Tooltip } from "@base-ui/react/tooltip";
import { Button, cn } from "../../design";

/** A local collapse affordance, placed at the edge of the panel it controls. */
export function SidebarCollapseButton({
	side,
	onCollapse,
}: {
	side: "left" | "right";
	onCollapse: () => void;
}) {
	const Icon =
		side === "left"
			? IconLayoutSidebarLeftCollapse
			: IconLayoutSidebarRightCollapse;
	const label = side === "left" ? "Hide left sidebar" : "Hide right sidebar";
	const shortcut = side === "left" ? "Ctrl/⌘ B" : "Ctrl/⌘ Alt B";

	return (
		<Tooltip.Root>
			<Tooltip.Trigger
				render={
					<Button
						variant="ghost"
						aria-label={`${label} (${shortcut})`}
						onClick={onCollapse}
						className={cn(
							"inline-flex size-7 shrink-0 items-center justify-center rounded-md p-0! text-ink-3 outline-none",
							"transition-[background-color,color,transform] duration-[var(--duration-fast)] ease-[var(--ease-out)]",
							"active:scale-[0.97] fine-hover:bg-surface fine-hover:text-ink",
							"focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent",
						)}
					>
						<Icon size={16} stroke={1.75} aria-hidden />
					</Button>
				}
			/>
			<Tooltip.Portal>
				<Tooltip.Positioner sideOffset={6}>
					<Tooltip.Popup className="surface-popover rounded-card border border-line bg-surface px-2.5 py-1.5 text-[12px] text-ink-2 shadow-soft-2 outline-none">
						{label} ({shortcut})
					</Tooltip.Popup>
				</Tooltip.Positioner>
			</Tooltip.Portal>
		</Tooltip.Root>
	);
}
