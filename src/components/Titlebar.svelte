<script lang="ts">
  import { fade } from 'svelte/transition';
  import { IconLayoutSidebar, IconPlus, IconTerminal2 } from '@tabler/icons-svelte';

  import appIcon from '../../assets/loopcode-mark.png';
  import ContextMenu from './ContextMenu.svelte';
  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import type { HarnessProfile, ThreadState } from '../types';
  import { folderName } from '../utils/threads';

  interface Props {
    settingsOpen: boolean;
    selectedThread?: ThreadState;
    windowMaximized: boolean;
    reducedMotion: boolean;
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

  function profileById(profileId: string) {
    return profiles.find((profile) => profile.id === profileId) ?? officialProfileById(profileId) ?? officialProfiles[0];
  }
</script>

{#snippet closeControl()}
  <button class="size-3 rounded-full bg-[rgba(255,95,87,.92)] shadow-[inset_0_0_0_.5px_rgba(0,0,0,.16)] active:brightness-[.78]" aria-label="Close" title="Close" onclick={closeApp}></button>
{/snippet}

{#snippet minimizeControl()}
  <button class="size-3 rounded-full bg-[rgba(254,188,46,.92)] shadow-[inset_0_0_0_.5px_rgba(0,0,0,.16)] active:brightness-[.78]" aria-label="Minimize" title="Minimize" onclick={minimize}></button>
{/snippet}

{#snippet maximizeControl()}
  <button
    class="size-3 rounded-full bg-[rgba(40,200,64,.92)] shadow-[inset_0_0_0_.5px_rgba(0,0,0,.16)] active:brightness-[.78]"
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
    <header {...props} class="relative z-10 grid h-[var(--titlebar-height)] w-[calc(100%-var(--project-explorer-width))] grid-cols-[var(--sidebar-width)_minmax(0,1fr)] select-none" role="presentation" data-tauri-drag-region>
      <div class="flex min-w-0 items-center gap-1 px-3.5 pr-2.5" data-tauri-drag-region>
        <div class="mr-2.5 flex items-center gap-2">
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
        <button class="grid size-[25px] shrink-0 place-items-center rounded-md text-muted hover:bg-panel-hover hover:text-ink-soft" aria-label="Toggle sidebar" title="Toggle sidebar" onclick={toggleSidebar}>
          <IconLayoutSidebar size={14} stroke={1.55} />
        </button>
        <button class="grid size-[25px] shrink-0 place-items-center rounded-md text-muted hover:bg-panel-hover hover:text-ink-soft" aria-label="New thread" title="New thread" onclick={addThread}>
          <IconPlus size={15} stroke={1.55} />
        </button>
      </div>
      <div class="relative min-w-0 overflow-hidden whitespace-nowrap px-3.5 pr-[46px]" data-tauri-drag-region>
        {#key settingsOpen ? 'settings' : `${selectedThread?.id ?? 'empty'}:${selectedThread?.title ?? ''}:${selectedThread?.profileId ?? ''}`}
          <span class="absolute inset-0 flex min-w-0 items-center gap-2 px-[inherit]" transition:fade={{ duration: reducedMotion ? 0 : 130 }}>
            {#if settingsOpen}
              <img class="size-3.5 shrink-0 opacity-70 [filter:var(--provider-filter)]" src={appIcon} alt="" aria-hidden="true" />
              <span class="flex min-w-0 flex-1 items-baseline gap-2" data-tauri-drag-region><strong class="min-w-0 flex-[0_1_auto] truncate text-xs font-semibold text-ink-soft">Settings</strong></span>
            {:else if selectedThread}
              {@const selectedProfile = profileById(selectedThread.profileId)}
              <span class="flex min-w-0 flex-1 items-center gap-2 overflow-hidden" data-tauri-drag-region>
                <img class:brand-color-icon={selectedProfile.iconMode === 'brand'} class="size-3.5 shrink-0 opacity-70 [filter:var(--provider-filter)]" src={selectedProfile.icon} alt="" />
                <span class="flex min-w-0 flex-1 items-baseline gap-2" data-tauri-drag-region>
                  <strong class="min-w-0 flex-[0_1_auto] truncate text-xs font-semibold text-ink-soft" title={selectedThread.title}>{selectedThread.title}</strong>
                  <small class="max-w-[38%] flex-[0_1_auto] truncate text-[11px] text-faint" title={selectedThread.cwd}>{#if selectedThread.cwd}{folderName(selectedThread.cwd)}{/if}</small>
                </span>
              </span>
            {/if}
          </span>
        {/key}
      </div>
      {#if !settingsOpen && selectedThread}
        <button
          class={`absolute top-1.5 right-2 z-3 grid size-[25px] place-items-center rounded-md text-muted hover:bg-panel-hover hover:text-ink-soft ${terminalOpen ? 'bg-panel-hover text-ink-soft' : ''}`}
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
