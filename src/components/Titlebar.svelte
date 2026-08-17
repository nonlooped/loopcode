<script lang="ts">
  import { IconLayoutSidebar, IconPlus, IconSettings } from '@tabler/icons-svelte';

  import { profileById } from '../config/providers';
  import type { ThreadState } from '../types';
  import { folderName } from '../utils/threads';

  interface Props {
    settingsOpen: boolean;
    selectedThread?: ThreadState;
    toggleSidebar: () => void;
    addThread: () => void;
    closeApp: () => void;
    minimize: () => void;
    toggleMaximize: () => void;
  }

  const {
    settingsOpen,
    selectedThread,
    toggleSidebar,
    addThread,
    closeApp,
    minimize,
    toggleMaximize,
  }: Props = $props();
</script>

<header class="titlebar" data-tauri-drag-region>
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
    {#if settingsOpen}
      <IconSettings class="title-settings-icon" size={14} stroke={1.55} />
      <span class="title-copy"><strong>General</strong></span>
    {:else if selectedThread}
      {@const selectedProfile = profileById(selectedThread.profileId)}
      <span class="title-thread-context">
        <img class="title-provider-icon" src={selectedProfile.icon} alt="" />
        <span class="title-copy">
          <strong title={selectedThread.title}>{selectedThread.title}</strong>
          <small title={selectedThread.cwd}>{#if selectedThread.cwd}{folderName(selectedThread.cwd)}{/if}</small>
        </span>
      </span>
    {/if}
  </div>
</header>
