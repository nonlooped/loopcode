<script lang="ts">
  import { onMount } from 'svelte';
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
    toggle: () => void;
    startResize: (event: PointerEvent) => void;
  }

  const { open, visible, projectRoot, projectName, toggle, startResize }: Props = $props();
  let entries = $state<ProjectFileEntry[]>([]);
  let loading = $state(true);
  let loadError = $state('');
  let notice = $state('');
  let revision = $state(0);
  let refreshTimer: number | undefined;
  let loadToken = 0;

  onMount(() => {
    let disposed = false;
    let watcherId: number | undefined;

    void startProjectFileWatcher(projectRoot, () => {
      if (disposed) return;
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
      onclick={() => { revision += 1; }}
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
            {revision}
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
