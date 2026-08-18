<script lang="ts">
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { IconBolt, IconCheck } from '@tabler/icons-svelte';

  import type { ProviderSessionState } from '../types';
  import { nextMenuItemIndex } from '../utils/context-menu';
  import { fastModeAvailable } from '../utils/fast-mode';

  interface Props {
    provider: ProviderSessionState;
    open: boolean;
    reducedMotion: boolean;
    setOpen: (open: boolean) => void;
    select: (reasoningId: string) => void;
    selectFastMode: (enabled: boolean) => void;
  }

  const props: Props = $props();
  const selected = $derived(
    props.provider.reasoningOptions.find((option) => option.id === props.provider.selectedReasoningId)
      ?? props.provider.reasoningOptions[0],
  );
  const selectedName = $derived(selected?.name ?? 'Reasoning');
  const fastModeEnabled = $derived(props.provider.fastModeEnabled === true);
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLElement>();

  $effect(() => {
    if (!props.open) return;
    void tick().then(() => {
      menu?.querySelector<HTMLButtonElement>('.reasoning-option.selected, .reasoning-option')?.focus();
    });
  });

  function close() {
    props.setOpen(false);
    void tick().then(() => trigger?.focus());
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key) || !menu) return;
    event.preventDefault();
    const items = Array.from(menu.querySelectorAll<HTMLButtonElement>('.reasoning-option:not(:disabled)'));
    const next = nextMenuItemIndex(
      items.indexOf(document.activeElement as HTMLButtonElement),
      items.length,
      event.key as 'ArrowDown' | 'ArrowUp' | 'Home' | 'End',
    );
    items[next]?.focus();
  }
</script>

<div class="reasoning-picker-wrap">
  <button
    bind:this={trigger}
    class="reasoning-picker-trigger"
    aria-label={`Reasoning: ${selectedName}${fastModeEnabled ? ', Fast mode' : ''}`}
    aria-expanded={props.open}
    aria-haspopup="menu"
    aria-controls="reasoning-menu"
    title={`Reasoning: ${selectedName}${fastModeEnabled ? ' · Fast mode' : ''}`}
    disabled={props.provider.turnStatus !== 'idle' || props.provider.connectionStatus === 'connecting'}
    onclick={() => props.setOpen(!props.open)}
  ><span>{selectedName}</span>{#if fastModeEnabled}<IconBolt class="fast-mode-indicator" size={13} stroke={1.9} aria-hidden="true" />{/if}</button>
  {#if props.open}
    <div
      bind:this={menu}
      id="reasoning-menu"
      class="reasoning-menu"
      role="menu"
      tabindex="-1"
      aria-label="Reasoning and speed"
      onkeydown={handleKeydown}
      transition:fly={{ y: props.reducedMotion ? 0 : 4, duration: props.reducedMotion ? 0 : 130 }}
    >
      {#if props.provider.reasoningOptions.length > 0}
        <div class="reasoning-menu-options">
          {#each props.provider.reasoningOptions as option (option.id)}
            {@const isSelected = option.id === selected?.id}
            <button
              class:selected={isSelected}
              class="reasoning-option"
              role="menuitemradio"
              aria-checked={isSelected}
              title={option.description ?? option.name}
              onclick={() => { close(); props.select(option.id); }}
            ><span>{option.name}</span>{#if isSelected}<IconCheck size={14} stroke={2} />{/if}</button>
          {/each}
        </div>
      {/if}
      {#if fastModeAvailable(props.provider)}
        {#if props.provider.reasoningOptions.length > 0}<div class="reasoning-menu-divider"></div>{/if}
        <div class="reasoning-menu-options">
          <span class="reasoning-menu-label">Speed</span>
          <button
            class:selected={!fastModeEnabled}
            class="reasoning-option"
            role="menuitemradio"
            aria-checked={!fastModeEnabled}
            title="Standard speed"
            onclick={() => { close(); props.selectFastMode(false); }}
          ><span>Standard</span>{#if !fastModeEnabled}<IconCheck size={14} stroke={2} />{/if}</button>
          <button
            class:selected={fastModeEnabled}
            class="reasoning-option"
            role="menuitemradio"
            aria-checked={fastModeEnabled}
            title={props.provider.fastModeDescription ?? 'Faster responses at a higher cost.'}
            onclick={() => { close(); props.selectFastMode(true); }}
          ><span>Fast</span>{#if fastModeEnabled}<IconCheck size={14} stroke={2} />{:else}<IconBolt size={14} stroke={1.9} />{/if}</button>
        </div>
      {/if}
    </div>
  {/if}
</div>
