<script lang="ts">
  import { IconChevronDown, IconChevronRight, IconLink } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import ProjectFileNode from './ProjectFileNode.svelte';
  import {
    openProjectPath,
    readProjectDirectory,
    revealProjectPath,
    type ProjectFileEntry,
  } from '../services/native';
  import { copyText } from '../utils/clipboard';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';

  interface Props {
    entry: ProjectFileEntry;
    projectRoot: string;
    depth: number;
    revision: number;
    activeFilePath: string | null;
    focusedPath: string | null;
    setFocusedPath: (path: string) => void;
    openFile: (path: string) => void;
    reportError: (message: string) => void;
  }

  const {
    entry, projectRoot, depth, revision, activeFilePath, focusedPath, setFocusedPath, openFile, reportError,
  }: Props = $props();
  let expanded = $state(false);
  let loaded = $state(false);
  let loading = $state(false);
  let children = $state<ProjectFileEntry[]>([]);
  let loadError = $state('');
  let loadedRevision = $state(-1);
  let loadToken = 0;

  const icon = $derived(
    entry.isDirectory
      ? materialFolderIcon(entry.name, expanded)
      : materialFileIcon(entry.name),
  );

  $effect(() => {
    if (expanded && loaded && revision !== loadedRevision) void loadChildren();
  });

  async function loadChildren() {
    const token = ++loadToken;
    const requestedRevision = revision;
    loading = true;
    loadError = '';
    try {
      const nextChildren = await readProjectDirectory(projectRoot, entry.path);
      if (token !== loadToken) return;
      children = nextChildren;
      loaded = true;
      loadedRevision = requestedRevision;
    } catch (error) {
      if (token !== loadToken) return;
      loadError = errorMessage(error);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  function runPathAction(action: Promise<void>) {
    void action.catch((error) => reportError(errorMessage(error)));
  }

  function activate() {
    if (entry.isDirectory) {
      expanded = !expanded;
      if (expanded && !loaded) void loadChildren();
      return;
    }
    openFile(entry.path);
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<li role="none">
  <ContextMenu
    items={entry.isDirectory
      ? [
          { label: expanded ? 'Collapse' : 'Expand', action: activate },
          { label: 'Open folder', action: () => runPathAction(openProjectPath(projectRoot, entry.path)) },
          { label: 'Reveal folder', action: () => runPathAction(revealProjectPath(projectRoot, entry.path)) },
          { label: 'Copy absolute path', action: () => copyText(entry.path) },
          { label: 'Refresh', separatorBefore: true, action: () => { expanded = true; void loadChildren(); } },
        ]
      : [
          { label: 'Open', action: activate },
          { label: 'Reveal in file manager', action: () => runPathAction(revealProjectPath(projectRoot, entry.path)) },
          { label: 'Copy absolute path', action: () => copyText(entry.path) },
        ]}
  >
    {#snippet children({ props })}
      <button
        {...props}
        type="button"
        role="treeitem"
        aria-expanded={entry.isDirectory ? expanded : undefined}
        aria-selected={!entry.isDirectory && activeFilePath === entry.path}
        data-path={entry.path}
        tabindex={focusedPath === entry.path ? 0 : -1}
        class={`flex h-6 w-full min-w-0 items-center gap-1 rounded px-1.5 text-left text-[12px] text-muted hover:bg-panel-hover hover:text-ink-soft aria-selected:bg-panel-active ${!entry.isDirectory && activeFilePath === entry.path ? 'bg-panel-active text-ink-soft' : ''}`}
        style={`padding-left: ${depth * 12 + 6}px`}
        title={entry.path}
        onclick={activate}
        onfocus={() => setFocusedPath(entry.path)}
      >
        <span class="grid size-3 shrink-0 place-items-center text-faint" aria-hidden="true">
          {#if entry.isDirectory}
            {#if expanded}<IconChevronDown size={13} stroke={1.55} />{:else}<IconChevronRight size={13} stroke={1.55} />{/if}
          {/if}
        </span>
        {#if icon}<img class="size-3 shrink-0" src={icon} alt="" />{/if}
        <span class="min-w-0 truncate">{entry.name}</span>
        {#if entry.isSymlink}<IconLink class="ml-auto shrink-0 text-faint" size={11} stroke={1.55} />{/if}
      </button>
    {/snippet}
  </ContextMenu>

  {#if entry.isDirectory && expanded}
    <ul class="m-0 list-none p-0" role="group">
      {#if loading && !loaded}
        <li class="h-6 text-[11px] leading-6 text-faint" style={`padding-left: ${(depth + 1) * 12 + 6}px`}>Loading…</li>
      {:else if loadError}
        <li class="h-6 text-[11px] leading-6 text-danger" style={`padding-left: ${(depth + 1) * 12 + 6}px`} title={loadError}>
          Folder unavailable
        </li>
      {:else if children.length === 0}
        <li class="h-6 text-[11px] leading-6 text-faint" style={`padding-left: ${(depth + 1) * 12 + 6}px`}>Empty</li>
      {:else}
        {#each children as child (child.path)}
          <ProjectFileNode
            entry={child}
            {projectRoot}
            depth={depth + 1}
            {revision}
            {activeFilePath}
            {focusedPath}
            {setFocusedPath}
            {openFile}
            {reportError}
          />
        {/each}
      {/if}
    </ul>
  {/if}
</li>
