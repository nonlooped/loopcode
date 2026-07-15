import { useState, type KeyboardEvent } from "react";
import {
  IconChevronRight,
  IconFile,
  IconFolder,
  IconFolderOpen,
} from "@tabler/icons-react";
import { Collapsible } from "@base-ui/react/collapsible";
import { Button, cn, ContextMenu, listRow, listRowIdle } from "../../design";
import { openExternal } from "../../ipc/commands";
import { copyText } from "../../lib/clipboard";
import { useTree } from "../../ipc/hooks";
import { useSession } from "../../store/session";
import type { TreeEntry } from "../../ipc/types";

function sortEntries(entries: TreeEntry[]): TreeEntry[] {
  return [...entries].sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
}

export function FilesPanel() {
  const projectId = useSession((s) => s.activeProjectId);
  // Always include excluded/noise entries; toggle removed from the UI for now.
  const { data: tree = [], isLoading, isError } = useTree(projectId, true);

  function open(path: string) {
    if (!projectId) return;
    void openExternal(projectId, path).catch(() => {
      // Open failures are silent in the tree; the OS/dialog surfaces errors.
    });
  }

  if (!projectId) {
    return (
      <div className="grid h-full place-items-center p-6 text-center">
        <div>
          <p className="text-[13.5px] font-medium text-ink">No project open</p>
          <p className="mt-1 max-w-[16rem] text-[12.5px] leading-relaxed text-ink-3">
            Open a folder from the left rail to browse its files here.
          </p>
        </div>
      </div>
    );
  }

  const entries = sortEntries(tree);

  function handleTreeKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const nodes = Array.from(
      event.currentTarget.querySelectorAll<HTMLElement>("[data-tree-node]"),
    );
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
      else current.closest<HTMLElement>("ul[role='group']")?.closest<HTMLElement>("li[role='treeitem']")?.querySelector<HTMLElement>("[data-tree-node]")?.focus();
    }
  }

  return (
    <div
      className="h-full min-h-0 overflow-auto px-1.5 py-1.5"
      role="tree"
      aria-label="Project files"
      onKeyDown={handleTreeKeyDown}
    >
      {isLoading && (
        <p className="px-2 py-3 text-[13px] text-ink-3">Loading tree…</p>
      )}
      {isError && (
        <p className="px-2 py-3 text-[13px] text-clay">Could not load project tree.</p>
      )}
      {!isLoading && !isError && entries.length === 0 && (
        <p className="px-2 py-3 text-[13px] text-ink-3">This project has no files to show.</p>
      )}
      {!isLoading && !isError && entries.length > 0 && (
        <ul className="flex flex-col gap-px">
          {entries.map((entry) => (
            <TreeNode key={entry.path} entry={entry} depth={0} onOpen={open} />
          ))}
        </ul>
      )}
    </div>
  );
}

function TreeNode({
  entry,
  depth,
  onOpen,
}: {
  entry: TreeEntry;
  depth: number;
  onOpen: (path: string) => void;
}) {
  return entry.isDir ? (
    <DirectoryNode entry={entry} depth={depth} onOpen={onOpen} />
  ) : (
    <FileNode entry={entry} depth={depth} onOpen={onOpen} />
  );
}

function FileNode({
  entry,
  depth,
  onOpen,
}: {
  entry: TreeEntry;
  depth: number;
  onOpen: (path: string) => void;
}) {
  return (
    <li role="treeitem">
      <ContextMenu
        items={[
          { label: "Open", onSelect: () => onOpen(entry.path) },
          "separator",
          { label: "Copy path", onSelect: () => void copyText(entry.path) },
          { label: "Copy relative path", onSelect: () => void copyText(entry.path) },
          { label: "Reveal in file manager", disabled: true },
          { label: "Add to chat context", disabled: true },
        ]}
      >
        <Button
          variant="ghost"
          data-tree-node
          type="button"
          onClick={() => onOpen(entry.path)}
          style={{ paddingLeft: `${depth * 14 + 8}px` }}
          title={`Open ${entry.path}`}
          className={cn(
            listRow,
            listRowIdle,
            "flex w-full items-center gap-1.5 py-1 pr-2 text-[13px]",
            entry.excluded && "opacity-55",
          )}
        >
          <span className="inline-block w-3.5 shrink-0" aria-hidden="true" />
          <IconFile size={14} stroke={1.75} className="shrink-0 text-ink-3" aria-hidden />
          <span className="min-w-0 flex-1 truncate">{entry.name}</span>
        </Button>
      </ContextMenu>
    </li>
  );
}

function DirectoryNode({
  entry,
  depth,
  onOpen,
}: {
  entry: TreeEntry;
  depth: number;
  onOpen: (path: string) => void;
}) {
  const children = entry.isDir ? sortEntries(entry.children ?? []) : [];
  const [open, setOpen] = useState(false);

  return (
    <Collapsible.Root open={open} onOpenChange={setOpen} render={<li role="treeitem" />}>
      <ContextMenu
        items={[
          { label: "New file", disabled: true },
          { label: "New folder", disabled: true },
          "separator",
          { label: "Copy path", onSelect: () => void copyText(entry.path) },
          { label: "Reveal in file manager", disabled: true },
          { label: "Collapse", onSelect: () => setOpen(false), disabled: !open },
        ]}
      >
        <Collapsible.Trigger
        data-tree-node
        data-tree-dir="true"
        data-tree-open={open ? "true" : "false"}
        style={{ paddingLeft: `${depth * 14 + 8}px` }}
        title={entry.path}
        className={cn(
          listRow,
          listRowIdle,
          // group so children can react to Trigger's data-panel-open
          "group flex w-full items-center gap-1.5 py-1 pr-2 text-[13px]",
          entry.excluded && "opacity-55",
        )}
      >
        <IconChevronRight
          size={14}
          stroke={1.75}
          aria-hidden
          className={cn(
            "shrink-0 text-ink-3 transition-transform duration-[var(--duration-fast)] ease-[var(--ease-out)]",
            "group-data-[panel-open]:rotate-90",
          )}
        />
        {children.length > 0 ? (
          <>
            <IconFolder
              size={14}
              stroke={1.75}
              className="shrink-0 text-ink-3 group-data-[panel-open]:hidden"
              aria-hidden
            />
            <IconFolderOpen
              size={14}
              stroke={1.75}
              className="hidden shrink-0 text-ink-2 group-data-[panel-open]:block"
              aria-hidden
            />
          </>
        ) : (
          <IconFolder size={14} stroke={1.75} className="shrink-0 text-ink-3" aria-hidden />
        )}
        <span className="min-w-0 flex-1 truncate">{entry.name}</span>
        </Collapsible.Trigger>
      </ContextMenu>
      <Collapsible.Panel
        keepMounted={false}
        render={<ul role="group" className="flex flex-col gap-px" />}
      >
        {children.map((child) => (
          <TreeNode key={child.path} entry={child} depth={depth + 1} onOpen={onOpen} />
        ))}
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}
