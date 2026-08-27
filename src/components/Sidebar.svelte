<script lang="ts">
  import { flip } from 'svelte/animate';
  import { fade } from 'svelte/transition';
  import { AlertDialog, Collapsible, DropdownMenu } from 'bits-ui';
  import {
    IconArchive,
    IconArrowLeft,
    IconCheck,
    IconChevronDown,
    IconChevronRight,
    IconContrast,
    IconFolder,
    IconFolderPlus,
    IconInbox,
    IconInfoCircle,
    IconMessageCircle,
    IconPlus,
    IconPlugConnected,
    IconRobot,
    IconSettings,
    IconTerminal2,
    IconTrash,
    IconWriting,
  } from '@tabler/icons-svelte';

  import appIcon from '../../assets/loopcode-mark.png';
  import ContextMenu, { type ContextMenuItem } from './ContextMenu.svelte';
  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import type { HarnessProfile, PermissionRequest, ProjectState, ThreadState } from '../types';
  import type { SettingsCategory } from '../utils/app-settings';
  import { copyText } from '../utils/clipboard';
  import { folderName, relativeTime, threadAttention, threadHarness } from '../utils/threads';

  interface Props {
    open: boolean;
    settingsOpen: boolean;
    settingsCategory: SettingsCategory;
    profiles: HarnessProfile[];
    compactMotion: boolean;
    defaultWorkingFolder: string;
    projects: ProjectState[];
    selectedProjectId: string | null;
    activeProject: ProjectState | null;
    inboxThreads: ThreadState[];
    interactionRequestsByThread: Record<string, PermissionRequest>;
    settledThreads: ThreadState[];
    selectedThreadId: string;
    showSettled: boolean;
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
    clearArchivedThreads: () => void;
    openSettings: () => void;
    closeSettings: () => void;
    setSettingsCategory: (category: SettingsCategory) => void;
    startResize: (event: PointerEvent) => void;
  }

  const props: Props = $props();
  const settingsCategories = [
    { category: 'general', label: 'General', icon: IconSettings },
    { category: 'appearance', label: 'Appearance', icon: IconContrast },
    { category: 'conversation', label: 'Conversation', icon: IconMessageCircle },
    { category: 'composer', label: 'Composer', icon: IconWriting },
    { category: 'agents', label: 'Agents and permissions', icon: IconRobot },
    { category: 'providers', label: 'Providers', icon: IconPlugConnected },
    { category: 'terminal', label: 'Terminal', icon: IconTerminal2 },
    { category: 'about', label: 'About', icon: IconInfoCircle },
  ] satisfies { category: SettingsCategory; label: string; icon: typeof IconSettings }[];
  let archiveDeletionPending = $state(false);
  const inboxGroups = $derived.by(() => {
    const groups: { kind: 'needs-you' | 'working' | 'recent'; label: string; threads: ThreadState[] }[] = [
      { kind: 'needs-you', label: 'Needs you', threads: [] },
      { kind: 'working', label: 'Working', threads: [] },
      { kind: 'recent', label: 'Recent', threads: [] },
    ];
    for (const thread of props.inboxThreads) {
      const attention = threadAttention(thread, props.interactionRequestsByThread[thread.id]);
      const group = attention.kind === 'working'
        ? groups[1]
        : attention.kind === 'recent'
          ? groups[2]
          : groups[0];
      group.threads.push(thread);
    }
    return groups.filter((group) => group.threads.length > 0);
  });

  function profileById(profileId: string) {
    return props.profiles.find((profile) => profile.id === profileId) ?? officialProfileById(profileId) ?? officialProfiles[0];
  }

  function threadMenuItems(thread: ThreadState): ContextMenuItem[] {
    return [
      { label: 'Rename', action: () => props.renameThread(thread.id) },
      { label: thread.settled ? 'Unarchive' : 'Archive', action: () => props.toggleSettled(thread.id) },
      { label: 'Open project folder', action: () => props.openThreadFolder(thread), disabled: !thread.cwd },
      { label: 'Delete thread', action: () => props.removeThread(thread.id), danger: true, separatorBefore: true },
    ];
  }

  function projectMenuItems(project: ProjectState): ContextMenuItem[] {
    return [
      { label: 'New thread in project', action: () => props.addThreadToProject(project.id) },
      { label: 'Open folder', action: () => props.openProjectFolder(project) },
      { label: 'Reveal folder', action: () => props.revealProjectFolder(project) },
      { label: 'Copy path', action: () => copyText(project.path) },
      { label: 'Remove from LoopCode', action: () => props.removeProject(project.id), danger: true, separatorBefore: true },
    ];
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
      <div class="settings-nav-label">
        <img class="settings-nav-logo" src={appIcon} alt="" aria-hidden="true" />
        <span>Settings</span>
      </div>
      <nav aria-label="Settings categories">
        {#each settingsCategories as { category, label, icon: Icon } (category)}
          <button
            class:active={props.settingsCategory === category}
            class="settings-nav-item"
            aria-current={props.settingsCategory === category ? 'page' : undefined}
            onclick={() => props.setSettingsCategory(category)}
          >
            <Icon size={16} stroke={1.55} />
            <span>{label}</span>
          </button>
        {/each}
      </nav>
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
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="workspace-identity" title="Choose folder">
          <IconFolder size={15} stroke={1.55} />
          <strong>{props.activeProject ? props.activeProject.name : 'All projects'}</strong>
          <IconChevronDown class="workspace-chevron" size={12} stroke={1.55} />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            class="workspace-dropdown"
            side="bottom"
            align="start"
            sideOffset={4}
            collisionPadding={8}
            aria-label="Choose folder"
          >
            <DropdownMenu.RadioGroup
              value={props.selectedProjectId ?? '__all-projects__'}
              onValueChange={(value) => props.selectProject(value === '__all-projects__' ? null : value)}
            >
              <DropdownMenu.RadioItem class="workspace-option" value="__all-projects__">
                {#snippet children({ checked })}
                  <IconFolder size={13} stroke={1.55} />
                  <span class="workspace-option-main">
                    <strong>All projects</strong>
                    <small title={props.defaultWorkingFolder}>{props.defaultWorkingFolder || 'local'} · default</small>
                  </span>
                  {#if checked}<IconCheck size={13} stroke={2} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
              {#each props.projects as project (project.id)}
                <ContextMenu items={projectMenuItems(project)}>
                  {#snippet children({ props: triggerProps })}
                    <DropdownMenu.RadioItem {...triggerProps} class="workspace-option" value={project.id}>
                      {#snippet children({ checked })}
                        <IconFolder size={13} stroke={1.55} />
                        <span class="workspace-option-main">
                          <strong>{project.name}</strong>
                          <small title={project.path}>{project.path}</small>
                        </span>
                        {#if checked}<IconCheck size={13} stroke={2} />{/if}
                      {/snippet}
                    </DropdownMenu.RadioItem>
                  {/snippet}
                </ContextMenu>
              {/each}
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator class="workspace-dropdown-separator" />
            <DropdownMenu.Item class="workspace-add-folder" onSelect={props.addProject}>
              <IconFolderPlus size={13} stroke={1.55} /> Add folder…
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <div class="heading-actions">
        <button class="icon-button" aria-label="New thread" title="New thread" onclick={props.addThread}>
          <IconPlus size={14} stroke={1.55} />
        </button>
      </div>
    </div>
    <div class="sidebar-scroll">
      <section class="sidebar-section inbox-section">
        {#if props.inboxThreads.length === 0}
          <div class="empty-hint">No threads yet</div>
        {:else}
          {#each inboxGroups as group (group.kind)}
            <div class="thread-group">
              <div class="thread-group-label">{group.label} · {group.threads.length}</div>
              <nav class="thread-list" aria-label={`${group.label} threads`}>
                {#each group.threads as thread (thread.id)}
                  {@const threadProfile = profileById(thread.profileId)}
                  {@const attention = threadAttention(thread, props.interactionRequestsByThread[thread.id])}
                  <div class="thread-motion" animate:flip={{ duration: props.compactMotion ? 0 : 260 }}>
                    <ContextMenu items={threadMenuItems(thread)}>
                      {#snippet children({ props: triggerProps })}
                        <div
                          {...triggerProps}
                          class:active={thread.id === props.selectedThreadId}
                          class="thread-item"
                          role="group"
                          aria-label={thread.title}
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
                            {#key attention.kind}
                              <span
                                class:attention={attention.kind === 'needs-approval' || attention.kind === 'needs-answer'}
                                class:live={attention.kind === 'working'}
                                class:error={attention.kind === 'failed'}
                                class="thread-updated"
                                transition:fade={{ duration: props.compactMotion ? 0 : 120 }}
                              >
                                {attention.kind === 'recent' ? relativeTime(thread.updatedAt) : attention.label}
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
                          <img class:brand-color-icon={threadProfile.iconMode === 'brand'} src={threadProfile.icon} alt="" />
                          <span class="thread-provider">{threadHarness(thread, props.profiles)}</span>
                          {#if attention.kind !== 'recent'}
                            <span class="thread-reason">· {attention.reason}</span>
                          {/if}
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
                          ><IconArchive size={13} stroke={1.55} /></button>
                          <button
                            type="button"
                            class="remove-thread"
                            aria-label={`Delete ${thread.title}`}
                            title="Delete thread"
                            onclick={(event) => { event.stopPropagation(); props.removeThread(thread.id); }}
                          ><IconTrash size={13} stroke={1.55} /></button>
                        </span>
                        </div>
                      {/snippet}
                    </ContextMenu>
                  </div>
                {/each}
              </nav>
            </div>
          {/each}
        {/if}
      </section>

      {#if props.settledThreads.length > 0}
      <Collapsible.Root
        class="sidebar-section settled-section"
        open={props.showSettled}
        onOpenChange={props.setShowSettled}
      >
        <Collapsible.Trigger class="section-header">
          <span class="section-title">Archived{props.showSettled ? '' : ` (${props.settledThreads.length})`}</span>
          <span class="section-rule"></span>
          <IconChevronRight class="section-chevron" size={11} stroke={1.55} />
        </Collapsible.Trigger>
        <Collapsible.Content>
          <nav class="thread-list settled" aria-label="Archived threads">
              {#each props.settledThreads as thread (thread.id)}
                {@const threadProfile = profileById(thread.profileId)}
                <div class="thread-motion" animate:flip={{ duration: props.compactMotion ? 0 : 260 }}>
                  <ContextMenu items={threadMenuItems(thread)}>
                    {#snippet children({ props: triggerProps })}
                      <div
                        {...triggerProps}
                        class:active={thread.id === props.selectedThreadId}
                        class="thread-item slim settled"
                        role="group"
                        aria-label={thread.title}
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
                      <img class:brand-color-icon={threadProfile.iconMode === 'brand'} src={threadProfile.icon} alt="" />
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
                      ><IconInbox size={12} stroke={1.55} /></button>
                      <button
                        type="button"
                        class="remove-thread"
                        aria-label={`Delete ${thread.title}`}
                        title="Delete thread"
                        onclick={(event) => { event.stopPropagation(); props.removeThread(thread.id); }}
                      ><IconTrash size={12} stroke={1.55} /></button>
                    </span>
                  </div>
                {/snippet}
              </ContextMenu>
              </div>
              {/each}
          </nav>
          <button class="clear-archived" onclick={() => { archiveDeletionPending = true; }}>
            <IconTrash size={12} stroke={1.55} /> Delete all archived
          </button>
        </Collapsible.Content>
      </Collapsible.Root>
      {/if}
    </div>

    <button class="sidebar-foot" onclick={props.openSettings}>
      <IconSettings size={16} stroke={1.55} />
      <span>Settings</span>
    </button>
  {/if}
</aside>

<AlertDialog.Root
  open={archiveDeletionPending}
  onOpenChange={(open) => { archiveDeletionPending = open; }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="modal-overlay" />
    <AlertDialog.Content class="permission-modal confirmation-modal">
      <AlertDialog.Title>Delete archived threads?</AlertDialog.Title>
      <AlertDialog.Description>
        {props.settledThreads.length} archived {props.settledThreads.length === 1 ? 'thread' : 'threads'} and their history will be permanently deleted.
      </AlertDialog.Description>
      <footer class="confirmation-actions">
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button class="danger" onclick={() => {
          archiveDeletionPending = false;
          props.clearArchivedThreads();
        }}>Delete</button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
