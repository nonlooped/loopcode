import { IconChevronRight } from "@tabler/icons-react";
import { Chip, Wordmark } from "../../design";
import { useChats, useProjects } from "../../ipc/hooks";
import { projectDirName } from "../../lib/path";
import { useSession } from "../../store/session";
import { useUi } from "../../store/ui";
import { WindowControls } from "./WindowControls";
import type { TimelineProjection } from "../../ipc/types";

export function TopBar({
	projection,
}: {
	projection: TimelineProjection | null;
}) {
	const { activeProjectId, activeChatId } = useSession();
	const settingsOpen = useUi((s) => s.settingsOpen);

	const { data: chats = [] } = useChats(activeProjectId);
	const { data: projects = [] } = useProjects();
	const chat = chats.find((c) => c.id === activeChatId);
	const project = projects.find((p) => p.id === activeProjectId);
	const projectName = project
		? projectDirName(project.displayPath || project.rootPath)
		: null;

	return (
		<header className="material-chrome flex h-11 shrink-0 select-none items-stretch border-b text-[13px]">
			{/* Drag region covers empty chrome; interactive controls sit outside it. */}
			<div
				data-tauri-drag-region
				className="flex min-w-0 flex-1 items-center gap-3 py-1.5 pl-4"
			>
				<Wordmark className="pointer-events-none shrink-0 text-[15px]" />
				{settingsOpen ? (
					<span className="pointer-events-none min-w-0 truncate text-[12.5px] text-ink-3">
						Settings
					</span>
				) : projectName || chat ? (
					<nav
						aria-label="Conversation path"
						className="pointer-events-none flex min-w-0 max-w-[36vw] items-center gap-1.5 border-l border-line pl-3 text-[12.5px]"
					>
						{projectName && (
							<span
								title={projectName}
								className="max-w-40 truncate text-ink-3"
							>
								{projectName}
							</span>
						)}
						{projectName && chat && (
							<IconChevronRight
								size={14}
								stroke={1.75}
								className="shrink-0 text-ink-3"
								aria-hidden
							/>
						)}
						{chat && (
							<span
								title={chat.title}
								className="min-w-0 truncate font-medium text-ink-2"
							>
								{chat.title}
							</span>
						)}
					</nav>
				) : null}

				<div data-tauri-drag-region className="min-h-full min-w-4 flex-1" />

				{!settingsOpen && projection && (
					<div
						data-tauri-drag-region
						className="pointer-events-none hidden items-center gap-2 md:flex"
						aria-label="Run metrics"
					>
						<Chip quiet>{projection.costDisplay}</Chip>
						<Chip quiet>{projection.tokensDisplay}</Chip>
					</div>
				)}
			</div>

			<WindowControls />
		</header>
	);
}
