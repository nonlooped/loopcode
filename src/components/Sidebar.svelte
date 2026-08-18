<script lang="ts">
  import { tick } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fade, fly } from 'svelte/transition';
  import {
    IconArchive,
    IconArrowLeft,
    IconCheck,
    IconChevronDown,
    IconChevronRight,
    IconFolder,
    IconFolderPlus,
    IconInbox,
    IconPlus,
    IconSettings,
    IconTrash,
  } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import { profileById } from '../config/providers';
  import type { ProjectState, ThreadState } from '../types';
  import { copyText } from '../utils/clipboard';
  import { menuFromEvent, nextMenuItemIndex, type ContextMenuState } from '../utils/context-menu';
  import { folderName, relativeTime, threadHarness, threadStatus } from '../utils/threads';

  interface Props {
    open: boolean;
    settingsOpen: boolean;
    compactMotion: boolean;
    defaultWorkingFolder: string;
    projects: ProjectState[];
    selectedProjectId: string | null;
    activeProject: ProjectState | null;
    workspaceDropdownOpen: boolean;
    inboxThreads: ThreadState[];
    settledThreads: ThreadState[];
    selectedThreadId: string;
    showSettled: boolean;
    setWorkspaceDropdownOpen: (open: boolean) => void;
    selectProject: (projectId: string | null) => void;
    addProject: () => void;
    addThread: () => void;
    addThreadToProject: (projectId: string) => void;
    selectThread: (threadId: string) => void;
    toggleSettled: (threadId: string) => void;
    renameThread: (threadId: string) => void;
    openThreadFolder: (thread: ThreadState) => void;
    removeThread: (threadId: string) => void;
    openProjectFolder: (project: ProjectState) => void;
    revealProjectFolder: (project: ProjectState) => void;
    removeProject: (projectId: string) => void;
    setShowSettled: (show: boolean) => void;
    openSettings: () => void;
    closeSettings: () => void;
    startResize: (event: PointerEvent) => void;
  }

  const props: Props = $props();
  let contextMenu = $state<ContextMenuState>();
  let workspaceTrigger = $state<HTMLButtonElement>();
  let workspaceMenu = $state<HTMLElement>();

  $effect(() => {
    if (!props.workspaceDropdownOpen) return;
    void tick().then(() => {
      workspaceMenu?.querySelector<HTMLButtonElement>('.workspace-option.active, button:not(:disabled)')?.focus();
    });
  });

  function closeWorkspaceMenu(returnFocus = false) {
    props.setWorkspaceDropdownOpen(false);
    if (returnFocus) void tick().then(() => workspaceTrigger?.focus());
  }

  function handleWorkspaceMenuKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeWorkspaceMenu(true);
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key) || !workspaceMenu) return;
    event.preventDefault();
    const items = Array.from(workspaceMenu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    const next = nextMenuItemIndex(
      items.indexOf(document.activeElement as HTMLButtonElement),
      items.length,
      event.key as 'ArrowDown' | 'ArrowUp' | 'Home' | 'End',
    );
    items[next]?.focus();
  }

  function openThreadMenu(event: MouseEvent, thread: ThreadState) {
    contextMenu = menuFromEvent(event, [
      { label: 'Rename', action: () => props.renameThread(thread.id) },
      { label: thread.settled ? 'Unarchive' : 'Archive', action: () => props.toggleSettled(thread.id) },
      { label: 'Open project folder', action: () => props.openThreadFolder(thread), disabled: !thread.cwd },
      { label: 'Delete thread', action: () => props.removeThread(thread.id), danger: true, separatorBefore: true },
    ]);
  }

  function openProjectMenu(event: MouseEvent, project: ProjectState) {
    contextMenu = menuFromEvent(event, [
      { label: 'New thread in project', action: () => props.addThreadToProject(project.id) },
      { label: 'Open folder', action: () => props.openProjectFolder(project) },
      { label: 'Reveal folder', action: () => props.revealProjectFolder(project) },
      { label: 'Copy path', action: () => copyText(project.path) },
      { label: 'Remove from LoopCode', action: () => props.removeProject(project.id), danger: true, separatorBefore: true },
    ]);
  }
</script>

