<script lang="ts">
  import { flip } from 'svelte/animate';
  import { fade } from 'svelte/transition';
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

  import { profileById } from '../config/providers';
  import type { ProjectState, ThreadState } from '../types';
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
    selectThread: (threadId: string) => void;
    selectThreadFromKeyboard: (event: KeyboardEvent, threadId: string) => void;
    toggleSettled: (threadId: string) => void;
    removeThread: (threadId: string) => void;
    setShowSettled: (show: boolean) => void;
    openSettings: () => void;
    closeSettings: () => void;
  }

  const props: Props = $props();
</script>

<aside class:open={props.open} class="thread-sidebar">
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
        class="workspace-identity"
        aria-haspopup="menu"
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
        onclick={() => props.setWorkspaceDropdownOpen(false)}
      ></button>
      <div class="workspace-dropdown" role="menu">
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
            <div class="empty-hint">No sessions yet</div>
          {:else}
            {#each props.inboxThreads as thread (thread.id)}
              {@const threadProfile = profileById(thread.profileId)}
              {@const status = threadStatus(thread)}
              <div class="thread-motion" animate:flip={{ duration: props.compactMotion ? 0 : 260 }}>
                <div
                  class:active={thread.id === props.selectedThreadId}
                  class="thread-item"
                  role="button"
                  tabindex="0"
                  onclick={() => props.selectThread(thread.id)}
                  onkeydown={(event) => props.selectThreadFromKeyboard(event, thread.id)}
                  transition:fade={{ duration: props.compactMotion ? 0 : 150 }}
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
                        <span>@ local</span>
                      </span>
                      <span class:live={status === 'running'} class:error={status === 'error'} class="thread-updated">
                        {status === 'running' ? 'Working' : status === 'error' ? 'Failed' : relativeTime(thread.updatedAt)}
                      </span>
                    </small>
                    <strong>{thread.title}</strong>
                    <span class="thread-details">
                      <img src={threadProfile.icon} alt="" />
                      <span>{threadHarness(thread)}</span>
                    </span>
                  </span>
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
                      aria-label={`Remove ${thread.title}`}
                      title="Remove thread"
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
              <div class="empty-hint">No archived sessions</div>
            {:else}
              {#each props.settledThreads as thread (thread.id)}
                {@const threadProfile = profileById(thread.profileId)}
                <div class="thread-motion" animate:flip={{ duration: props.compactMotion ? 0 : 260 }}>
                  <div
                    class:active={thread.id === props.selectedThreadId}
                    class="thread-item slim settled"
                    role="button"
                    tabindex="0"
                    onclick={() => props.selectThread(thread.id)}
                    onkeydown={(event) => props.selectThreadFromKeyboard(event, thread.id)}
                    title={thread.title}
                    transition:fade={{ duration: props.compactMotion ? 0 : 150 }}
                  >
                    <span class="slim-copy"><img src={threadProfile.icon} alt="" /><strong>{thread.title}</strong></span>
                    <span class="slim-updated">{relativeTime(thread.updatedAt)}</span>
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
                        aria-label={`Remove ${thread.title}`}
                        title="Remove thread"
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

    <button class="sidebar-foot" onclick={props.openSettings} aria-label="Open settings">
      <span class="local-avatar">L</span>
      <span class="sidebar-foot-copy"><strong>LoopCode</strong><small>Local only</small></span>
      <IconSettings class="sidebar-foot-settings" size={15} stroke={1.55} />
    </button>
  {/if}
</aside>
