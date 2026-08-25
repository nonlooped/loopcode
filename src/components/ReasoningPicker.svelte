<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
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

<DropdownMenu.Root open={props.open} onOpenChange={props.setOpen}>
  <DropdownMenu.Trigger
    class="reasoning-picker-trigger"
    disabled={props.provider.turnStatus !== 'idle' || props.provider.connectionStatus === 'connecting'}
    aria-label={`Reasoning: ${selectedName}${fastModeEnabled ? ', Fast mode' : ''}`}
    title={`Reasoning: ${selectedName}${fastModeEnabled ? ' · Fast mode' : ''}`}
  >
    <span>{selectedName}</span>
    {#if fastModeEnabled}<IconBolt class="fast-mode-indicator" size={13} stroke={1.55} aria-hidden="true" />{/if}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content
      class="reasoning-menu"
      side="top"
      align="end"
      sideOffset={10}
      collisionPadding={12}
      aria-label="Reasoning and speed"
    >
      {#if props.provider.reasoningOptions.length > 0}
        <DropdownMenu.RadioGroup value={selected?.id} onValueChange={props.select}>
          <div class="reasoning-menu-options">
            {#each props.provider.reasoningOptions as option (option.id)}
              <DropdownMenu.RadioItem
                class="reasoning-option"
                value={option.id}
                title={option.description ?? option.name}
              >
                {#snippet children({ checked })}
                  <span>{option.name}</span>
                  {#if checked}<IconCheck size={14} stroke={2} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
            {/each}
          </div>
        </DropdownMenu.RadioGroup>
      {/if}
      {#if fastModeAvailable(props.provider)}
        <DropdownMenu.Separator class="reasoning-menu-divider" />
        <DropdownMenu.Group>
          <DropdownMenu.GroupHeading class="reasoning-menu-label">Speed</DropdownMenu.GroupHeading>
          <div class="reasoning-menu-options">
            <DropdownMenu.RadioGroup
              value={fastModeEnabled ? 'fast' : 'standard'}
              onValueChange={(value) => props.selectFastMode(value === 'fast')}
            >
              <DropdownMenu.RadioItem class="reasoning-option" value="standard" title="Standard speed">
                {#snippet children({ checked })}
                  <span>Standard</span>
                  {#if checked}<IconCheck size={14} stroke={2} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
              <DropdownMenu.RadioItem
                class="reasoning-option"
                value="fast"
                title={props.provider.fastModeDescription ?? 'Faster responses at a higher cost.'}
              >
                {#snippet children({ checked })}
                  <span>Fast</span>
                  {#if checked}<IconCheck size={14} stroke={2} />{:else}<IconBolt size={14} stroke={1.55} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
            </DropdownMenu.RadioGroup>
          </div>
        </DropdownMenu.Group>
      {/if}
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
