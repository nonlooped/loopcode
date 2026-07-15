import { useEffect, useState, type KeyboardEvent, type MouseEvent } from "react";
import {
	IconChevronRight,
	IconFolder,
	IconFolderOpen,
	IconFolderPlus,
	IconMessage,
	IconPlus,
	IconSettings,
	IconSortAZ,
	IconSortZA,
} from "@tabler/icons-react";
import { Collapsible } from "@base-ui/react/collapsible";
import { Input } from "@base-ui/react/input";
import { useQueries } from "@tanstack/react-query";
import {
	Button,
	ContextMenu,
	cn,
	inputChrome,
	listRow,
	listRowActive,
	listRowIdle,
} from "../../design";
import * as api from "../../ipc/commands";
import { isTauri } from "../../ipc/client";
import {
	useCreateChat,
	useDeleteChat,
	useDeleteProject,
	useOpenProject,
	useProjects,
	useUpdateChat,
} from "../../ipc/hooks";
import { pickFolder } from "../../ipc/dialog";
import { copyText } from "../../lib/clipboard";
import { projectDirName } from "../../lib/path";
import { useSession } from "../../store/session";
import { useUi } from "../../store/ui";
import type { Chat, Project } from "../../ipc/types";
import { SidebarCollapseButton } from "./SidebarCollapseButton";

/** Compact icon control for the rail toolbar and tree rows. */
const iconBtn =
	"inline-flex size-7 shrink-0 items-center justify-center rounded-md p-0! text-ink-3 outline-none " +
	"transition-[background-color,color,transform] duration-[var(--duration-fast)] ease-[var(--ease-out)] " +
	"active:scale-[0.97] fine-hover:bg-surface fine-hover:text-ink " +
	"focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent";

