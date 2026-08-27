<script lang="ts">
  import { IconChevronRight, IconGitBranch } from '@tabler/icons-svelte';

  import {
    getGitFileDiff,
    listGitChanges,
    type GitChange,
    type GitFileDiff,
  } from '../services/native';
  import { gitDiffLines } from '../utils/git-diff';
  import { materialFileIcon } from '../utils/material-file-icons';
  import MotionRotate from './motion/MotionRotate.svelte';

  interface Props {
    cwd: string;
    currentBranch: string | null | undefined;
    branches: string[] | null | undefined;
    revision: number;
  }

  type ComparisonMode = 'working' | 'branch';
  type DiffState =
    | { kind: 'loading' }
    | { kind: 'ready'; diff: GitFileDiff }
    | { kind: 'error'; message: string };

  const props: Props = $props();
  let mode = $state<ComparisonMode>('working');
  let baseBranch = $state('');
  let changes = $state<GitChange[]>([]);
  let expanded = $state<string[]>([]);
  let diffs = $state<Record<string, DiffState>>({});
  let loading = $state(true);
  let loadError = $state('');
  let summary = $state({ additions: 0, deletions: 0 });
  let generation = 0;
  let loadedComparison = '';
  let hasLoaded = false;
  const comparisonBranches = $derived(
    (props.branches ?? []).filter((branch) => branch !== props.currentBranch),
  );

  const statusColors: Record<GitChange['status'], string> = {
    added: 'text-success',
    untracked: 'text-success',
    copied: 'text-success',
    deleted: 'text-danger',
    conflicted: 'text-danger',
    modified: 'text-warning',
    renamed: 'text-warning',
    typeChanged: 'text-warning',
  };

  $effect(() => {
    if (comparisonBranches.includes(baseBranch)) return;
    baseBranch = comparisonBranches.find((branch) => branch === 'main')
      ?? comparisonBranches.find((branch) => branch === 'master')
      ?? comparisonBranches[0]
      ?? '';
  });

  $effect(() => {
    const revision = props.revision;
    const activeBase = mode === 'branch' ? baseBranch : null;
    const comparison = `${mode}:${activeBase ?? ''}`;
    void revision;
    if (comparison !== loadedComparison) {
      loadedComparison = comparison;
      hasLoaded = false;
      changes = [];
      summary = { additions: 0, deletions: 0 };
      expanded = [];
      diffs = {};
    }
    const token = ++generation;
    if (props.currentBranch === undefined || (mode === 'branch' && !activeBase)) {
      changes = [];
      summary = { additions: 0, deletions: 0 };
      diffs = {};
      loading = props.currentBranch === undefined;
      loadError = '';
      return;
    }
    void loadChanges(activeBase, token);
    return () => {
      if (generation === token) generation += 1;
    };
  });

  async function loadChanges(activeBase: string | null, token: number) {
    const initialLoad = !hasLoaded;
    loading = initialLoad;
    loadError = '';
    try {
      const result = await listGitChanges(props.cwd, activeBase);
      if (token !== generation) return;
      changes = result.changes;
      summary = { additions: result.additions, deletions: result.deletions };
      const available = new Set(result.changes.map(changeKey));
      expanded = expanded.filter((key) => available.has(key));
      diffs = Object.fromEntries(
        Object.entries(diffs).filter(([key]) => available.has(key)),
      );
      for (const change of result.changes) {
        if (expanded.includes(changeKey(change))) void loadDiff(change, activeBase, token, false);
      }
    } catch (error) {
      if (token !== generation || !initialLoad) return;
      changes = [];
      summary = { additions: 0, deletions: 0 };
      diffs = {};
      loadError = errorMessage(error);
    } finally {
      if (token === generation) {
        hasLoaded = true;
        loading = false;
      }
    }
  }

  function toggleChange(change: GitChange) {
    const key = changeKey(change);
    if (expanded.includes(key)) {
      expanded = expanded.filter((candidate) => candidate !== key);
      return;
    }
    expanded = [...expanded, key];
    if (!diffs[key]) {
      void loadDiff(change, mode === 'branch' ? baseBranch : null, generation);
    }
  }

  async function loadDiff(
    change: GitChange,
    activeBase: string | null,
    token: number,
    showLoading = true,
  ) {
    const key = changeKey(change);
    const previous = diffs[key];
    if (showLoading || !previous) diffs = { ...diffs, [key]: { kind: 'loading' } };
    try {
      const diff = await getGitFileDiff(props.cwd, activeBase, change.path, change.oldPath);
      if (token !== generation || !expanded.includes(key)) return;
      if (!showLoading && previous?.kind === 'ready' && sameDiff(previous.diff, diff)) return;
      diffs = { ...diffs, [key]: { kind: 'ready', diff } };
    } catch (error) {
      if (token !== generation || !expanded.includes(key)) return;
      diffs = { ...diffs, [key]: { kind: 'error', message: errorMessage(error) } };
    }
  }

  function changeKey(change: GitChange) {
    return `${change.oldPath ?? ''}\0${change.path}`;
  }

  function sameDiff(left: GitFileDiff, right: GitFileDiff) {
    return left.binary === right.binary
      && left.tooLarge === right.tooLarge
      && left.hunks.length === right.hunks.length
      && left.hunks.every((hunk, index) => hunk === right.hunks[index]);
  }

  function statusCode(status: GitChange['status']) {
    switch (status) {
      case 'added': return 'A';
      case 'modified': return 'M';
      case 'deleted': return 'D';
      case 'renamed': return 'R';
      case 'copied': return 'C';
      case 'untracked': return '?';
      case 'conflicted': return 'U';
      case 'typeChanged': return 'T';
    }
  }

  function changeTitle(change: GitChange) {
    if (mode === 'branch') return `${change.path} changed since ${baseBranch}`;
    const areas = [change.staged ? 'staged' : '', change.unstaged ? 'unstaged' : '']
      .filter(Boolean)
      .join(' and ');
    return `${change.oldPath ? `${change.oldPath} → ` : ''}${change.path}${areas ? ` · ${areas}` : ''}`;
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

{#if changes.length > 0}
  <div
    class="flex min-h-[29px] items-center gap-2 border-b border-line px-[11px] py-1.5 text-[11px] tabular-nums text-faint"
    aria-label={`${changes.length} changed ${changes.length === 1 ? 'file' : 'files'}, ${summary.additions} additions, ${summary.deletions} deletions`}
  >
    <strong class="mr-auto font-medium text-text-soft">{changes.length} {changes.length === 1 ? 'file' : 'files'}</strong>
    <span class="text-success">+{summary.additions}</span>
    <span class="text-danger">-{summary.deletions}</span>
  </div>
{/if}

<div class="flex flex-wrap items-center gap-[7px] border-b border-line px-[7px] pt-[3px] pb-2">
  <div class="grid auto-cols-max grid-flow-col rounded-[7px] border border-line bg-recessed p-0.5" role="group" aria-label="Diff comparison">
    <button
      class="h-6 rounded-[5px] border-0 bg-transparent px-[7px] text-[11px] text-faint hover:text-text-soft {mode === 'working' ? 'bg-panel-active text-text shadow-[inset_0_0_0_1px_var(--line)]' : ''}"
      aria-pressed={mode === 'working'}
      onclick={() => { mode = 'working'; }}
    >
      Working tree
    </button>
    <button
      class="h-6 rounded-[5px] border-0 bg-transparent px-[7px] text-[11px] text-faint hover:text-text-soft {mode === 'branch' ? 'bg-panel-active text-text shadow-[inset_0_0_0_1px_var(--line)]' : ''}"
      aria-pressed={mode === 'branch'}
      onclick={() => { mode = 'branch'; }}
    >
      Branch
    </button>
  </div>
  {#if mode === 'branch'}
    <label class="flex h-[25px] min-w-0 flex-[1_1_100%] items-center gap-[5px] text-[11px] text-faint [&_span]:shrink-0 [&_svg]:shrink-0">
      <IconGitBranch size={12} stroke={1.55} />
      <span>Base</span>
      <select
        class="h-[25px] min-w-0 flex-1 rounded-md border border-line bg-recessed px-[7px] pr-[22px] text-[11px] text-text-soft"
        value={baseBranch}
        aria-label="Base branch"
        disabled={comparisonBranches.length === 0}
        onchange={(event) => { baseBranch = event.currentTarget.value; }}
      >
        {#each comparisonBranches as branch (branch)}
          <option value={branch}>{branch}</option>
        {/each}
      </select>
    </label>
  {/if}
  {#if loading}
    <span class="min-w-0 text-[11px] text-faint">Loading…</span>
  {:else if loadError}
    <span class="min-w-0 text-[11px] text-danger" title={loadError}>Could not read changes</span>
  {:else if mode === 'branch' && comparisonBranches.length === 0}
    <span class="min-w-0 text-[11px] text-faint">No branch to compare</span>
  {:else if changes.length === 0}
    <span class="min-w-0 text-[11px] text-faint">No changes</span>
  {/if}
</div>

{#if changes.length > 0}
  <div class="px-[5px] pb-2.5">
    {#each changes as change, index (changeKey(change))}
      {@const key = changeKey(change)}
      {@const state = diffs[key]}
      {@const isExpanded = expanded.includes(key)}
      <section class="min-w-0 border-b border-[color-mix(in_srgb,var(--line)_68%,transparent)]">
        <button
          class="flex min-h-[31px] w-full min-w-0 items-center gap-[5px] overflow-hidden rounded-[5px] border-0 bg-transparent py-[3px] pr-1.5 pl-[3px] text-left text-muted hover:bg-panel-hover hover:text-text-soft"
          title={changeTitle(change)}
          aria-expanded={isExpanded}
          aria-controls={`git-change-diff-${index}`}
          onclick={() => toggleChange(change)}
        >
          <MotionRotate active={isExpanded} class="shrink-0 text-faint">
            <IconChevronRight size={12} stroke={1.55} />
          </MotionRotate>
          <img
            class="size-[15px] shrink-0 opacity-[0.64] [filter:var(--provider-filter)]"
            src={materialFileIcon(change.path.split('/').at(-1) ?? change.path)}
            alt=""
          />
          <span class="flex min-w-0 flex-1 flex-col leading-tight [&_small]:truncate [&_small]:text-[11px] [&_small]:text-faint [&_strong]:truncate [&_strong]:text-[11px] [&_strong]:font-medium">
            {#if change.oldPath}<small>{change.oldPath} →</small>{/if}
            <strong>{change.path}</strong>
          </span>
          <span class="w-[15px] shrink-0 text-center font-mono text-[11px] font-[650] {statusColors[change.status]}">{statusCode(change.status)}</span>
        </button>
        {#if isExpanded}
          <div id={`git-change-diff-${index}`} class="mx-0.5 mb-1.5 min-w-0 overflow-hidden rounded-md border border-line bg-recessed">
            {#if !state || state.kind === 'loading'}
              <p class="m-0 p-2 text-[11px] leading-snug text-faint">Loading diff…</p>
            {:else if state.kind === 'error'}
              <p class="m-0 p-2 text-[11px] leading-snug text-danger" title={state.message}>Could not read this diff.</p>
            {:else if state.diff.tooLarge}
              <p class="m-0 p-2 text-[11px] leading-snug text-faint">This diff is larger than 2 MB.</p>
            {:else if state.diff.binary}
              <p class="m-0 p-2 text-[11px] leading-snug text-faint">Binary files cannot be previewed.</p>
            {:else if state.diff.hunks.length === 0}
              <p class="m-0 p-2 text-[11px] leading-snug text-faint">No text diff to display.</p>
            {:else}
              <div class="max-h-[280px] overflow-auto py-1 font-mono text-[11px] leading-snug tabular-nums">
                {#each gitDiffLines(state.diff) as line, lineIndex (`${lineIndex}:${line.kind}`)}
                  <span class="block px-2 break-anywhere whitespace-pre-wrap {line.kind === 'add' ? 'bg-[color-mix(in_srgb,var(--success)_13%,transparent)]' : ''} {line.kind === 'del' ? 'bg-[color-mix(in_srgb,var(--danger)_13%,transparent)]' : ''} {line.kind === 'hunk' ? 'bg-[color-mix(in_srgb,var(--muted)_8%,transparent)] text-faint' : ''}">{line.text || ' '}</span>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </section>
    {/each}
  </div>
{/if}
