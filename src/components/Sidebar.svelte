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

<aside class="absolute top-0 bottom-0 left-0 z-9 flex w-[var(--sidebar-width)] min-h-0 flex-col overflow-visible border-r border-line bg-[var(--sidebar-tint)] pt-[var(--titlebar-height)]">
  <div
    class="absolute top-0 right-[-4px] bottom-0 z-4 w-2 touch-none cursor-ew-resize before:absolute before:top-0 before:bottom-0 before:left-1 before:w-px before:bg-transparent hover:before:bg-line-strong"
    role="separator"
    aria-label="Resize left sidebar"
    aria-orientation="vertical"
    onpointerdown={props.startResize}
  ></div>
  {#if props.settingsOpen}
    <div class="px-2 pt-3">
      <div class="flex items-center gap-1.5 px-2 pb-1 text-[11px] font-medium text-faint">
        <img class="size-3 shrink-0 opacity-70 [filter:var(--provider-filter)]" src={appIcon} alt="" aria-hidden="true" />
        <span>Settings</span>
      </div>
      <nav class="grid gap-0.5" aria-label="Settings categories">
        {#each settingsCategories as { category, label, icon: Icon } (category)}
          <button
            class={`flex min-h-8 w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] text-muted hover:bg-panel-hover hover:text-ink-soft [&>svg]:shrink-0 ${props.settingsCategory === category ? 'bg-panel-active font-medium text-ink' : ''}`}
            aria-current={props.settingsCategory === category ? 'page' : undefined}
            onclick={() => props.setSettingsCategory(category)}
          >
            <Icon size={16} stroke={1.55} />
            <span>{label}</span>
          </button>
        {/each}
      </nav>
    </div>
    <div class="flex-1"></div>
    <div class="px-2 pb-3">
      <button class="flex min-h-8 w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] text-muted hover:bg-panel-hover hover:text-ink" onclick={props.closeSettings}>
        <IconArrowLeft size={16} stroke={1.55} />
        <span>Back</span>
      </button>
    </div>
  {:else}
    <div class="flex h-[41px] shrink-0 items-center justify-between px-2 pt-2 pb-1 text-[11px] text-muted">
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="flex h-[29px] min-w-0 flex-1 items-center gap-2 rounded-lg px-2 text-left text-muted hover:bg-panel-hover hover:text-ink-soft" title="Choose folder">
          <IconFolder size={15} stroke={1.55} />
          <strong class="min-w-0 flex-1 truncate text-[13px] font-medium text-ink-soft">{props.activeProject ? props.activeProject.name : 'All projects'}</strong>
          <IconChevronDown class="shrink-0 text-faint" size={12} stroke={1.55} />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            class="grid max-h-80 w-[min(320px,calc(100vw_-_16px))] gap-0.5 overflow-auto rounded-xl border border-line bg-floating p-1.5 shadow-overlay backdrop-blur-xl"
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
              <DropdownMenu.RadioItem class="flex min-h-[38px] w-full items-center gap-2 rounded-md border border-transparent px-2 py-1.5 text-left text-muted hover:bg-panel-hover hover:text-ink-soft data-[state=checked]:border-line data-[state=checked]:bg-panel-active" value="__all-projects__">
                {#snippet children({ checked })}
                  <IconFolder size={13} stroke={1.55} />
                  <span class="grid min-w-0 flex-1 gap-px">
                    <strong class="truncate text-xs font-semibold text-ink-soft">All projects</strong>
                    <small class="truncate text-[11px] text-faint" title={props.defaultWorkingFolder}>{props.defaultWorkingFolder || 'local'} · default</small>
                  </span>
                  {#if checked}<IconCheck size={13} stroke={2} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
              {#each props.projects as project (project.id)}
                <ContextMenu items={projectMenuItems(project)}>
                  {#snippet children({ props: triggerProps })}
                    <DropdownMenu.RadioItem {...triggerProps} class="flex min-h-[38px] w-full items-center gap-2 rounded-md border border-transparent px-2 py-1.5 text-left text-muted hover:bg-panel-hover hover:text-ink-soft data-[state=checked]:border-line data-[state=checked]:bg-panel-active" value={project.id}>
                      {#snippet children({ checked })}
                        <IconFolder size={13} stroke={1.55} />
                        <span class="grid min-w-0 flex-1 gap-px">
                          <strong class="truncate text-xs font-semibold text-ink-soft">{project.name}</strong>
                          <small class="truncate text-[11px] text-faint" title={project.path}>{project.path}</small>
                        </span>
                        {#if checked}<IconCheck size={13} stroke={2} />{/if}
                      {/snippet}
                    </DropdownMenu.RadioItem>
                  {/snippet}
                </ContextMenu>
              {/each}
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator class="mt-1 mb-0.5 h-px bg-line" />
            <DropdownMenu.Item class="flex min-h-8 w-full items-center gap-1.5 rounded-md px-2 text-[11px] text-muted hover:bg-panel-hover hover:text-ink-soft" onSelect={props.addProject}>
              <IconFolderPlus size={13} stroke={1.55} /> Add folder…
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <div class="ml-1 flex shrink-0 items-center gap-1">
        <button class="grid size-6 place-items-center rounded-md text-muted hover:bg-panel-hover hover:text-ink-soft" aria-label="New thread" title="New thread" onclick={props.addThread}>
          <IconPlus size={14} stroke={1.55} />
        </button>
      </div>
    </div>
    <div class="min-h-0 flex-1 overflow-auto px-2 pt-1 pb-2">
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
