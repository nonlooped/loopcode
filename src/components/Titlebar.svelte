<script lang="ts">
  import { Tween } from 'svelte/motion';
  import { IconLayoutSidebar, IconPlus, IconTerminal2 } from '@tabler/icons-svelte';

  import appIcon from '../../assets/loopcode-mark.png';
  import ContextMenu from './ContextMenu.svelte';
  import MotionFade from './motion/MotionFade.svelte';
  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import type { HarnessProfile, ThreadState } from '../types';
  import { folderName } from '../utils/threads';
  import { shellLayoutDuration } from '../utils/shell-layout-motion';

  interface Props {
    settingsOpen: boolean;
    selectedThread?: ThreadState;
    windowMaximized: boolean;
    reducedMotion: boolean;
    sidebarCollapsed: boolean;
    projectExplorerCollapsed: boolean;
    compactLayout: boolean;
    compactExplorerOpen: boolean;
    hideChromeNewThread: boolean;
    profiles: HarnessProfile[];
    terminalOpen: boolean;
    toggleSidebar: () => void;
    toggleTerminal: () => void;
    addThread: () => void;
    closeApp: () => void;
    minimize: () => void;
    toggleMaximize: () => void;
  }

  const {
    settingsOpen,
    selectedThread,
    windowMaximized,
    reducedMotion,
    sidebarCollapsed,
    projectExplorerCollapsed,
    compactLayout,
    compactExplorerOpen,
    hideChromeNewThread,
    profiles,
    terminalOpen,
    toggleSidebar,
    toggleTerminal,
    addThread,
    closeApp,
    minimize,
    toggleMaximize,
  }: Props = $props();

  const macOS = ['darwin', 'macos'].includes(document.documentElement.dataset.platform ?? '');
  const fadeDuration = $derived(reducedMotion ? 0 : 130);

  const titleContextPaddingLeftTarget = $derived(
    compactLayout ? (sidebarCollapsed ? 14 : 160) : (sidebarCollapsed ? 2 : 14),
  );
  const titleContextPaddingLeft = new Tween(14, { duration: 0 });
  $effect(() => {
    void titleContextPaddingLeft.set(titleContextPaddingLeftTarget, {
      duration: shellLayoutDuration(reducedMotion),
    });
  });

  const titleContextPaddingRight = $derived(
    compactExplorerOpen
      ? (macOS ? `calc(46px + var(--project-explorer-expanded-width))` : `calc(var(--window-controls-width) + 46px + var(--project-explorer-expanded-width))`)
      : macOS
        ? (projectExplorerCollapsed ? 76 : 46)
        : (projectExplorerCollapsed
          ? `calc(var(--window-controls-width) + 76px)`
          : `calc(var(--window-controls-width) + 46px)`),
  );

  const terminalToggleRight = $derived(
    compactExplorerOpen
      ? `calc(8px + var(--project-explorer-expanded-width))`
      : macOS
        ? (projectExplorerCollapsed ? 41 : 8)
        : (projectExplorerCollapsed
          ? `calc(var(--window-controls-width) + 41px)`
          : `calc(var(--window-controls-width) + 8px)`),
  );

  const chromeButton =
    'relative z-10 grid size-[25px] shrink-0 place-items-center rounded-md border-0 bg-transparent p-0 text-muted hover:bg-panel-hover hover:text-text-soft';

  const trafficBase =
    'relative size-3 shrink-0 border-0 p-0 rounded-full shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.16)] active:brightness-[0.78] non-macos:text-text-soft non-macos:before:absolute non-macos:before:top-1/2 non-macos:before:left-1/2 non-macos:before:content-[\'\'] non-macos:after:absolute non-macos:after:top-1/2 non-macos:after:left-1/2 non-macos:after:content-[\'\'] windows:h-titlebar windows:w-[46px] windows:rounded-none windows:bg-transparent windows:shadow-none windows:hover:bg-[rgba(127,127,127,0.18)] non-macos-non-windows:size-6 non-macos-non-windows:bg-panel-hover non-macos-non-windows:shadow-[inset_0_0_0_1px_var(--line)] non-macos-non-windows:hover:bg-panel-active non-macos-non-windows:hover:text-text';

  function profileById(profileId: string) {
    return profiles.find((profile) => profile.id === profileId) ?? officialProfileById(profileId) ?? officialProfiles[0];
  }
