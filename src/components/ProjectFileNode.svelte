<script lang="ts">
  import { IconChevronDown, IconChevronRight, IconLink } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import ProjectFileNode from './ProjectFileNode.svelte';
  import {
    openProjectFile,
    openProjectPath,
    readProjectDirectory,
    revealProjectPath,
    type ProjectFileEntry,
  } from '../services/native';
  import { copyText } from '../utils/clipboard';
  import { menuFromEvent, type ContextMenuState } from '../utils/context-menu';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';

  interface Props {
    entry: ProjectFileEntry;
    projectRoot: string;
    depth: number;
    revision: number;
    reportError: (message: string) => void;
  }

  const { entry, projectRoot, depth, revision, reportError }: Props = $props();
  let expanded = $state(false);
  let loaded = $state(false);
  let loading = $state(false);
  let children = $state<ProjectFileEntry[]>([]);
  let loadError = $state('');
  let loadedRevision = $state(-1);
  let loadToken = 0;
  let contextMenu = $state<ContextMenuState>();

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

  function openMenu(event: MouseEvent) {
    contextMenu = menuFromEvent(event, entry.isDirectory
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
        ]);
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
    void openProjectFile(projectRoot, entry.path).catch((error) => {
      reportError(`Could not open ${entry.name}: ${errorMessage(error)}`);
    });
  }

  function errorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<li role="treeitem" aria-expanded={entry.isDirectory ? expanded : undefined} aria-selected="false">
  <button
    type="button"
    class:folder={entry.isDirectory}
    class="project-file-row"
    style={`--file-depth: ${depth}`}
    title={entry.path}
    onclick={activate}
    oncontextmenu={openMenu}
  >
    <span class="project-file-chevron" aria-hidden="true">
      {#if entry.isDirectory}
        {#if expanded}<IconChevronDown size={13} stroke={1.65} />{:else}<IconChevronRight size={13} stroke={1.65} />{/if}
      {/if}
    </span>
    {#if icon}<img class="project-file-icon" src={icon} alt="" />{/if}
    <span class="project-file-name">{entry.name}</span>
    {#if entry.isSymlink}<IconLink class="project-file-link" size={11} stroke={1.6} />{/if}
  </button>

  {#if entry.isDirectory && expanded}
    <ul class="project-file-group" role="group">
      {#if loading && !loaded}
        <li class="project-file-state" style={`--file-depth: ${depth + 1}`}>Loading…</li>
      {:else if loadError}
        <li class="project-file-state error" style={`--file-depth: ${depth + 1}`} title={loadError}>
          Folder unavailable
        </li>
      {:else if children.length === 0}
        <li class="project-file-state" style={`--file-depth: ${depth + 1}`}>Empty</li>
      {:else}
        {#each children as child (child.path)}
          <ProjectFileNode
            entry={child}
            {projectRoot}
            depth={depth + 1}
            {revision}
            {reportError}
          />
        {/each}
      {/if}
    </ul>
  {/if}
</li>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}
