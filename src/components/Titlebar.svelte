<script lang="ts">
  import { fade } from 'svelte/transition';
  import { IconLayoutSidebar, IconPlus, IconSettings } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import { profileById } from '../config/providers';
  import type { ThreadState } from '../types';
  import { menuFromEvent, type ContextMenuState } from '../utils/context-menu';
  import { folderName } from '../utils/threads';

  interface Props {
    settingsOpen: boolean;
    selectedThread?: ThreadState;
    windowMaximized: boolean;
    reducedMotion: boolean;
    toggleSidebar: () => void;
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
    toggleSidebar,
    addThread,
    closeApp,
    minimize,
    toggleMaximize,
  }: Props = $props();
  let contextMenu = $state<ContextMenuState>();

  function openWindowMenu(event: MouseEvent) {
    contextMenu = menuFromEvent(event, [
      { label: 'Minimize', action: minimize },
      { label: windowMaximized ? 'Restore' : 'Maximize', action: toggleMaximize },
      { label: 'Close', action: closeApp, separatorBefore: true },
    ]);
  }
</script>

<header class="titlebar" role="presentation" data-tauri-drag-region oncontextmenu={openWindowMenu}>
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
          <span class="title-copy" data-tauri-drag-region><strong>General</strong></span>
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
  </div>
</header>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}
