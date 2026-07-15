import { useState, type ComponentType } from "react";
import { IconFileMinus, IconFilePencil, IconFilePlus } from "@tabler/icons-react";
import { Button, cn, Chip, ContextMenu, listRow, listRowIdle, SelectField } from "../../design";
import { openExternal } from "../../ipc/commands";
import { useDiffs } from "../../ipc/hooks";
import { copyText } from "../../lib/clipboard";
import { useSession } from "../../store/session";
import type { DiffChange, DiffScope } from "../../ipc/types";

// "buffer_vs_disk" and "vs_checkpoint" exist in Core but have no UI inputs yet
// (no in-app editor buffers, no checkpoint picker) — offer only working scopes.
const SCOPES: { value: DiffScope; label: string }[] = [
  { value: "since_chat_start", label: "Since chat start" },
  { value: "this_run", label: "This turn" },
];

type IconType = ComponentType<{
  size?: number;
  stroke?: number;
  className?: string;
  "aria-hidden"?: boolean;
}>;

const KIND_META: Record<string, { icon: IconType; className: string; label: string }> = {
  add: { icon: IconFilePlus, className: "text-accent", label: "Added" },
  modify: { icon: IconFilePencil, className: "text-ink-3", label: "Modified" },
  delete: { icon: IconFileMinus, className: "text-clay", label: "Deleted" },
};

/** Split "src/features/DiffsPanel.tsx" → dimmed dir + prominent basename. */
function splitPath(path: string): { dir: string; base: string } {
  const normalized = path.replace(/\\/g, "/");
  const idx = normalized.lastIndexOf("/");
  if (idx < 0) return { dir: "", base: normalized };
  return { dir: normalized.slice(0, idx + 1), base: normalized.slice(idx + 1) };
}

function EmptyState({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="grid h-full place-items-center p-6 text-center">
      <div>
        <p className="text-[13.5px] font-medium text-ink">{title}</p>
        <p className="mt-1 max-w-[16rem] text-[12.5px] leading-relaxed text-ink-3">{hint}</p>
      </div>
    </div>
  );
}

export function DiffsPanel() {
  const { activeChatId, activeRunId, lastRunId, activeProjectId } = useSession();
  const [scope, setScope] = useState<DiffScope>("since_chat_start");
  // After a run completes activeRunId is nulled; fall back to the last run so
  // "This turn" keeps showing the most recent turn's changes.
  const runId = activeRunId ?? lastRunId;
  const { data } = useDiffs({ chatId: activeChatId, scope, runId, projectId: activeProjectId });
  const changes = data?.changes ?? [];

  if (!activeChatId) {
    return (
      <EmptyState
        title="No chat selected"
        hint="Select a chat and any file changes from that conversation will show up here."
      />
    );
  }

  const totalAdded = changes.reduce((n, c) => n + (c.linesAdded ?? 0), 0);
  const totalRemoved = changes.reduce((n, c) => n + (c.linesRemoved ?? 0), 0);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between gap-2 border-b border-line py-1.5 pl-1 pr-2.5">
        <SelectField
          value={scope}
          onValueChange={setScope}
          items={SCOPES}
          aria-label="Diff scope"
          variant="ghost"
        />
        {changes.length > 0 && (
          <span className="shrink-0 font-mono text-[11.5px] tabular-nums text-ink-3">
            {changes.length === 1 ? "1 file" : `${changes.length} files`}
            {(totalAdded > 0 || totalRemoved > 0) && (
              <>
                {" · "}
                <span className="text-accent">+{totalAdded}</span>{" "}
                <span className="text-clay">−{totalRemoved}</span>
              </>
            )}
          </span>
        )}
      </div>
      {changes.length === 0 ? (
        <EmptyState
          title={scope === "this_run" ? "No changes this turn" : "No changes yet"}
          hint={
            scope === "this_run" && !runId
              ? "Send a message first — changes from the latest turn will be listed here."
              : "When the agent edits files, each changed file will be listed here."
          }
        />
      ) : (
        <ul className="flex min-h-0 flex-1 flex-col gap-px overflow-auto px-1.5 py-1.5">
          {changes.map((c) => (
            <ChangeRow key={c.path} change={c} projectId={activeProjectId} />
          ))}
        </ul>
      )}
    </div>
  );
}

function ChangeRow({ change: c, projectId }: { change: DiffChange; projectId: string | null }) {
  const kind = KIND_META[c.kind] ?? KIND_META.modify;
  const KindIcon = kind.icon;
  const { dir, base } = splitPath(c.path);
  const deleted = c.kind === "delete";
  // Agent authorship is the norm; only surface the exceptions.
  const attribution = c.attribution === "manual" || c.attribution === "external" ? c.attribution : null;
  const hasStats = typeof c.linesAdded === "number" || typeof c.linesRemoved === "number";

  return (
    <li>
      <ContextMenu
        items={[
          {
            label: "Open file",
            disabled: deleted,
            onSelect: () => {
              if (projectId) void openExternal(projectId, c.path);
            },
          },
          { label: "Copy path", onSelect: () => void copyText(c.path) },
          { label: "View full diff", disabled: true },
          "separator",
          { label: "Accept / keep", disabled: true },
          { label: "Copy diff", disabled: true },
          { label: "Revert change", tone: "danger", disabled: true },
        ]}
      >
        <Button
          variant="ghost"
          onClick={() => {
            if (!deleted && projectId) void openExternal(projectId, c.path);
          }}
          title={`${kind.label} · ${c.path}`}
          className={cn(listRow, listRowIdle, "flex items-center gap-1.5 py-1.5 pl-2 pr-2 text-[13px]")}
        >
          <KindIcon size={14} stroke={1.75} className={cn("shrink-0", kind.className)} aria-hidden />
          <span className="sr-only">{kind.label}</span>
          <span className="flex min-w-0 flex-1 items-baseline">
            <span
              className={cn(
                "min-w-0 truncate font-medium",
                deleted ? "text-clay line-through decoration-clay/60" : "text-ink",
              )}
            >
              {base}
            </span>
            {dir && (
              <span className="ml-1.5 min-w-0 truncate text-[12px] text-ink-3">{dir}</span>
            )}
          </span>
          {attribution && <Chip quiet>{attribution}</Chip>}
          {hasStats && (
            <span className="shrink-0 font-mono text-[11.5px] tabular-nums">
              <span className="text-accent">+{c.linesAdded ?? 0}</span>{" "}
              <span className="text-clay">−{c.linesRemoved ?? 0}</span>
            </span>
          )}
        </Button>
      </ContextMenu>
    </li>
  );
}
