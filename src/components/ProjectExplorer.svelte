<script lang="ts">
  import { onMount } from 'svelte';
  import { Tabs } from 'bits-ui';
  import { IconAlertCircle, IconLayoutSidebarRight, IconRefresh } from '@tabler/icons-svelte';

  import ProjectFileNode from './ProjectFileNode.svelte';
  import ChangesPanel from './ChangesPanel.svelte';
  import {
    readProjectDirectory,
    startProjectFileWatcher,
    stopProjectFileWatcher,
    type ProjectFileEntry,
  } from '../services/native';
  interface Props {
    open: boolean;
    visible: boolean;
    projectRoot: string;
    projectName: string;
    activeFilePath: string | null;
    currentBranch: string | null | undefined;
    branches: string[] | null | undefined;
    toggle: () => void;
    openFile: (path: string) => void;
    filesChanged: (paths: string[]) => void;
    startResize: (event: PointerEvent) => void;
  }

  const {
    open, visible, projectRoot, projectName, activeFilePath, currentBranch, branches,
    toggle, openFile, filesChanged, startResize,
  }: Props = $props();
  let entries = $state<ProjectFileEntry[]>([]);
  let loading = $state(true);
  let loadError = $state('');
  let notice = $state('');
  let revision = $state(0);
  let refreshTimer: number | undefined;
  let loadToken = 0;
  let tree = $state<HTMLUListElement>();
  let focusedPath = $state<string | null>(null);
  let view = $state<'files' | 'changes'>('files');

  onMount(() => {
    let disposed = false;
    let watcherId: number | undefined;

    void startProjectFileWatcher(projectRoot, (change) => {
      if (disposed) return;
      filesChanged(change.paths);
      window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => { revision += 1; }, 160);
    }).then((id) => {
      if (disposed) void stopProjectFileWatcher(id);
      else watcherId = id;
    }).catch((error) => {
      if (!disposed) notice = `Live updates unavailable: ${errorMessage(error)}`;
    });

    return () => {
      disposed = true;
      window.clearTimeout(refreshTimer);
      if (watcherId !== undefined) void stopProjectFileWatcher(watcherId);
    };
  });

  $effect(() => {
    void loadRoot(revision);
  });

  $effect(() => {
    if (currentBranch === null) view = 'files';
  });

  $effect(() => {
    if (!tree) return;
    const observer = new MutationObserver(resetMissingTreeFocus);
    observer.observe(tree, { childList: true, subtree: true });
    resetMissingTreeFocus();
    return () => observer.disconnect();
  });

  async function loadRoot(requestedRevision: number) {
    const token = ++loadToken;
    loading = true;
    loadError = '';
    try {
      const nextEntries = await readProjectDirectory(projectRoot, projectRoot);
      if (token !== loadToken || requestedRevision !== revision) return;
      entries = nextEntries;
    } catch (error) {
      if (token !== loadToken) return;
      loadError = errorMessage(error);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  function resetMissingTreeFocus() {
    const items = Array.from(tree?.querySelectorAll<HTMLButtonElement>('[role="treeitem"]') ?? []);
    if (!items.some((item) => item.dataset.path === focusedPath)) focusedPath = items[0]?.dataset.path ?? null;
  }

  function focusTreeItem(item: HTMLButtonElement | undefined) {
    if (!item) return;
    focusedPath = item.dataset.path ?? null;
    item.focus();
  }

  function handleTreeKeydown(event: KeyboardEvent) {
    const current = event.target instanceof HTMLElement
      ? event.target.closest<HTMLButtonElement>('[role="treeitem"]')
      : null;
    if (!current || !tree) return;
    const items = Array.from(tree.querySelectorAll<HTMLButtonElement>('[role="treeitem"]'));
    const index = items.indexOf(current);
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      focusTreeItem(event.key === 'Home'
        ? items[0]
        : event.key === 'End'
          ? items.at(-1)
          : items[index + (event.key === 'ArrowDown' ? 1 : -1)]);
      return;
    }
    if (event.key === 'ArrowRight' && current.hasAttribute('aria-expanded')) {
      event.preventDefault();
      if (current.getAttribute('aria-expanded') === 'false') current.click();
      else focusTreeItem(current.closest('li')?.querySelector<HTMLButtonElement>(':scope > ul > li > [role="treeitem"]') ?? undefined);
      return;
    }
    if (event.key === 'ArrowLeft') {
      const parent = current.closest('li')?.parentElement?.closest('li')?.querySelector<HTMLButtonElement>(':scope > [role="treeitem"]');
      if (current.getAttribute('aria-expanded') === 'true') {
        event.preventDefault();
        current.click();
      } else if (parent) {
        event.preventDefault();
        focusTreeItem(parent);
      }
    }
  }

  function setView(value: string) {
    if (value !== 'files' && value !== 'changes') return;
    view = value;
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<aside class={`relative col-start-3 flex min-h-0 min-w-0 flex-col border-l border-line bg-[var(--sidebar-tint)] ${visible ? '' : 'pointer-events-none opacity-0'}`} aria-label="Project explorer">
  <div
    class="absolute top-0 bottom-0 left-[-4px] z-4 w-2 touch-none cursor-ew-resize before:absolute before:top-0 before:bottom-0 before:left-1 before:w-px before:bg-transparent hover:before:bg-line-strong"
    role="separator"
    aria-label="Resize right sidebar"
    aria-orientation="vertical"
    onpointerdown={startResize}
  ></div>

  <div class="flex h-[41px] shrink-0 items-center justify-end gap-1 px-2 pt-2 pb-1" data-tauri-drag-region>
    <button
      type="button"
      class="grid size-[25px] place-items-center rounded-md text-muted hover:bg-panel-hover hover:text-ink-soft"
      aria-label={view === 'files' ? 'Refresh project files' : 'Refresh Git changes'}
      title={view === 'files' ? 'Refresh project files' : 'Refresh Git changes'}
      onclick={() => { revision += 1; }}
    >
      <IconRefresh size={14} stroke={1.55} />
    </button>
    <button
      type="button"
      class={`grid size-[25px] place-items-center rounded-md text-muted hover:bg-panel-hover hover:text-ink-soft ${visible ? 'bg-panel-hover text-ink-soft' : ''}`}
      aria-label={visible ? 'Collapse right sidebar' : 'Expand right sidebar'}
      aria-pressed={visible}
      title={visible ? 'Collapse right sidebar' : 'Expand right sidebar'}
      onclick={toggle}
    >
      <IconLayoutSidebarRight size={14} stroke={1.55} />
    </button>
  </div>

  <Tabs.Root class="flex min-h-0 flex-1 flex-col" value={view} onValueChange={setView}>
    <Tabs.List class="flex shrink-0 gap-1 border-b border-line px-2 py-1" aria-label="Right sidebar views">
      <Tabs.Trigger class="rounded px-2 py-1 text-[11px] text-muted hover:bg-panel-hover data-[state=active]:bg-panel-active data-[state=active]:text-ink-soft" value="files">Files</Tabs.Trigger>
      {#if currentBranch !== null}<Tabs.Trigger class="rounded px-2 py-1 text-[11px] text-muted hover:bg-panel-hover data-[state=active]:bg-panel-active data-[state=active]:text-ink-soft" value="changes">Changes</Tabs.Trigger>{/if}
    </Tabs.List>
    <Tabs.Content class="min-h-0 flex-1 overflow-auto p-2" value="files">
      {#if loading && entries.length === 0}
        <p class="p-2 text-xs text-faint">Loading project…</p>
      {:else if loadError}
        <p class="p-2 text-xs text-danger" title={loadError}>Could not read this project.</p>
      {:else if entries.length === 0}
        <p class="p-2 text-xs text-faint">This project is empty.</p>
      {:else}
        <ul
          bind:this={tree}
          class="m-0 list-none p-0"
          role="tree"
          aria-label={`${projectName} files`}
          onkeydown={handleTreeKeydown}
        >
          {#each entries as entry (entry.path)}
            <ProjectFileNode
              {entry}
              {projectRoot}
              depth={0}
              {revision}
              {activeFilePath}
              {focusedPath}
              setFocusedPath={(path) => { focusedPath = path; }}
              {openFile}
              reportError={(message) => { notice = message; }}
            />
          {/each}
        </ul>
      {/if}
    </Tabs.Content>
    {#if currentBranch !== null}
      <Tabs.Content class="min-h-0 flex-1 overflow-auto p-2" value="changes">
        {#if view === 'changes'}
          <ChangesPanel
            cwd={projectRoot}
            {currentBranch}
            {branches}
            {revision}
          />
        {/if}
      </Tabs.Content>
    {/if}
  </Tabs.Root>

  {#if notice}
    <button class="m-2 flex min-w-0 items-center gap-1.5 rounded-md border border-warning/30 bg-panel px-2 py-1.5 text-left text-[11px] text-warning" title={notice} onclick={() => { notice = ''; }}>
      <IconAlertCircle size={13} stroke={1.55} />
      <span class="truncate">{notice}</span>
    </button>
  {/if}
</aside>
