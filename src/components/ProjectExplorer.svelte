<script lang="ts">
  import { onMount } from 'svelte';
  import { Tabs } from 'bits-ui';
  import { IconAlertCircle, IconLayoutSidebarRight, IconRefresh } from '@tabler/icons-svelte';

  import ProjectFileNode from './ProjectFileNode.svelte';
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
    colorMode: 'light' | 'dark';
    toggle: () => void;
    openFile: (path: string) => void;
    filesChanged: (paths: string[]) => void;
    startResize: (event: PointerEvent) => void;
  }

  const {
    open, visible, projectRoot, projectName, activeFilePath, currentBranch, branches, colorMode,
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
  let changesPanel = $state<Promise<typeof import('./ChangesPanel.svelte')>>();

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
    if (value === 'changes') changesPanel ??= import('./ChangesPanel.svelte');
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<aside class:open class="project-explorer" aria-label="Project explorer">
  <div
    class="sidebar-resize-handle right-sidebar-resize-handle"
    role="separator"
    aria-label="Resize right sidebar"
    aria-orientation="vertical"
    onpointerdown={startResize}
  ></div>

  <div class="project-explorer-actions" data-tauri-drag-region>
    <button
      type="button"
      class="chrome-button project-explorer-refresh"
      aria-label={view === 'files' ? 'Refresh project files' : 'Refresh Git changes'}
      title={view === 'files' ? 'Refresh project files' : 'Refresh Git changes'}
      onclick={() => { revision += 1; }}
    >
      <IconRefresh size={14} stroke={1.55} />
    </button>
    <button
      type="button"
      class="chrome-button project-explorer-toggle"
      class:active={visible}
      aria-label={visible ? 'Collapse right sidebar' : 'Expand right sidebar'}
      aria-pressed={visible}
      title={visible ? 'Collapse right sidebar' : 'Expand right sidebar'}
      onclick={toggle}
    >
      <IconLayoutSidebarRight size={14} stroke={1.45} />
    </button>
  </div>

  <Tabs.Root class="project-explorer-tabs" value={view} onValueChange={setView}>
    <Tabs.List class="project-explorer-tab-list" aria-label="Right sidebar views">
      <Tabs.Trigger value="files">Files</Tabs.Trigger>
      <Tabs.Trigger value="changes">Changes</Tabs.Trigger>
    </Tabs.List>
    <Tabs.Content class="project-explorer-scroll" value="files">
      {#if loading && entries.length === 0}
        <p class="project-explorer-empty">Loading project…</p>
      {:else if loadError}
        <p class="project-explorer-empty error" title={loadError}>Could not read this project.</p>
      {:else if entries.length === 0}
        <p class="project-explorer-empty">This project is empty.</p>
      {:else}
        <ul
          bind:this={tree}
          class="project-file-tree"
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
    <Tabs.Content class="project-explorer-scroll project-changes-scroll" value="changes">
      {#if changesPanel}
        {#await changesPanel}
          <p class="project-explorer-empty">Loading changes…</p>
        {:then { default: ChangesPanel }}
          <ChangesPanel
            cwd={projectRoot}
            {currentBranch}
            {branches}
            {revision}
            {colorMode}
          />
        {/await}
      {/if}
    </Tabs.Content>
  </Tabs.Root>

  {#if notice}
    <button class="project-explorer-notice" title={notice} onclick={() => { notice = ''; }}>
      <IconAlertCircle size={13} stroke={1.6} />
      <span>{notice}</span>
    </button>
  {/if}
</aside>
