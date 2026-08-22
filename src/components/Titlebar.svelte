<script lang="ts">
  import { fade } from 'svelte/transition';
  import { IconLayoutSidebar, IconPlus, IconSettings, IconTerminal2 } from '@tabler/icons-svelte';

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

  function profileById(profileId: string) {
    return profiles.find((profile) => profile.id === profileId) ?? officialProfileById(profileId) ?? officialProfiles[0];
  }
</script>

<ContextMenu
  items={[
    { label: 'Minimize', action: minimize },
    { label: windowMaximized ? 'Restore' : 'Maximize', action: toggleMaximize },
    { label: 'Close', action: closeApp, separatorBefore: true },
  ]}
>
  {#snippet children({ props })}
    <header {...props} class="titlebar" role="presentation" data-tauri-drag-region>
      <div class="window-chrome" data-tauri-drag-region>
        <div class="traffic-controls">
          <button class="traffic-close" aria-label="Close" title="Close" onclick={closeApp}></button>
          <button class="traffic-minimize" aria-label="Minimize" title="Minimize" onclick={minimize}></button>
          <button class="traffic-maximize" aria-label="Maximize" title="Maximize" onclick={toggleMaximize}></button>
        </div>
        <button class="chrome-button" aria-label="Toggle sidebar" title="Toggle sidebar" onclick={toggleSidebar}>
          <IconLayoutSidebar size={14} stroke={1.45} />
        </button>
        <button class="chrome-button" aria-label="New thread" title="New thread" onclick={addThread}>
          <IconPlus size={15} stroke={1.55} />
        </button>
      </div>
      <div class="title-context" data-tauri-drag-region>
        {#key settingsOpen ? 'settings' : `${selectedThread?.id ?? 'empty'}:${selectedThread?.title ?? ''}:${selectedThread?.profileId ?? ''}`}
          <span class="title-context-motion" transition:fade={{ duration: reducedMotion ? 0 : 130 }}>
            {#if settingsOpen}
              <IconSettings class="title-settings-icon" size={14} stroke={1.55} />
              <span class="title-copy" data-tauri-drag-region><strong>Settings</strong></span>
            {:else if selectedThread}
              {@const selectedProfile = profileById(selectedThread.profileId)}
              <span class="title-thread-context" data-tauri-drag-region>
                <img class="title-provider-icon" src={selectedProfile.icon} alt="" />
                <span class="title-copy" data-tauri-drag-region>
                  <strong title={selectedThread.title}>{selectedThread.title}</strong>
                  <small title={selectedThread.cwd}>{#if selectedThread.cwd}{folderName(selectedThread.cwd)}{/if}</small>
                </span>
              </span>
            {/if}
          </span>
        {/key}
        {#if !settingsOpen && selectedThread}
          <button
            class:active={terminalOpen}
            class="chrome-button title-terminal-toggle"
            aria-label={terminalOpen ? 'Close terminal drawer' : 'Open terminal drawer'}
            aria-pressed={terminalOpen}
            title={`${terminalOpen ? 'Close' : 'Open'} terminal (Ctrl/Cmd+\`)`}
            onclick={toggleTerminal}
          >
            <IconTerminal2 size={14} stroke={1.55} />
          </button>
        {/if}
      </div>
    </header>
  {/snippet}
</ContextMenu>
