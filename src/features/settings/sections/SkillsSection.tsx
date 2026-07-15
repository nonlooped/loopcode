import { useMemo, useState } from "react";
import {
  IconChevronDown,
  IconCode,
  IconFolder,
  IconRefresh,
} from "@tabler/icons-react";
import { Collapsible } from "@base-ui/react/collapsible";
import { Button, Chip, TextField, cn, pressable } from "../../../design";
import { loadSkill } from "../../../ipc/commands";
import { useSkills } from "../../../ipc/hooks";
import { isTauri } from "../../../ipc/client";
import type { SkillMeta } from "../../../ipc/types";
import { useSession } from "../../../store/session";
import { EmptyState, MonoBadge, SectionLabel, SettingsCard, StatusLine } from "../layout";

const ORIGIN_LABEL: Record<string, string> = {
  project: "skills/",
  loopcode: ".loopcode/skills/",
  agents: ".agents/skills/",
};

function originTone(origin: string): "accent" | "amber" | "neutral" {
  if (origin === "agents") return "accent";
  if (origin === "loopcode") return "amber";
  return "neutral";
}

function SkillRow({ skill, projectId }: { skill: SkillMeta; projectId: string }) {
  const [open, setOpen] = useState(false);
  const [body, setBody] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function ensureBody() {
    if (body != null || loading) return;
    setLoading(true);
    setError(null);
    try {
      const res = await loadSkill(projectId, skill.path);
      setBody(res.body);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <Collapsible.Root
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (next) void ensureBody();
      }}
    >
      <Collapsible.Trigger
        className={cn(
          pressable,
          "flex w-full items-start gap-3 border-b border-line px-4 py-3.5 text-left",
          "fine-hover:bg-surface-2/60",
          "focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent",
        )}
      >
        <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-accent-tint text-accent">
          <IconCode size={16} stroke={1.75} aria-hidden />
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex flex-wrap items-center gap-2">
            <span className="text-[14px] font-medium text-ink">{skill.name}</span>
            <Chip tone={originTone(skill.origin)} quiet>
              {ORIGIN_LABEL[skill.origin] ?? skill.origin}
            </Chip>
          </span>
          <span className="mt-0.5 block text-[13px] leading-snug text-ink-3">
            {skill.description || "No description in frontmatter."}
          </span>
          <span className="mt-1.5 flex flex-wrap items-center gap-2">
            {(skill.scripts?.length ?? 0) > 0 ? (
              <span className="text-[12px] text-ink-3">
                {skill.scripts.length} script{skill.scripts.length === 1 ? "" : "s"} · approval-gated
              </span>
            ) : (
              <span className="text-[12px] text-ink-3">No scripts</span>
            )}
          </span>
        </span>
        <IconChevronDown
          size={16}
          stroke={1.75}
          className={cn(
            "mt-1 shrink-0 text-ink-3 transition-transform duration-[var(--duration-fast)] ease-[var(--ease-out)]",
            open && "rotate-180",
          )}
          aria-hidden
        />
      </Collapsible.Trigger>

      <Collapsible.Panel className="border-b border-line bg-surface-2/40 px-4 py-3 last:border-b-0">
        <div className="flex flex-col gap-3">
          <div>
            <p className="text-[13px] font-semibold text-ink-2">
              Path
            </p>
            <p className="mt-1 font-mono text-[13px] break-all text-ink-2">{skill.path}</p>
          </div>

          {(skill.scripts?.length ?? 0) > 0 && (
            <div>
              <p className="text-[13px] font-semibold text-ink-2">
                Scripts
              </p>
              <ul className="mt-1.5 flex flex-wrap gap-1.5">
                {skill.scripts.map((s) => (
                  <li key={s}>
                    <MonoBadge>{s}</MonoBadge>
                  </li>
                ))}
              </ul>
              <p className="mt-1.5 text-[13px] text-ink-3">
                Scripts are never run from Settings. The agent must request approval first.
              </p>
            </div>
          )}

          <div>
            <p className="text-[13px] font-semibold text-ink-2">
              SKILL.md
            </p>
            {loading && <p className="mt-1.5 text-[13px] text-ink-3">Loading…</p>}
            {error && <StatusLine tone="error">{error}</StatusLine>}
            {body != null && (
              <pre className="mt-1.5 max-h-56 overflow-auto rounded-md border border-line bg-surface p-3 font-mono text-[13px] leading-relaxed whitespace-pre-wrap text-ink-2">
                {body}
              </pre>
            )}
          </div>
        </div>
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

export function SkillsSection() {
  const projectId = useSession((s) => s.activeProjectId);
  const skills = useSkills(projectId);
  const [query, setQuery] = useState("");
  const list = skills.data?.skills ?? [];

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return list;
    return list.filter((s) => {
      const hay = `${s.name} ${s.description} ${s.path} ${s.origin}`.toLowerCase();
      return hay.includes(q);
    });
  }, [list, query]);

  if (!projectId) {
    return (
      <SettingsCard>
        <EmptyState
          title="Open a project first"
          description="Skills are discovered on disk under the active project root. Open a folder as a project to scan for SKILL.md files."
        />
      </SettingsCard>
    );
  }

  return (
    <div className="flex flex-col gap-7">
      <div className="flex flex-wrap items-center gap-2">
        <TextField
          label={<span className="sr-only">Filter skills</span>}
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter by name, path, or origin…"
          className="min-w-0 flex-1 text-[13.5px]"
        />
        <Button
          variant="secondary"
          className="gap-1.5 px-3 py-2 text-[13px]"
          disabled={skills.isFetching}
          onClick={() => void skills.refetch()}
        >
          <IconRefresh size={15} stroke={1.75} aria-hidden />
          Refresh
        </Button>
      </div>

      <div>
        <div className="mb-2 flex items-baseline justify-between gap-3 px-0.5">
          <SectionLabel>Discovered</SectionLabel>
          <span className="text-[12px] text-ink-3">
            {list.length} skill{list.length === 1 ? "" : "s"}
            {query.trim() && filtered.length !== list.length
              ? ` · ${filtered.length} shown`
              : ""}
          </span>
        </div>
        <SettingsCard>
          {skills.isLoading ? (
            <p className="px-4 py-6 text-[13.5px] text-ink-3">Scanning project…</p>
          ) : skills.isError ? (
            <EmptyState
              title="Couldn’t load skills"
              description={String(skills.error)}
              action={
                <Button variant="secondary" className="px-3 py-2 text-[13px]" onClick={() => void skills.refetch()}>
                  Try again
                </Button>
              }
            />
          ) : list.length === 0 ? (
            <EmptyState
              title="No skills found"
              description={
                <>
                  Place a folder with a <code className="font-mono text-[12.5px]">SKILL.md</code>{" "}
                  under any of these paths in the project root:
                  <ul className="mt-2 flex flex-col gap-1">
                    <li className="flex items-center gap-1.5 font-mono text-[12.5px] text-ink-2">
                      <IconFolder size={14} stroke={1.75} aria-hidden />
                      .agents/skills/&lt;name&gt;/
                    </li>
                    <li className="flex items-center gap-1.5 font-mono text-[12.5px] text-ink-2">
                      <IconFolder size={14} stroke={1.75} aria-hidden />
                      skills/&lt;name&gt;/
                    </li>
                    <li className="flex items-center gap-1.5 font-mono text-[12.5px] text-ink-2">
                      <IconFolder size={14} stroke={1.75} aria-hidden />
                      .loopcode/skills/&lt;name&gt;/
                    </li>
                  </ul>
                  {!isTauri() && (
                    <span className="mt-2 block">
                      Browser preview cannot scan disk — run the desktop app to list skills.
                    </span>
                  )}
                </>
              }
              action={
                <Button variant="secondary" className="px-3 py-2 text-[13px]" onClick={() => void skills.refetch()}>
                  Scan again
                </Button>
              }
            />
          ) : filtered.length === 0 ? (
            <EmptyState
              title="No matches"
              description={`Nothing matches “${query.trim()}”.`}
              action={
                <Button variant="ghost" className="px-3 py-2 text-[13px]" onClick={() => setQuery("")}>
                  Clear filter
                </Button>
              }
            />
          ) : (
            filtered.map((s) => <SkillRow key={s.path} skill={s} projectId={projectId} />)
          )}
        </SettingsCard>
        <p className="mt-2 px-0.5 text-[12.5px] leading-snug text-ink-3">
          Listing is metadata only. Expanding a skill loads SKILL.md on demand; scripts stay
          approval-gated.
        </p>
      </div>
    </div>
  );
}