export function LeftRail() {
	const { activeProjectId, activeChatId } = useSession();
	const setActiveProject = useSession((s) => s.setActiveProject);
	const setActiveChat = useSession((s) => s.setActiveChat);
	const setSettingsOpen = useUi((s) => s.setSettingsOpen);
	const toggleLeft = useUi((s) => s.toggleLeft);

	const { data: projects = [] } = useProjects();
	const openProject = useOpenProject();
	const createChat = useCreateChat();
	const deleteChat = useDeleteChat();
	const deleteProject = useDeleteProject();
	const updateChat = useUpdateChat();

	const [query, setQuery] = useState("");
	const [sortAsc, setSortAsc] = useState(true);
	const [expanded, setExpanded] = useState<Record<string, boolean>>({});

	const chatQueries = useQueries({
		queries: projects.map((p) => ({
			queryKey: ["chats", p.id] as const,
			queryFn: async () => (isTauri() ? api.listChats(p.id) : ([] as Chat[])),
			enabled: Boolean(p.id),
		})),
	});

	const chatsByProject = (() => {
		const map = new Map<string, Chat[]>();
		projects.forEach((p, i) => {
			map.set(p.id, chatQueries[i]?.data ?? []);
		});
		return map;
	})();

	// Keep the active project open so its chats stay visible.
	useEffect(() => {
		if (!activeProjectId) return;
		setExpanded((prev) =>
			prev[activeProjectId] ? prev : { ...prev, [activeProjectId]: true },
		);
	}, [activeProjectId]);

	const q = query.trim().toLowerCase();

	const sortedProjects = (() => {
		const named = projects.map((p) => ({
			project: p,
			name: projectDirName(p.displayPath || p.rootPath),
		}));
		named.sort((a, b) => {
			const cmp = a.name.localeCompare(b.name, undefined, {
				sensitivity: "base",
			});
			return sortAsc ? cmp : -cmp;
		});
		return named;
	})();

	const visibleTree = (() => {
		return sortedProjects
			.map(({ project, name }) => {
				const chats = chatsByProject.get(project.id) ?? [];
				if (!q) return { project, name, chats, forceOpen: false };

				const projectMatch = name.toLowerCase().includes(q);
				const matchingChats = chats.filter((c) =>
					c.title.toLowerCase().includes(q),
				);
				if (!projectMatch && matchingChats.length === 0) return null;

				// When searching, prefer showing matching sessions; if only the project
				// name matches, still show all of its chats so the folder isn't empty.
				return {
					project,
					name,
					chats:
						projectMatch && matchingChats.length === 0 ? chats : matchingChats,
					forceOpen: true,
				};
			})
			.filter(Boolean) as {
			project: Project;
			name: string;
			chats: Chat[];
			forceOpen: boolean;
		}[];
	})();

	async function handleOpenFolder() {
		const path = await pickFolder();
		if (!path) return;
		const project = await openProject.mutateAsync(path);
		setActiveProject(project.id);
		setExpanded((prev) => ({ ...prev, [project.id]: true }));
	}

	async function handleNewChat(projectId: string, e?: MouseEvent) {
		e?.stopPropagation();
		e?.preventDefault();
		const chat = await createChat.mutateAsync({ projectId });
		setActiveProject(projectId);
		setActiveChat(chat.id);
		setExpanded((prev) => ({ ...prev, [projectId]: true }));
	}

	function toggleProject(projectId: string, open: boolean) {
		setExpanded((prev) => ({ ...prev, [projectId]: open }));
	}

	function handleTreeKeyDown(event: KeyboardEvent<HTMLUListElement>) {
		const nodes = Array.from(event.currentTarget.querySelectorAll<HTMLElement>("[data-tree-node]"));
		const current = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>("[data-tree-node]") : null;
		if (!current || nodes.length === 0) return;
		const index = nodes.indexOf(current);
		const focus = (node: HTMLElement | undefined) => node?.focus();
		if (event.key === "ArrowDown") {
			event.preventDefault();
			focus(nodes[index + 1] ?? nodes[0]);
		} else if (event.key === "ArrowUp") {
			event.preventDefault();
			focus(nodes[index - 1] ?? nodes[nodes.length - 1]);
		} else if (event.key === "Home") {
			event.preventDefault();
			focus(nodes[0]);
		} else if (event.key === "End") {
			event.preventDefault();
			focus(nodes[nodes.length - 1]);
		} else if (event.key === "ArrowRight" && current.dataset.treeDir === "true") {
			event.preventDefault();
			if (current.dataset.treeOpen !== "true") current.click();
			else focus(nodes[index + 1]);
		} else if (event.key === "ArrowLeft" && current.dataset.treeDir === "true") {
			event.preventDefault();
			if (current.dataset.treeOpen === "true") current.click();
		}
	}

	function renameChat(chat: Chat) {
		const title = window.prompt("Rename chat", chat.title)?.trim();
		if (!title || title === chat.title) return;
		void updateChat.mutateAsync({ chatId: chat.id, title });
	}

	function removeChat(chat: Chat) {
		if (!window.confirm(`Delete “${chat.title}”? This cannot be undone.`)) return;
		void deleteChat.mutateAsync(chat.id).then(() => {
			if (chat.id === activeChatId) setActiveChat(null);
		});
	}

	function removeProject(project: Project, name: string) {
		if (!window.confirm(`Remove “${name}” from LoopCode? Your files will stay on disk.`)) return;
		void deleteProject.mutateAsync(project.id).then(() => {
			if (project.id === activeProjectId) setActiveProject(null);
		});
	}

	return (
		<aside className="flex h-full min-h-0 flex-col border-r border-line bg-surface-2">
			{/* Toolbar: search sessions + sort projects + open folder */}
			<div className="flex shrink-0 items-center gap-1 border-b border-line px-2 py-2">
				<Input
					type="search"
					aria-label="Search sessions"
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					placeholder="Search sessions…"
					className={cn(inputChrome, "h-7 min-w-0 flex-1 px-2 py-1 text-[13px]")}
				/>
				<Button
					variant="ghost"
					className={iconBtn}
					onClick={() => setSortAsc((v) => !v)}
					title={sortAsc ? "Sort projects Z–A" : "Sort projects A–Z"}
					aria-label={sortAsc ? "Sort projects Z to A" : "Sort projects A to Z"}
					aria-pressed={!sortAsc}
				>
					{sortAsc ? (
						<IconSortAZ size={16} stroke={1.75} aria-hidden />
					) : (
						<IconSortZA size={16} stroke={1.75} aria-hidden />
					)}
				</Button>
				<Button
					variant="ghost"
					className={iconBtn}
					onClick={handleOpenFolder}
					title="Open folder as project"
					aria-label="Open folder as project"
				>
					<IconFolderPlus size={16} stroke={1.75} aria-hidden />
				</Button>
				<SidebarCollapseButton side="left" onCollapse={toggleLeft} />
			</div>

			{/* Explorer tree: project folders → chat files */}
			<div className="min-h-0 flex-1 overflow-y-auto px-1.5 py-1.5">
				{visibleTree.length === 0 ? (
					<p className="px-2 py-3 text-[13px] text-ink-3">
						{q
							? "No sessions match your search."
							: "No projects yet. Use the folder button above to open one."}
					</p>
				) : (
					<ul
						className="flex flex-col gap-px"
						role="tree"
						aria-label="Projects and sessions"
						onKeyDown={handleTreeKeyDown}
					>
						{visibleTree.map(({ project, name, chats, forceOpen }) => {
							const full = project.displayPath || project.rootPath;
							const isActiveProject = project.id === activeProjectId;
							const chatInProjectActive = chats.some(
								(c) => c.id === activeChatId,
							);
							// Expanded by default; only an explicit collapse (false) closes it.
							const isOpen = forceOpen || expanded[project.id] !== false;
							// Full active tint only when the project itself is selected (no chat focus).
							const projectRowActive = isActiveProject && !chatInProjectActive;

							return (
								<Collapsible.Root
									key={project.id}
									open={isOpen}
									onOpenChange={(open) => {
										if (forceOpen) return;
										toggleProject(project.id, open);
									}}
									render={<li role="treeitem" aria-expanded={isOpen} />}
								>
									<div className="flex items-center gap-0.5">
										<ContextMenu
											items={[
												{ label: "New chat", onSelect: () => void handleNewChat(project.id) },
												"separator",
												{ label: "Copy path", onSelect: () => void copyText(full) },
												{ label: "Open in file manager", disabled: true },
												{
													label: "Collapse all",
													onSelect: () => setExpanded(Object.fromEntries(projects.map((p) => [p.id, false]))),
												},
												"separator",
												{ label: "Remove project…", tone: "danger", onSelect: () => removeProject(project, name) },
											]}
										>
											<Collapsible.Trigger
											data-tree-node
											data-tree-dir="true"
											data-tree-open={isOpen ? "true" : "false"}
											title={full}
											className={cn(
												listRow,
												"flex min-w-0 flex-1 items-center gap-1 py-1 pr-1 pl-1.5 text-[13px]",
												projectRowActive
													? listRowActive
													: isActiveProject
														? "font-semibold text-ink fine-hover:bg-surface"
														: listRowIdle,
											)}
											onClick={() => setActiveProject(project.id)}
										>
											<IconChevronRight
												size={14}
												stroke={1.75}
												aria-hidden
												className={cn(
													"shrink-0 text-ink-3 transition-transform duration-[var(--duration-fast)] ease-[var(--ease-out)]",
													isOpen && "rotate-90",
												)}
											/>
											{isOpen ? (
												<IconFolderOpen
													size={15}
													stroke={1.75}
													className="shrink-0 text-ink-2"
													aria-hidden
												/>
											) : (
												<IconFolder
													size={15}
													stroke={1.75}
													className="shrink-0 text-ink-3"
													aria-hidden
												/>
											)}
											<span className="min-w-0 flex-1 truncate text-left">
												{name}
											</span>
											</Collapsible.Trigger>
										</ContextMenu>

										<Button
											variant="ghost"
											className={cn(iconBtn, "mr-0.5")}
											title="New chat"
											aria-label={`New chat in ${name}`}
											onClick={(e) => handleNewChat(project.id, e)}
										>
																			<IconPlus size={15} stroke={1.75} aria-hidden />
										</Button>
									</div>

									<Collapsible.Panel
										keepMounted={false}
										render={
											<ul role="group" className="flex flex-col gap-px" />
										}
									>
										{chats.length === 0 ? (
											<li className="py-1 pr-2 pl-8 text-[12.5px] text-ink-3">
												No chats yet
											</li>
										) : (
											chats.map((c) => {
												const isActiveChat = c.id === activeChatId;
												return (
													<li key={c.id} role="treeitem">
														<ContextMenu
															items={[
																{ label: "Rename", onSelect: () => renameChat(c) },
																{ label: "Duplicate", disabled: true },
																{ label: "New chat here", onSelect: () => void handleNewChat(project.id) },
																"separator",
																{ label: "Reveal project folder", disabled: true },
																{ label: "Delete chat…", tone: "danger", onSelect: () => removeChat(c) },
															]}
														>
																		<Button
																			variant="ghost"
																			data-tree-node
															onClick={() => {
																setActiveProject(project.id);
																setActiveChat(c.id);
															}}
															className={cn(
																listRow,
																"flex items-center gap-1.5 py-1 pr-2 pl-7 text-[13px]",
																isActiveChat ? listRowActive : listRowIdle,
															)}
														>
															<IconMessage
																size={14}
																stroke={1.75}
																className={cn(
																	"shrink-0",
																	isActiveChat ? "text-accent" : "text-ink-3",
																)}
																aria-hidden
															/>
															<span className="min-w-0 flex-1 truncate text-left">
																{c.title}
															</span>
																			</Button>
														</ContextMenu>
													</li>
												);
											})
										)}
									</Collapsible.Panel>
								</Collapsible.Root>
							);
						})}
					</ul>
				)}
			</div>

			<div className="shrink-0 border-t border-line px-2 py-2">
				<Button
					variant="ghost"
					className="w-full justify-start gap-2.5 px-2.5"
					onClick={() => setSettingsOpen(true)}
				>
					<IconSettings size={16} stroke={1.75} aria-hidden />
					Settings
				</Button>
			</div>
		</aside>
	);
}