</script>

{#snippet closeControl()}
  <button
    class="{trafficBase} traffic-close bg-[rgba(255,95,87,0.92)] non-macos:before:h-px non-macos:before:w-[11px] non-macos:before:bg-current non-macos:before:-translate-x-1/2 non-macos:before:-translate-y-1/2 non-macos:before:rotate-45 non-macos:after:h-px non-macos:after:w-[11px] non-macos:after:bg-current non-macos:after:-translate-x-1/2 non-macos:after:-translate-y-1/2 non-macos:after:-rotate-45 windows:hover:bg-[#c42b1c] windows:hover:text-white"
    aria-label="Close"
    title="Close"
    onclick={closeApp}
  ></button>
{/snippet}

{#snippet minimizeControl()}
  <button
    class="{trafficBase} traffic-minimize bg-[rgba(254,188,46,0.92)] non-macos:before:h-px non-macos:before:w-[9px] non-macos:before:bg-current non-macos:before:-translate-x-1/2 non-macos:before:-translate-y-1/2"
    aria-label="Minimize"
    title="Minimize"
    onclick={minimize}
  ></button>
{/snippet}

{#snippet maximizeControl()}
  <button
    class:restore={windowMaximized}
    class="{trafficBase} traffic-maximize bg-[rgba(40,200,64,0.92)] non-macos:before:box-border non-macos:before:h-[9px] non-macos:before:w-[9px] non-macos:before:-translate-x-1/2 non-macos:before:-translate-y-1/2 non-macos:before:border non-macos:before:border-current restore:non-macos:before:-translate-x-[65%] restore:non-macos:before:-translate-y-[35%] restore:non-macos:after:box-border restore:non-macos:after:h-[9px] restore:non-macos:after:w-[9px] restore:non-macos:after:-translate-x-[35%] restore:non-macos:after:-translate-y-[65%] restore:non-macos:after:border restore:non-macos:after:border-current"
    aria-label={windowMaximized ? 'Restore' : 'Maximize'}
    title={windowMaximized ? 'Restore' : 'Maximize'}
    onclick={toggleMaximize}
  ></button>
{/snippet}

<ContextMenu
  items={[
    { label: 'Minimize', action: minimize },
    { label: windowMaximized ? 'Restore' : 'Maximize', action: toggleMaximize },
    { label: 'Close', action: closeApp, separatorBefore: true },
  ]}
>
  {#snippet children({ props })}
    <header
      {...props}
      class="titlebar relative z-10 grid h-titlebar w-[calc(100%-var(--project-explorer-width))] select-none compact:w-full {sidebarCollapsed && !compactLayout ? 'grid-cols-[max-content_minmax(0,1fr)]' : 'grid-cols-[var(--sidebar-width)_minmax(0,1fr)]'} compact:grid-cols-[0_minmax(0,1fr)] non-macos:z-auto"
      role="presentation"
      data-tauri-drag-region
    >
      <div
        class="window-chrome flex min-w-0 items-center gap-[3px] bg-transparent py-0 pr-2.5 pl-3.5 compact:absolute compact:z-[2] compact:h-titlebar compact:w-[154px] compact:border-r-0 compact:pl-2.5 non-macos:z-auto"
        data-tauri-drag-region
      >
        <div
          class="traffic-controls mr-2.5 flex items-center gap-2 non-macos:fixed non-macos:top-0 non-macos:right-0 non-macos:z-[15] non-macos:mr-0 non-macos:h-titlebar non-macos:w-window-controls non-macos:justify-center windows:gap-0 non-macos-non-windows:gap-1.5"
        >
          {#if macOS}
            {@render closeControl()}
            {@render minimizeControl()}
            {@render maximizeControl()}
          {:else}
            {@render minimizeControl()}
            {@render maximizeControl()}
            {@render closeControl()}
          {/if}
        </div>
        <button class="{chromeButton}" aria-label="Toggle sidebar" title="Toggle sidebar" onclick={toggleSidebar}>
          <IconLayoutSidebar size={14} stroke={1.55} />
        </button>
        {#if !hideChromeNewThread}
          <button class="{chromeButton}" aria-label="New thread" title="New thread" onclick={addThread}>
            <IconPlus size={15} stroke={1.55} />
          </button>
        {/if}
      </div>
      <div
        class="title-context relative min-w-0 overflow-hidden whitespace-nowrap bg-transparent"
        data-tauri-drag-region
        style:padding-left="{titleContextPaddingLeft.current}px"
        style:padding-right={typeof titleContextPaddingRight === 'number' ? `${titleContextPaddingRight}px` : titleContextPaddingRight}
      >
        {#key settingsOpen ? 'settings' : `${selectedThread?.id ?? 'empty'}:${selectedThread?.title ?? ''}:${selectedThread?.profileId ?? ''}`}
          <MotionFade
            show={true}
            duration={fadeDuration}
            class="title-context-motion absolute inset-0 box-border flex min-w-0 items-center gap-[7px] px-[inherit] py-0"
          >
            {#if settingsOpen}
              <img class="title-brand-icon size-3.5 shrink-0 opacity-[0.68] [filter:var(--provider-filter)]" src={appIcon} alt="" aria-hidden="true" />
              <span class="title-copy flex min-w-0 flex-1 items-baseline gap-2" data-tauri-drag-region><strong class="min-w-0 flex-[1_1_auto] overflow-hidden text-xs font-semibold text-text-soft text-ellipsis">Settings</strong></span>
            {:else if selectedThread}
              {@const selectedProfile = profileById(selectedThread.profileId)}
              <span class="title-thread-context flex min-w-0 flex-1 items-center gap-[7px] overflow-hidden" data-tauri-drag-region>
                <img class:brand-color-icon={selectedProfile.iconMode === 'brand'} class="title-provider-icon size-3.5 shrink-0 opacity-[0.68] [filter:var(--provider-filter)]" src={selectedProfile.icon} alt="" />
                <span class="title-copy flex min-w-0 flex-1 items-baseline gap-2" data-tauri-drag-region>
                  <strong class="min-w-0 flex-[0_1_auto] overflow-hidden text-xs font-semibold text-text-soft text-ellipsis" title={selectedThread.title}>{selectedThread.title}</strong>
                  <small class="max-w-[38%] flex-[0_1_auto] overflow-hidden text-[11px] text-faint text-ellipsis max-[680px]:hidden" title={selectedThread.cwd}>{#if selectedThread.cwd}{folderName(selectedThread.cwd)}{/if}</small>
                </span>
              </span>
            {/if}
          </MotionFade>
        {/key}
      </div>
      {#if !settingsOpen && selectedThread}
        <button
          class:active={terminalOpen}
          class="{chromeButton} title-terminal-toggle !absolute top-1.5 z-[3] [&.active]:bg-panel-hover [&.active]:text-text-soft compact:!fixed compact:top-1.5 compact:z-[13]"
          style:right={typeof terminalToggleRight === 'number' ? `${terminalToggleRight}px` : terminalToggleRight}
          aria-label={terminalOpen ? 'Close terminal drawer' : 'Open terminal drawer'}
          aria-pressed={terminalOpen}
          title={`${terminalOpen ? 'Close' : 'Open'} terminal (Ctrl/Cmd+\`)`}
          onclick={toggleTerminal}
        >
          <IconTerminal2 size={14} stroke={1.55} />
        </button>
      {/if}
    </header>
  {/snippet}
</ContextMenu>
