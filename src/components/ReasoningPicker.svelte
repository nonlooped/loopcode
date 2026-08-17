<script lang="ts">
  import { fade } from 'svelte/transition';
  import { IconBolt, IconCheck } from '@tabler/icons-svelte';

  import type { ProviderSessionState } from '../types';
  import { fastModeAvailable } from '../utils/fast-mode';

  interface Props {
    provider: ProviderSessionState;
    open: boolean;
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
</script>

<div class="reasoning-picker-wrap">
  <button
    class="reasoning-picker-trigger"
    aria-label={`Reasoning: ${selectedName}${fastModeEnabled ? ', Fast mode' : ''}`}
    aria-expanded={props.open}
    aria-haspopup="menu"
    title={`Reasoning: ${selectedName}${fastModeEnabled ? ' · Fast mode' : ''}`}
    disabled={props.provider.status === 'running' || props.provider.status === 'connecting'}
    onclick={() => props.setOpen(!props.open)}
  ><span>{selectedName}</span>{#if fastModeEnabled}<IconBolt class="fast-mode-indicator" size={13} stroke={1.9} aria-hidden="true" />{/if}</button>
  {#if props.open}
    <div class="reasoning-menu" role="menu" aria-label="Reasoning and speed" transition:fade={{ duration: 110 }}>
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
              onclick={() => { props.setOpen(false); props.select(option.id); }}
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
            onclick={() => { props.setOpen(false); props.selectFastMode(false); }}
          ><span>Standard</span>{#if !fastModeEnabled}<IconCheck size={14} stroke={2} />{/if}</button>
          <button
            class:selected={fastModeEnabled}
            class="reasoning-option"
            role="menuitemradio"
            aria-checked={fastModeEnabled}
            title={props.provider.fastModeDescription ?? 'Faster responses at a higher cost.'}
            onclick={() => { props.setOpen(false); props.selectFastMode(true); }}
          ><span>Fast</span>{#if fastModeEnabled}<IconCheck size={14} stroke={2} />{:else}<IconBolt size={14} stroke={1.9} />{/if}</button>
        </div>
      {/if}
    </div>
  {/if}
</div>
