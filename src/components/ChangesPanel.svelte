<script lang="ts">
  import { DiffModeEnum, DiffView } from '@git-diff-view/svelte';
  import '@git-diff-view/svelte/styles/diff-view-pure.css';
  import { IconChevronRight, IconGitBranch } from '@tabler/icons-svelte';

  import {
    getGitFileDiff,
    listGitChanges,
    type GitChange,
    type GitFileDiff,
  } from '../services/native';
  import { gitDiffViewData } from '../utils/git-diff';
  import { materialFileIcon } from '../utils/material-file-icons';

  interface Props {
    cwd: string;
    currentBranch: string | null | undefined;
    branches: string[] | null | undefined;
    revision: number;
    colorMode: 'light' | 'dark';
  }

  type ComparisonMode = 'working' | 'branch';
  type DiffState =
    | { kind: 'loading' }
    | { kind: 'ready'; diff: GitFileDiff; data: ReturnType<typeof gitDiffViewData> }
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
      diffs = { ...diffs, [key]: { kind: 'ready', diff, data: gitDiffViewData(change, diff) } };
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

{#if !loading && !loadError}
  <div
    class="git-change-summary"
    aria-label={`${changes.length} changed ${changes.length === 1 ? 'file' : 'files'}, ${summary.additions} additions, ${summary.deletions} deletions`}
  >
    <strong>{changes.length} {changes.length === 1 ? 'file' : 'files'}</strong>
    <span class="additions">+{summary.additions}</span>
    <span class="deletions">-{summary.deletions}</span>
  </div>
{/if}

<div class="git-changes-controls">
  <div class="git-comparison-switch" role="group" aria-label="Diff comparison">
    <button class:active={mode === 'working'} aria-pressed={mode === 'working'} onclick={() => { mode = 'working'; }}>
      Working tree
    </button>
    <button class:active={mode === 'branch'} aria-pressed={mode === 'branch'} onclick={() => { mode = 'branch'; }}>
      Branch
    </button>
  </div>
  {#if mode === 'branch'}
    <label class="git-base-branch">
      <IconGitBranch size={12} stroke={1.55} />
      <span>Base</span>
      <select
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
</div>

{#if loading}
  <p class="project-explorer-empty">Loading changes…</p>
{:else if loadError}
  <p class="project-explorer-empty error" title={loadError}>Could not read Git changes.</p>
{:else if mode === 'branch' && comparisonBranches.length === 0}
  <p class="project-explorer-empty">No other local branch is available for comparison.</p>
{:else if changes.length === 0}
  <p class="project-explorer-empty">
    {mode === 'working' ? 'No working-tree changes.' : `No changes since ${baseBranch}.`}
  </p>
{:else}
  <div class="git-change-list">
    {#each changes as change, index (changeKey(change))}
      {@const key = changeKey(change)}
      {@const state = diffs[key]}
      {@const isExpanded = expanded.includes(key)}
      <section class:expanded={isExpanded} class="git-change-item">
        <button
          class="git-change-row"
          title={changeTitle(change)}
          aria-expanded={isExpanded}
          aria-controls={`git-change-diff-${index}`}
          onclick={() => toggleChange(change)}
        >
          <IconChevronRight class="git-change-chevron" size={12} stroke={1.65} />
          <img src={materialFileIcon(change.path.split('/').at(-1) ?? change.path)} alt="" />
          <span class="git-change-path">
            {#if change.oldPath}<small>{change.oldPath} →</small>{/if}
            <strong>{change.path}</strong>
          </span>
          <span class={`git-change-status ${change.status}`}>{statusCode(change.status)}</span>
        </button>
        {#if isExpanded}
          <div id={`git-change-diff-${index}`} class="git-change-diff">
            {#if !state || state.kind === 'loading'}
              <p class="git-change-state">Loading diff…</p>
            {:else if state.kind === 'error'}
              <p class="git-change-state error" title={state.message}>Could not read this diff.</p>
            {:else if state.diff.tooLarge}
              <p class="git-change-state">This diff is larger than 2 MB.</p>
            {:else if state.diff.binary}
              <p class="git-change-state">Binary files cannot be previewed.</p>
            {:else if state.diff.hunks.length === 0}
              <p class="git-change-state">No text diff to display.</p>
            {:else}
              <DiffView
                data={state.data}
                diffViewMode={DiffModeEnum.Unified}
                diffViewTheme={props.colorMode}
                diffViewFontSize={11}
                diffViewWrap
                diffViewHighlight
                diffViewAddWidget={false}
              />
            {/if}
          </div>
        {/if}
      </section>
    {/each}
  </div>
{/if}
