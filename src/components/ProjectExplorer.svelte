<script lang="ts">
  import { onMount } from 'svelte';
  import { Tween } from 'svelte/motion';
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
  import { shellLayoutDuration } from '../utils/shell-layout-motion';

  const macOS = ['darwin', 'macos'].includes(document.documentElement.dataset.platform ?? '');

  interface Props {
    open: boolean;
    visible: boolean;
    compactLayout: boolean;
    reducedMotion: boolean;
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
    open, visible, compactLayout, reducedMotion, projectRoot, projectName, activeFilePath, currentBranch, branches,
    toggle, openFile, filesChanged, startResize,
  }: Props = $props();

  const chromeButton =
    'grid size-[25px] shrink-0 place-items-center rounded-md border-0 bg-transparent p-0 text-muted hover:bg-panel-hover hover:text-text-soft';

  const drawerRightOffset = new Tween(0, { duration: 0 });
  const actionsRight = $derived(
    compactLayout && !open
      ? 'auto'
      : macOS ? '12px' : 'calc(var(--window-controls-width) + 12px)',
  );
  const actionsLeft = $derived(visible ? '6px' : compactLayout ? '3px' : 'auto');
  const actionsWidth = $derived(visible ? 'auto' : '25px');

  function compactClosedRight() {
    const expanded = parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue('--project-explorer-expanded-width'),
    ) || 280;
    const controls = parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue('--window-controls-width'),
    ) || 104;
    return (macOS ? 37 : 37 + controls) - expanded;
  }

  $effect(() => {
    if (!compactLayout) return;
    void drawerRightOffset.set(open ? 0 : compactClosedRight(), {
      duration: shellLayoutDuration(reducedMotion),
    });
  });

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
  const verticalTreeKeys = new Set(['ArrowDown', 'ArrowUp', 'Home', 'End']);

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
    if (verticalTreeKeys.has(event.key)) {
      event.preventDefault();
      focusTreeItem(verticalTreeItem(event.key, items, index));
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

  function verticalTreeItem(key: string, items: HTMLButtonElement[], index: number) {
    if (key === 'Home') return items[0];
    if (key === 'End') return items.at(-1);
    return items[index + (key === 'ArrowDown' ? 1 : -1)];
  }

  function setView(value: string) {
    if (value !== 'files' && value !== 'changes') return;
    view = value;
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<aside
  class:open
  class="project-explorer absolute top-0 right-0 bottom-0 z-[11] flex min-h-0 w-explorer min-w-0 flex-col overflow-visible border-l border-line bg-sidebar pt-titlebar opacity-100 compact:fixed compact:top-0 compact:bottom-0 compact:w-[var(--project-explorer-expanded-width)] compact:pt-titlebar compact:[&:not(.open)]:pointer-events-none compact:[&:not(.open)]:border-transparent compact:[&:not(.open)]:bg-transparent compact:[&:not(.open)]:shadow-none compact:[&.open]:bg-raised compact:[&.open]:shadow-[-22px_0_60px_rgba(0,0,0,0.4)] non-macos:shell-explorer-open:pt-[calc(var(--titlebar-height)+32px)]"
  style:right={compactLayout ? `${drawerRightOffset.current}px` : undefined}
  aria-label="Project explorer"
>
  <div
    class="sidebar-resize-handle right-sidebar-resize-handle absolute top-0 bottom-0 -left-1 z-[4] w-[7px] cursor-ew-resize touch-none after:absolute after:inset-y-0 after:left-[3px] after:w-px after:bg-transparent after:content-[''] hover:after:bg-line-strong shell-explorer-collapsed:hidden compact:hidden"
    role="separator"
    aria-label="Resize right sidebar"
    aria-orientation="vertical"
    onpointerdown={startResize}
  ></div>

  <div
    class="project-explorer-actions pointer-events-auto absolute top-1.5 z-[3] flex items-center justify-between"
    style:right={actionsRight}
    style:left={actionsLeft}
    style:width={actionsWidth}
    data-tauri-drag-region
  >
    <button
      type="button"
      class="{chromeButton} project-explorer-refresh {visible ? '' : 'hidden'}"
      aria-label={view === 'files' ? 'Refresh project files' : 'Refresh Git changes'}
      title={view === 'files' ? 'Refresh project files' : 'Refresh Git changes'}
      onclick={() => { revision += 1; }}
    >
      <IconRefresh size={14} stroke={1.55} />
    </button>
    <button
      type="button"
      class="{chromeButton} project-explorer-toggle [&.active]:bg-panel-hover [&.active]:text-text-soft"
      class:active={visible}
      aria-label={visible ? 'Collapse right sidebar' : 'Expand right sidebar'}
      aria-pressed={visible}
      title={visible ? 'Collapse right sidebar' : 'Expand right sidebar'}
      onclick={toggle}
    >
      <IconLayoutSidebarRight size={14} stroke={1.55} />
    </button>
  </div>

  <Tabs.Root class="flex min-h-0 w-full flex-1 flex-col" value={view} onValueChange={setView}>
    <Tabs.List
      class="project-explorer-tab-list absolute top-[7px] right-[38px] left-[38px] z-[4] flex h-[23px] items-center justify-center gap-0.5 shell-explorer-collapsed:invisible shell-explorer-collapsed:pointer-events-none shell-explorer-collapsed:opacity-0 non-macos:shell-explorer-open:top-[calc(var(--titlebar-height)+7px)] compact:[[class~='project-explorer']:not(.open)_&]:invisible compact:[[class~='project-explorer']:not(.open)_&]:pointer-events-none compact:[[class~='project-explorer']:not(.open)_&]:opacity-0 [&_button]:h-[23px] [&_button]:rounded-[5px] [&_button]:border-0 [&_button]:bg-transparent [&_button]:px-[7px] [&_button]:text-[11px] [&_button]:font-medium [&_button]:text-faint hover:[&_button]:bg-panel-hover hover:[&_button]:text-text-soft [&_button[data-state=active]]:bg-panel-active [&_button[data-state=active]]:text-text"
      aria-label="Right sidebar views"
    >
      <Tabs.Trigger value="files">Files</Tabs.Trigger>
      {#if currentBranch !== null}<Tabs.Trigger value="changes">Changes</Tabs.Trigger>{/if}
    </Tabs.List>
    <Tabs.Content
      class="project-explorer-scroll min-h-0 w-full flex-1 overflow-auto p-[5px_5px_10px] shell-explorer-collapsed:invisible shell-explorer-collapsed:pointer-events-none shell-explorer-collapsed:opacity-0 compact:[[class~='project-explorer']:not(.open)_&]:invisible compact:[[class~='project-explorer']:not(.open)_&]:pointer-events-none compact:[[class~='project-explorer']:not(.open)_&]:opacity-0"
      value="files"
    >
      {#if loading && entries.length === 0}
        <p class="m-0 p-2.5 text-[11px] leading-[1.45] text-faint">Loading project…</p>
      {:else if loadError}
        <p class="m-0 p-2.5 text-[11px] leading-[1.45] text-danger" title={loadError}>Could not read this project.</p>
      {:else if entries.length === 0}
        <p class="m-0 p-2.5 text-[11px] leading-[1.45] text-faint">This project is empty.</p>
      {:else}
        <ul
          bind:this={tree}
          class="project-file-tree m-0 list-none p-0"
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
      <Tabs.Content
        class="project-explorer-scroll project-changes-scroll min-h-0 w-full flex-1 overflow-auto py-[5px] pr-0 pl-0 shell-explorer-collapsed:invisible shell-explorer-collapsed:pointer-events-none shell-explorer-collapsed:opacity-0"
        value="changes"
      >
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
    <button
      class="project-explorer-notice mx-1.5 mb-[7px] flex min-h-[31px] w-[calc(100%-12px)] shrink-0 items-center gap-[7px] overflow-hidden rounded-[7px] border border-[color-mix(in_srgb,var(--danger)_22%,transparent)] bg-[color-mix(in_srgb,var(--danger)_7%,transparent)] px-2 py-1.5 text-left text-[color-mix(in_srgb,var(--danger)_86%,var(--text-soft))] shell-explorer-collapsed:invisible shell-explorer-collapsed:pointer-events-none shell-explorer-collapsed:opacity-0"
      title={notice}
      onclick={() => { notice = ''; }}
    >
      <IconAlertCircle size={13} stroke={1.55} />
      <span class="overflow-hidden text-[11px] text-ellipsis whitespace-nowrap">{notice}</span>
    </button>
  {/if}
</aside>
