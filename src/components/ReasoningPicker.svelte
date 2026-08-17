<script lang="ts">
  import { fade } from 'svelte/transition';
  import { IconCheck } from '@tabler/icons-svelte';

  import type { ProviderSessionState } from '../types';

  interface Props {
    provider: ProviderSessionState;
    open: boolean;
    setOpen: (open: boolean) => void;
    select: (reasoningId: string) => void;
  }

  const props: Props = $props();
  const selected = $derived(
    props.provider.reasoningOptions.find((option) => option.id === props.provider.selectedReasoningId)
      ?? props.provider.reasoningOptions[0],
  );
</script>

<div class="reasoning-picker-wrap">
  <button
    class="reasoning-picker-trigger"
    aria-label={`Reasoning: ${selected.name}`}
    aria-expanded={props.open}
    aria-haspopup="menu"
    title={`Reasoning: ${selected.name}`}
    disabled={props.provider.status === 'running' || props.provider.status === 'connecting'}
    onclick={() => props.setOpen(!props.open)}
  ><span>{selected.name}</span></button>
  {#if props.open}
    <div class="reasoning-menu" role="menu" aria-label="Reasoning" transition:fade={{ duration: 110 }}>
      <div class="reasoning-menu-options">
        {#each props.provider.reasoningOptions as option (option.id)}
          {@const isSelected = option.id === selected.id}
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
    </div>
  {/if}
</div>