<aside class:open={props.open} class="thread-sidebar">
  <div
    class="sidebar-resize-handle left-sidebar-resize-handle"
    role="separator"
    aria-label="Resize left sidebar"
    aria-orientation="vertical"
    onpointerdown={props.startResize}
  ></div>
  {#if props.settingsOpen}
    <div class="settings-nav">
      <div class="settings-nav-label">Settings</div>
      <button class="settings-nav-item active" aria-current="page">
        <IconSettings size={16} stroke={1.55} />
        <span>General</span>
      </button>
    </div>
    <div class="settings-nav-spacer"></div>
    <div class="settings-nav-footer">
      <button class="settings-back" onclick={props.closeSettings}>
        <IconArrowLeft size={16} stroke={1.55} />
        <span>Back</span>
      </button>
    </div>
  {:else}
    <div class="sidebar-heading">
      <button
        bind:this={workspaceTrigger}
        class="workspace-identity"
        aria-haspopup="menu"
        aria-controls="workspace-menu"
        aria-expanded={props.workspaceDropdownOpen}
        onclick={() => props.setWorkspaceDropdownOpen(!props.workspaceDropdownOpen)}
        title="Choose folder"
      >
        <IconFolder size={15} stroke={1.6} />
        <strong>{props.activeProject ? props.activeProject.name : 'All projects'}</strong>
        <IconChevronDown class="workspace-chevron" size={12} stroke={1.55} />
      </button>
      <div class="heading-actions">
        <button class="icon-button" aria-label="New thread" title="New thread" onclick={props.addThread}>
          <IconPlus size={14} stroke={1.7} />
        </button>
      </div>
    </div>
    {#if props.workspaceDropdownOpen}
      <button
        class="dropdown-backdrop"
        tabindex="-1"
        aria-label="Close folder menu"
        onclick={() => closeWorkspaceMenu()}
      ></button>
      <div
        bind:this={workspaceMenu}
        id="workspace-menu"
        class="workspace-dropdown"
        role="menu"
        tabindex="-1"
        onkeydown={handleWorkspaceMenuKeydown}
        transition:fly={{ y: props.compactMotion ? 0 : -4, duration: props.compactMotion ? 0 : 130 }}
      >
        <button
          class="workspace-option"
          class:active={props.selectedProjectId === null}
          role="menuitem"
          onclick={() => props.selectProject(null)}
        >
          <IconFolder size={13} stroke={1.6} />
          <span class="workspace-option-main">
            <strong>All projects</strong>
            <small title={props.defaultWorkingFolder}>{props.defaultWorkingFolder || 'local'} · default</small>
          </span>
          {#if props.selectedProjectId === null}<IconCheck size={13} stroke={2} />{/if}
        </button>
        {#each props.projects as project (project.id)}
          <button
            class="workspace-option"
            class:active={props.selectedProjectId === project.id}
            role="menuitem"
            onclick={() => props.selectProject(project.id)}
            oncontextmenu={(event) => openProjectMenu(event, project)}
          >
            <IconFolder size={13} stroke={1.6} />
            <span class="workspace-option-main">
              <strong>{project.name}</strong>
              <small title={project.path}>{project.path}</small>
            </span>
            {#if props.selectedProjectId === project.id}<IconCheck size={13} stroke={2} />{/if}
          </button>
        {/each}
        <div class="workspace-dropdown-foot">
          <button class="workspace-add-folder" onclick={props.addProject}>
            <IconFolderPlus size={13} stroke={1.7} /> Add folder…
          </button>
        </div>
      </div>
    {/if}

    <div class="sidebar-scroll">
      <section class="sidebar-section inbox-section">
        <nav class="thread-list" aria-label="Inbox threads">
          {#if props.inboxThreads.length === 0}
            <div class="empty-hint">No threads yet</div>
          {:else}
            {#each props.inboxThreads as thread (thread.id)}
              {@const threadProfile = profileById(thread.profileId)}
              {@const status = threadStatus(thread)}
              <div class="thread-motion" animate:flip={{ duration: props.compactMotion ? 0 : 260 }}>
                <div
                  class:active={thread.id === props.selectedThreadId}
                  class="thread-item"
                  role="group"
                  aria-label={thread.title}
                  oncontextmenu={(event) => openThreadMenu(event, thread)}
                  transition:fade={{ duration: props.compactMotion ? 0 : 150 }}
                >
                  <button
                    type="button"
                    class="thread-select"
                    aria-current={thread.id === props.selectedThreadId ? 'page' : undefined}
                    onclick={() => props.selectThread(thread.id)}
                  >
                  <span class="thread-copy">
                    <small class="thread-topline">
                      <span class="thread-location">
                        {#if thread.projectId}
                          {@const project = props.projects.find((item) => item.id === thread.projectId)}
                          {project ? project.name : folderName(thread.cwd)}
                        {:else}
                          {thread.cwd ? folderName(thread.cwd) : '~'}
                        {/if}
                      </span>
                      <span class="thread-status-motion">
                        {#key status}
                          <span
                            class:live={status === 'running'}
                            class:error={status === 'error'}
                            class="thread-updated"
                            transition:fade={{ duration: props.compactMotion ? 0 : 120 }}
                          >
                            {status === 'running' ? 'Working' : status === 'error' ? 'Failed' : relativeTime(thread.updatedAt)}
                          </span>
                        {/key}
                      </span>
                    </small>
                    <span class="thread-title-motion">
                      {#key thread.title}
                        <strong transition:fade={{ duration: props.compactMotion ? 0 : 120 }}>{thread.title}</strong>
                      {/key}
                    </span>
                    <span class="thread-details">
                      <img src={threadProfile.icon} alt="" />
                      <span>{threadHarness(thread)}</span>
                    </span>
                  </span>
                  </button>
                  <span class="thread-actions">
                    <button
                      type="button"
                      class="action-settle"
                      aria-label={`Archive ${thread.title}`}
                      title="Archive"
                      onclick={(event) => { event.stopPropagation(); props.toggleSettled(thread.id); }}
                    ><IconArchive size={13} stroke={1.7} /></button>
                    <button
                      type="button"
                      class="remove-thread"
                      aria-label={`Delete ${thread.title}`}
                      title="Delete thread"
                      onclick={(event) => { event.stopPropagation(); props.removeThread(thread.id); }}
                    ><IconTrash size={13} stroke={1.7} /></button>
                  </span>
                </div>
              </div>
            {/each}
          {/if}
        </nav>
      </section>

      <section class="sidebar-section settled-section">
        <button
          class="section-header"
          onclick={() => props.setShowSettled(!props.showSettled)}
          aria-expanded={props.showSettled}
        >
          <span class="section-title">Archived{#if !props.showSettled} ({props.settledThreads.length}){/if}</span>
          <span class="section-rule"></span>
          {#if props.showSettled}<IconChevronDown size={11} stroke={1.7} />{:else}<IconChevronRight size={11} stroke={1.7} />{/if}
        </button>
        {#if props.showSettled}
          <nav class="thread-list settled" aria-label="Archived threads">
            {#if props.settledThreads.length === 0}
              <div class="empty-hint">No archived threads</div>
            {:else}
              {#each props.settledThreads as thread (thread.id)}
                {@const threadProfile = profileById(thread.profileId)}
                <div class="thread-motion" animate:flip={{ duration: props.compactMotion ? 0 : 260 }}>
                  <div
                    class:active={thread.id === props.selectedThreadId}
                    class="thread-item slim settled"
                    role="group"
                    aria-label={thread.title}
                    oncontextmenu={(event) => openThreadMenu(event, thread)}
                    title={thread.title}
                    transition:fade={{ duration: props.compactMotion ? 0 : 150 }}
                  >
                    <button
                      type="button"
                      class="thread-select"
                      aria-current={thread.id === props.selectedThreadId ? 'page' : undefined}
                      onclick={() => props.selectThread(thread.id)}
                    >
                    <span class="slim-copy">
                      <img src={threadProfile.icon} alt="" />
                      <span class="thread-title-motion">
                        {#key thread.title}
                          <strong transition:fade={{ duration: props.compactMotion ? 0 : 120 }}>{thread.title}</strong>
                        {/key}
                      </span>
                    </span>
                    <span class="slim-updated">{relativeTime(thread.updatedAt)}</span>
                    </button>
                    <span class="thread-actions">
                      <button
                        type="button"
                        class="action-unsettle"
                        aria-label={`Unarchive ${thread.title}`}
                        title="Unarchive"
                        onclick={(event) => { event.stopPropagation(); props.toggleSettled(thread.id); }}
                      ><IconInbox size={12} stroke={1.7} /></button>
                      <button
                        type="button"
                        class="remove-thread"
                        aria-label={`Delete ${thread.title}`}
                        title="Delete thread"
                        onclick={(event) => { event.stopPropagation(); props.removeThread(thread.id); }}
                      ><IconTrash size={12} stroke={1.7} /></button>
                    </span>
                  </div>
                </div>
              {/each}
            {/if}
          </nav>
        {/if}
      </section>
    </div>

    <button class="sidebar-foot" onclick={props.openSettings}>
      <IconSettings size={16} stroke={1.55} />
      <span>Settings</span>
    </button>
  {/if}
</aside>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}

<svelte:window
  onblur={() => { if (props.workspaceDropdownOpen) closeWorkspaceMenu(); }}
  onkeydown={(event) => { if (event.key === 'Escape' && props.workspaceDropdownOpen) closeWorkspaceMenu(true); }}
/>
