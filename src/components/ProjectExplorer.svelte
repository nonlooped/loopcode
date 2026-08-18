<script lang="ts">
  import IconAlertCircle from '@tabler/icons-svelte/icons/alert-circle';
  import IconLayoutSidebarRight from '@tabler/icons-svelte/icons/layout-sidebar-right';
  import IconRefresh from '@tabler/icons-svelte/icons/refresh';

  import ProjectFileNode from './ProjectFileNode.svelte';
  import {
    readProjectDirectory,
    startProjectFileWatcher,
    stopProjectFileWatcher,
    type ProjectFileEntry,
  } from '../services/native';
  import { changedParentDirectories, pathKey } from '../utils/project-file-changes';

  interface Props {
    open: boolean;
    visible: boolean;
    projectRoot: string;
    projectName: string;
    activeFilePath: string | null;
    toggle: () => void;
    openFile: (path: string) => void;
    filesChanged: (paths: string[]) => void;
    startResize: (event: PointerEvent) => void;
  }

  const {
    open, visible, projectRoot, projectName, activeFilePath, toggle, openFile, filesChanged, startResize,
  }: Props = $props();
  let entries = $state<ProjectFileEntry[]>([]);
  let loading = $state(true);
  let loadError = $state('');
  let notice = $state('');
  let directoryRevisions = $state<Record<string, number>>({});
  let manualRevision = $state(0);
  let loadToken = 0;

  $effect(() => {
    const watchedRoot = projectRoot;
    let disposed = false;
    let watcherId: number | undefined;
    let refreshTimer: number | undefined;
    const changedDirectories = new Set<string>();

    void startProjectFileWatcher(watchedRoot, (change) => {
      if (disposed) return;
      filesChanged(change.paths);
      for (const directory of changedParentDirectories(watchedRoot, change.paths)) {
        changedDirectories.add(directory);
      }
      window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => {
        const next = { ...directoryRevisions };
        for (const directory of changedDirectories) next[directory] = (next[directory] ?? 0) + 1;
        changedDirectories.clear();
        directoryRevisions = next;
      }, 160);
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
    const requestedRoot = projectRoot;
    void directoryRevisions[pathKey(requestedRoot)];
    void manualRevision;
    void loadRoot(requestedRoot);
  });

  async function loadRoot(requestedRoot: string) {
    const token = ++loadToken;
    loading = true;
    loadError = '';
    try {
      const nextEntries = await readProjectDirectory(requestedRoot, requestedRoot);
      if (token !== loadToken) return;
      entries = nextEntries;
    } catch (error) {
      if (token !== loadToken) return;
      loadError = errorMessage(error);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  function errorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
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
      aria-label="Refresh project files"
      title="Refresh project files"
      onclick={() => { manualRevision += 1; }}
    >
      <IconRefresh size={14} stroke={1.55} />
    </button>
    <button
      type="button"
      class="chrome-button project-explorer-toggle"
      class:active={visible}
      aria-label={visible ? 'Collapse project explorer' : 'Expand project explorer'}
      aria-pressed={visible}
      title={visible ? 'Collapse project explorer' : 'Expand project explorer'}
      onclick={toggle}
    >
      <IconLayoutSidebarRight size={14} stroke={1.45} />
    </button>
  </div>

  <div class="project-explorer-scroll">
    {#if loading && entries.length === 0}
      <p class="project-explorer-empty">Loading project…</p>
    {:else if loadError}
      <p class="project-explorer-empty error" title={loadError}>Could not read this project.</p>
    {:else if entries.length === 0}
      <p class="project-explorer-empty">This project is empty.</p>
    {:else}
      <ul class="project-file-tree" role="tree" aria-label={`${projectName} files`}>
        {#each entries as entry (entry.path)}
          <ProjectFileNode
            {entry}
            {projectRoot}
            depth={0}
            {directoryRevisions}
            {manualRevision}
            {activeFilePath}
            {openFile}
            reportError={(message) => { notice = message; }}
          />
        {/each}
      </ul>
    {/if}
  </div>

  {#if notice}
    <button class="project-explorer-notice" title={notice} onclick={() => { notice = ''; }}>
      <IconAlertCircle size={13} stroke={1.6} />
      <span>{notice}</span>
    </button>
  {/if}
</aside>
