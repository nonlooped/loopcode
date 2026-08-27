<script lang="ts">
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
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
  import MotionFade from './motion/MotionFade.svelte';
  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import type { HarnessProfile, PermissionRequest, ProjectState, ThreadState } from '../types';
  import type { SettingsCategory } from '../utils/app-settings';
  import { copyText } from '../utils/clipboard';
  import { folderName, relativeTime, threadAttention, threadHarness } from '../utils/threads';

  interface Props {
    open: boolean;
    collapsed: boolean;
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

  const settingsNavItem =
    'flex min-h-8 w-full items-center gap-2 rounded-lg border-0 bg-transparent px-2 py-1.5 text-left text-[13px] text-muted hover:bg-panel-hover hover:text-text-soft [&.active]:bg-panel-active [&.active]:font-medium [&.active]:text-text [&_span]:min-w-0 [&_span]:truncate [&_svg]:shrink-0 [&_svg]:text-muted';
  const workspaceIdentity =
    'flex h-[29px] min-w-0 flex-1 items-center gap-2 rounded-lg border-0 bg-transparent px-2 text-left text-muted hover:bg-panel-hover hover:text-text-soft [&>svg:first-child]:shrink-0 [&>svg:first-child]:text-muted [&_strong]:min-w-0 [&_strong]:flex-1 [&_strong]:truncate [&_strong]:text-[13px] [&_strong]:font-medium [&_strong]:text-text-soft';
  const iconButton =
    'grid size-6 shrink-0 place-items-center rounded-md border border-transparent bg-transparent p-0 text-muted hover:bg-panel-hover hover:text-text-soft';
  const threadItem =
    'group/thread relative block w-full min-h-[65px] mt-0.5 rounded-lg border-0 bg-transparent px-2.5 py-[9px] text-left hover:bg-panel-hover [&.active]:bg-row-selected [&.active_.thread-location_span]:text-muted [&.active_.thread-details]:text-muted [&.active_.slim-updated]:text-muted hover:[&:not(.slim)]:[&_.thread-title-motion]:pr-[50px] focus-within:[&:not(.slim)]:[&_.thread-title-motion]:pr-[50px]';
  const threadItemSlim =
    'slim settled flex h-9 min-h-9! items-center gap-2.5 px-2.5 py-0';
  const threadSelect = 'block w-full border-0 bg-transparent p-0 text-left text-inherit group-[.slim]/thread:flex group-[.slim]/thread:min-w-0 group-[.slim]/thread:flex-1 group-[.slim]/thread:items-center group-[.slim]/thread:gap-2.5';
  const threadActionBtn =
    'grid size-[22px] place-items-center rounded-[5px] border-0 bg-panel-active p-0 text-muted hover:bg-panel-active hover:text-text-soft [&.remove-thread]:hover:text-danger';
  const confirmationOverlay = 'fixed inset-0 z-[90] bg-overlay backdrop-blur-scrim';
  const confirmationModal =
    'fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw-40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-overlay border border-line-strong bg-decision p-[18px] shadow-overlay outline-0 backdrop-blur-overlay [&_[role=heading]]:text-[15px] [&_[role=heading]]:leading-snug [&_[role=heading]]:font-semibold [&_[role=heading]]:tracking-tight [&_[role=heading]]:text-text [&_[data-alert-dialog-description]]:mt-[5px] [&_[data-alert-dialog-description]]:text-xs [&_[data-alert-dialog-description]]:leading-snug [&_[data-alert-dialog-description]]:text-muted';
  const confirmationActions =
    'mt-[18px] flex flex-wrap justify-end gap-[7px] [&_button]:rounded-[7px] [&_button]:border [&_button]:border-line [&_button]:bg-transparent [&_button]:px-3 [&_button]:py-[7px] [&_button]:text-text-soft hover:[&_button]:border-line-strong hover:[&_button]:bg-panel-hover hover:[&_button]:text-text [&_button.danger]:border-[color-mix(in_srgb,var(--danger)_30%,transparent)] [&_button.danger]:text-danger hover:[&_button.danger]:bg-[color-mix(in_srgb,var(--danger)_12%,transparent)]';

  function motionDuration(ms: number) {
    return props.compactMotion ? 0 : ms;
  }

  const sidebarOpacity = new Tween(1, { duration: 0, easing: cubicOut });
  $effect(() => {
    void sidebarOpacity.set(props.collapsed ? 0 : 1, { duration: motionDuration(180) });
  });

  const chevronRotation = new Tween(untrack(() => props.showSettled ? 90 : 0), { duration: 0, easing: cubicOut });
  $effect(() => {
    void chevronRotation.set(props.showSettled ? 90 : 0, { duration: motionDuration(140) });
  });

  const settingsCategories = [
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

<aside
  class:open={props.open}
  class="thread-sidebar absolute top-0 bottom-0 left-0 z-[9] flex min-h-0 w-sidebar flex-col overflow-visible border-r border-line bg-sidebar pt-titlebar will-change-[width,opacity] compact:fixed compact:top-titlebar compact:bottom-0 compact:w-[var(--sidebar-expanded-width)] compact:translate-x-[-103%] compact:border-r-0 compact:bg-raised compact:pt-0 compact:shadow-[22px_0_60px_rgba(0,0,0,0.4)] compact:[&:not(.open)]:invisible compact:[&:not(.open)]:pointer-events-none compact:[&.open]:visible compact:[&.open]:pointer-events-auto compact:[&.open]:translate-x-0 non-macos:desktop-min:shell-sidebar-open:has-[.sidebar-heading]:pt-0 {props.collapsed ? 'pointer-events-none' : ''}"
  style:opacity={sidebarOpacity.current}
>
  <div
    class="sidebar-resize-handle left-sidebar-resize-handle absolute top-0 -right-1 bottom-0 z-[4] w-[7px] cursor-ew-resize touch-none after:absolute after:inset-y-0 after:left-[3px] after:w-px after:bg-transparent after:content-[''] hover:after:bg-line-strong compact:hidden {props.collapsed ? 'hidden' : ''}"
    role="separator"
    aria-label="Resize left sidebar"
    aria-orientation="vertical"
    onpointerdown={props.startResize}
  ></div>
  {#if props.settingsOpen}
    <div class="settings-nav px-2 pt-3 pb-0">
      <div class="settings-nav-label flex items-center gap-1.5 px-1 pb-1 text-[11px] font-medium text-faint">
        <button class="settings-back grid size-5 shrink-0 place-items-center rounded-md border-0 bg-transparent p-0 text-muted hover:bg-panel-hover hover:text-text" aria-label="Back" title="Back" onclick={props.closeSettings}>
          <IconArrowLeft size={13} stroke={1.55} />
        </button>
        <img class="settings-nav-logo size-3 shrink-0 opacity-[0.68] [filter:var(--provider-filter)]" src={appIcon} alt="" aria-hidden="true" />
        <span>Settings</span>
      </div>
      <nav class="grid gap-0.5" aria-label="Settings categories">
        {#each settingsCategories as { category, label, icon: Icon } (category)}
          <button
            class:active={props.settingsCategory === category}
            class="{settingsNavItem} settings-nav-item"
            aria-current={props.settingsCategory === category ? 'page' : undefined}
            onclick={() => props.setSettingsCategory(category)}
          >
            <Icon class="shrink-0 text-muted" size={16} stroke={1.55} />
            <span class="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{label}</span>
          </button>
        {/each}
      </nav>
    </div>
    <div class="settings-nav-spacer flex-1"></div>
  {:else}
    <div class="sidebar-heading flex h-[41px] shrink-0 items-center justify-between px-2 pt-2 pb-1 text-[11px] text-muted non-macos:desktop-min:shell-sidebar-open:h-titlebar non-macos:desktop-min:shell-sidebar-open:py-1 non-macos:desktop-min:shell-sidebar-open:pr-2 non-macos:desktop-min:shell-sidebar-open:pl-[43px]">
      <DropdownMenu.Root>
        <DropdownMenu.Trigger
          class="workspace-identity flex h-[29px] min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-lg border-0 bg-transparent px-2 text-left text-muted hover:bg-panel-hover hover:text-text-soft [&>svg:first-child]:shrink-0 [&>svg:first-child]:text-muted [&_strong]:min-w-0 [&_strong]:flex-1 [&_strong]:overflow-hidden [&_strong]:text-[13px] [&_strong]:font-medium [&_strong]:text-text-soft [&_strong]:text-ellipsis [&_strong]:whitespace-nowrap"
          title="Choose folder"
        >
          <IconFolder size={15} stroke={1.55} />
          <strong>{props.activeProject ? props.activeProject.name : 'All projects'}</strong>
          <IconChevronDown class="workspace-chevron shrink-0 text-faint" size={12} stroke={1.55} />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            class="workspace-dropdown grid max-h-80 w-[min(320px,calc(100vw-16px))] gap-0.5 overflow-auto rounded-overlay border border-line bg-floating p-1.5 shadow-overlay backdrop-blur-overlay"
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
              <DropdownMenu.RadioItem
                class="workspace-option flex min-h-[38px] w-full items-center gap-2 rounded-[7px] border border-transparent bg-transparent px-2 py-1.5 text-left text-muted hover:bg-panel-hover hover:text-text-soft data-[state=checked]:border-line data-[state=checked]:bg-panel-active data-[state=checked]:text-text-soft"
                value="__all-projects__"
              >
                {#snippet children({ checked })}
                  <IconFolder size={13} stroke={1.55} />
                  <span class="workspace-option-main grid min-w-0 flex-1 gap-px">
                    <strong class="overflow-hidden text-xs font-semibold text-text-soft text-ellipsis whitespace-nowrap">All projects</strong>
                    <small class="overflow-hidden text-[11px] text-faint text-ellipsis whitespace-nowrap" title={props.defaultWorkingFolder}>{props.defaultWorkingFolder || 'local'} · default</small>
                  </span>
                  {#if checked}<IconCheck size={13} stroke={2} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
              {#each props.projects as project (project.id)}
                <ContextMenu items={projectMenuItems(project)}>
                  {#snippet children({ props: triggerProps })}
                    <DropdownMenu.RadioItem {...triggerProps} class="workspace-option flex min-h-[38px] w-full items-center gap-2 rounded-[7px] border border-transparent bg-transparent px-2 py-1.5 text-left text-muted hover:bg-panel-hover hover:text-text-soft data-[state=checked]:border-line data-[state=checked]:bg-panel-active data-[state=checked]:text-text-soft" value={project.id}>
                      {#snippet children({ checked })}
                        <IconFolder size={13} stroke={1.55} />
                        <span class="workspace-option-main grid min-w-0 flex-1 gap-px">
                          <strong class="overflow-hidden text-xs font-semibold text-text-soft text-ellipsis whitespace-nowrap">{project.name}</strong>
                          <small class="overflow-hidden text-[11px] text-faint text-ellipsis whitespace-nowrap" title={project.path}>{project.path}</small>
                        </span>
                        {#if checked}<IconCheck size={13} stroke={2} />{/if}
                      {/snippet}
                    </DropdownMenu.RadioItem>
                  {/snippet}
                </ContextMenu>
              {/each}
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator class="workspace-dropdown-separator my-1 h-px bg-line" />
            <DropdownMenu.Item class="workspace-add-folder flex min-h-8 w-full items-center gap-1.5 rounded-[7px] border-0 bg-transparent px-2 text-[11px] text-muted hover:bg-panel-hover hover:text-text-soft" onSelect={props.addProject}>
              <IconFolderPlus size={13} stroke={1.55} /> Add folder…
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <div class="heading-actions ml-1 flex shrink-0 items-center gap-1">
        <button class="icon-button grid size-6 shrink-0 place-items-center rounded-md border border-transparent bg-transparent p-0 text-muted hover:bg-panel-hover hover:text-text-soft" aria-label="New thread" title="New thread" onclick={props.addThread}>
          <IconPlus size={14} stroke={1.55} />
        </button>
      </div>
    </div>
    <div class="sidebar-scroll min-h-0 flex-1 overflow-auto px-2 pt-1 pb-2">
      <section class="sidebar-section m-0">
        {#if props.inboxThreads.length === 0}
          <div class="empty-hint px-2 py-2 text-xs leading-snug text-faint">No threads yet</div>
        {:else}
          {#each inboxGroups as group (group.kind)}
            <div class="thread-group not-first:mt-2">
              <div class="thread-group-label px-2.5 pt-1 pb-0.5 text-[11px] font-semibold text-faint">{group.label} · {group.threads.length}</div>
              <nav class="thread-list min-h-0 overflow-visible p-0" aria-label={`${group.label} threads`}>
                {#each group.threads as thread (thread.id)}
                  {@const threadProfile = profileById(thread.profileId)}
                  {@const attention = threadAttention(thread, props.interactionRequestsByThread[thread.id])}
                  <div class="thread-motion w-full" animate:flip={{ duration: motionDuration(260) }}>
                    <ContextMenu items={threadMenuItems(thread)}>
                      {#snippet children({ props: triggerProps })}
                        <MotionFade show={true} duration={motionDuration(150)} class="contents">
                          <div
                            {...triggerProps}
                            class:active={thread.id === props.selectedThreadId}
                            class="{threadItem} thread-item focus-within:[&_.thread-actions]:pointer-events-auto focus-within:[&_.thread-actions]:opacity-100 hover:[&_.thread-actions]:pointer-events-auto hover:[&_.thread-actions]:opacity-100 compact-session-rows:min-h-12 compact-session-rows:py-1 compact-session-rows:[&_.thread-topline]:hidden compact-session-rows:[&_.thread-details]:hidden"
                            role="group"
                            aria-label={thread.title}
                          >
                            <button
                              type="button"
                              class="thread-select block w-full border-0 bg-transparent p-0 text-left text-inherit"
                              aria-current={thread.id === props.selectedThreadId ? 'page' : undefined}
                              onclick={() => props.selectThread(thread.id)}
                            >
                              <span class="thread-copy grid min-w-0 gap-0.5">
                                <small class="thread-topline flex min-w-0 items-center justify-between gap-2 text-[11px] leading-[14px] font-medium text-muted">
                                  <span class="thread-location flex min-w-0 items-center gap-1 overflow-hidden text-ellipsis whitespace-nowrap text-faint">
                                    {#if thread.projectId}
                                      {@const project = props.projects.find((item) => item.id === thread.projectId)}
                                      {project ? project.name : folderName(thread.cwd)}
                                    {:else}
                                      {thread.cwd ? folderName(thread.cwd) : '~'}
                                    {/if}
                                  </span>
                                  <span class="thread-status-motion ml-auto grid min-w-0">
                                    {#key attention.kind}
                                      <MotionFade show={true} duration={motionDuration(120)} class="min-w-0 [grid-area:1/1]">
                                        <span
                                          class:attention={attention.kind === 'needs-approval' || attention.kind === 'needs-answer'}
                                          class:live={attention.kind === 'working'}
                                          class:error={attention.kind === 'failed'}
                                          class="thread-updated shrink-0 [&.attention]:text-warning [&.live]:text-success [&.live]:before:mr-[5px] [&.live]:before:mb-px [&.live]:before:inline-block [&.live]:before:size-[5px] [&.live]:before:rounded-full [&.live]:before:bg-success [&.live]:before:content-[''] [&.error]:text-danger"
                                        >
                                          {attention.kind === 'recent' ? relativeTime(thread.updatedAt) : attention.label}
                                        </span>
                                      </MotionFade>
                                    {/key}
                                  </span>
                                </small>
                                <span class="thread-title-motion grid min-w-0">
                                  {#key thread.title}
                                    <MotionFade show={true} duration={motionDuration(120)} class="min-w-0 overflow-hidden text-[13px] leading-[17px] font-medium text-text-soft text-ellipsis whitespace-nowrap [grid-area:1/1]">
                                      <strong>{thread.title}</strong>
                                    </MotionFade>
                                  {/key}
                                </span>
                                <span class="thread-details flex h-3.5 min-w-0 items-center gap-1 overflow-hidden text-[11px] leading-[14px] text-ellipsis whitespace-nowrap text-faint">
                                  <img class:brand-color-icon={threadProfile.iconMode === 'brand'} class="size-[11px] shrink-0 opacity-[0.62] [filter:var(--provider-filter)]" src={threadProfile.icon} alt="" />
                                  <span class="thread-provider shrink-0">{threadHarness(thread, props.profiles)}</span>
                                  {#if attention.kind !== 'recent'}
                                    <span class="thread-reason min-w-0 overflow-hidden text-muted text-ellipsis whitespace-nowrap">· {attention.reason}</span>
                                  {/if}
                                </span>
                              </span>
                            </button>
                            <span class="thread-actions pointer-events-none absolute top-[22px] right-[5px] flex items-center gap-0.5 opacity-0">
                              <button
                                type="button"
                                class="action-settle grid size-[22px] place-items-center rounded-[5px] border-0 bg-panel-active p-0 text-muted hover:bg-panel-active hover:text-text-soft"
                                aria-label={`Archive ${thread.title}`}
                                title="Archive"
                                onclick={(event) => { event.stopPropagation(); props.toggleSettled(thread.id); }}
                              ><IconArchive size={13} stroke={1.55} /></button>
                              <button
                                type="button"
                                class="remove-thread grid size-[22px] place-items-center rounded-[5px] border-0 bg-panel-active p-0 text-muted hover:text-danger"
                                aria-label={`Delete ${thread.title}`}
                                title="Delete thread"
                                onclick={(event) => { event.stopPropagation(); props.removeThread(thread.id); }}
                              ><IconTrash size={13} stroke={1.55} /></button>
                            </span>
                          </div>
                        </MotionFade>
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
        class="sidebar-section settled-section mt-0.5"
        open={props.showSettled}
        onOpenChange={props.setShowSettled}
      >
        <Collapsible.Trigger class="section-header mt-3 mb-1 flex h-6 w-full items-center justify-between gap-2 rounded-md border-0 bg-transparent px-2.5 text-[11px] font-semibold tracking-normal text-faint hover:bg-panel-hover hover:text-text-soft">
          <span class="section-title flex min-w-0 shrink-0 items-center gap-[5px]">Archived · {props.settledThreads.length}</span>
          <span class="section-rule h-px flex-1 bg-line"></span>
          <span class="section-chevron shrink-0" style:transform="rotate({chevronRotation.current}deg)">
            <IconChevronRight size={11} stroke={1.55} />
          </span>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <nav class="thread-list settled min-h-0 overflow-visible p-0" aria-label="Archived threads">
              {#each props.settledThreads as thread (thread.id)}
                {@const threadProfile = profileById(thread.profileId)}
                <div class="thread-motion w-full" animate:flip={{ duration: motionDuration(260) }}>
                  <ContextMenu items={threadMenuItems(thread)}>
                    {#snippet children({ props: triggerProps })}
                      <MotionFade show={true} duration={motionDuration(150)} class="contents">
                        <div
                          {...triggerProps}
                          class:active={thread.id === props.selectedThreadId}
                          class="{threadItem} {threadItemSlim} thread-item group relative mt-0.5 w-full rounded-lg border-0 bg-transparent text-left hover:bg-panel-hover focus-within:[&_.slim-updated]:opacity-0 hover:[&_.slim-updated]:opacity-0 focus-within:[&_.thread-actions]:pointer-events-auto focus-within:[&_.thread-actions]:opacity-100 hover:[&_.thread-actions]:pointer-events-auto hover:[&_.thread-actions]:opacity-100"
                          role="group"
                          aria-label={thread.title}
                          title={thread.title}
                        >
                          <button
                            type="button"
                            class="thread-select flex min-w-0 flex-1 items-center gap-2.5 border-0 bg-transparent p-0 text-left text-inherit"
                            aria-current={thread.id === props.selectedThreadId ? 'page' : undefined}
                            onclick={() => props.selectThread(thread.id)}
                          >
                            <span class="slim-copy flex min-w-0 flex-1 items-center gap-2 overflow-hidden text-[13px] text-muted">
                              <img class:brand-color-icon={threadProfile.iconMode === 'brand'} class="size-3.5 shrink-0 opacity-[0.48] [filter:var(--provider-filter)]" src={threadProfile.icon} alt="" />
                              <span class="thread-title-motion grid min-w-0 flex-1">
                                {#key thread.title}
                                  <MotionFade show={true} duration={motionDuration(120)} class="min-w-0 overflow-hidden text-[13px] font-normal text-muted text-ellipsis whitespace-nowrap [grid-area:1/1]">
                                    <span>{thread.title}</span>
                                  </MotionFade>
                                {/key}
                              </span>
                            </span>
                            <span class="slim-updated shrink-0 text-[11px] text-faint">{relativeTime(thread.updatedAt)}</span>
                          </button>
                          <span class="thread-actions pointer-events-none absolute top-[7px] right-[5px] flex items-center gap-0.5 opacity-0">
                            <button
                              type="button"
                              class="action-unsettle grid size-[22px] place-items-center rounded-[5px] border-0 bg-panel-active p-0 text-muted hover:bg-panel-active hover:text-text-soft"
                              aria-label={`Unarchive ${thread.title}`}
                              title="Unarchive"
                              onclick={(event) => { event.stopPropagation(); props.toggleSettled(thread.id); }}
                            ><IconInbox size={12} stroke={1.55} /></button>
                            <button
                              type="button"
                              class="remove-thread grid size-[22px] place-items-center rounded-[5px] border-0 bg-panel-active p-0 text-muted hover:text-danger"
                              aria-label={`Delete ${thread.title}`}
                              title="Delete thread"
                              onclick={(event) => { event.stopPropagation(); props.removeThread(thread.id); }}
                            ><IconTrash size={12} stroke={1.55} /></button>
                          </span>
                        </div>
                      </MotionFade>
                    {/snippet}
                  </ContextMenu>
                </div>
              {/each}
          </nav>
          <button class="clear-archived mx-2 mt-1 flex min-h-8 w-[calc(100%-16px)] items-center gap-1.5 rounded-[7px] border-0 bg-transparent px-2 text-[11px] text-faint hover:bg-panel-hover hover:text-danger" onclick={() => { archiveDeletionPending = true; }}>
            <IconTrash size={12} stroke={1.55} /> Delete all archived
          </button>
        </Collapsible.Content>
      </Collapsible.Root>
      {/if}
    </div>

    <button class="sidebar-foot mx-2 mt-1 mb-2 flex min-h-9 w-[calc(100%-16px)] shrink-0 items-center gap-2 rounded-lg border border-transparent bg-transparent px-2 py-2 text-left text-[13px] text-muted hover:bg-panel-hover hover:text-text [&_svg]:shrink-0" onclick={props.openSettings}>
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
    <AlertDialog.Overlay class={confirmationOverlay} />
    <AlertDialog.Content class={confirmationModal}>
      <AlertDialog.Title>Delete archived threads?</AlertDialog.Title>
      <AlertDialog.Description>
        {props.settledThreads.length} archived {props.settledThreads.length === 1 ? 'thread' : 'threads'} and their history will be permanently deleted.
      </AlertDialog.Description>
      <footer class={confirmationActions}>
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button class="danger" onclick={() => {
          archiveDeletionPending = false;
          props.clearArchivedThreads();
        }}>Delete</button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
