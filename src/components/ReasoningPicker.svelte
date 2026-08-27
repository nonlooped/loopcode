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
    class="flex h-7 max-w-[210px] items-center gap-1.5 rounded-md border border-transparent px-2 text-[11px] font-medium text-muted hover:border-line hover:bg-panel hover:text-ink-soft aria-expanded:border-line aria-expanded:bg-panel aria-expanded:text-ink-soft disabled:cursor-default disabled:opacity-45 [&>span]:min-w-0 [&>span]:truncate [&>svg]:shrink-0 [&>svg]:text-muted"
    disabled={props.provider.turnStatus !== 'idle' || props.provider.connectionStatus === 'connecting'}
    aria-label={`Reasoning: ${selectedName}${fastModeEnabled ? ', Fast mode' : ''}`}
    title={`Reasoning: ${selectedName}${fastModeEnabled ? ' · Fast mode' : ''}`}
  >
    <span>{selectedName}</span>
    {#if fastModeEnabled}<IconBolt class="fast-mode-indicator" size={13} stroke={1.55} aria-hidden="true" />{/if}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content
      class="w-60 max-w-[calc(100vw_-_42px)] overflow-hidden rounded-xl border border-line bg-floating p-1 shadow-overlay text-left text-ink whitespace-normal backdrop-blur-xl"
      side="top"
      align="end"
      sideOffset={10}
      collisionPadding={12}
      aria-label="Reasoning and speed"
    >
      {#if props.provider.reasoningOptions.length > 0}
        <DropdownMenu.Group>
          <DropdownMenu.GroupHeading class="px-2 py-1 text-[11px] font-semibold tracking-[.04em] text-muted uppercase">Reasoning</DropdownMenu.GroupHeading>
          <DropdownMenu.RadioGroup value={selected?.id} onValueChange={props.select}>
            <div class="flex flex-col gap-0.5 py-0.5">
              {#each props.provider.reasoningOptions as option (option.id)}
                <DropdownMenu.RadioItem
                  class="flex min-h-8 w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-[13px] text-[color-mix(in_srgb,var(--text)_90%,transparent)] outline-none hover:bg-panel-active hover:text-ink data-[highlighted]:bg-panel-active data-[highlighted]:text-ink [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate [&>svg]:shrink-0 [&>svg]:text-ink-soft"
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
        </DropdownMenu.Group>
      {/if}
      {#if fastModeAvailable(props.provider)}
        <DropdownMenu.Separator class="mx-1 my-1 h-px bg-line" />
        <DropdownMenu.Group>
          <DropdownMenu.GroupHeading class="px-2 py-1 text-[11px] font-semibold tracking-[.04em] text-muted uppercase">Speed</DropdownMenu.GroupHeading>
          <div class="flex flex-col gap-0.5 py-0.5">
            <DropdownMenu.RadioGroup
              value={fastModeEnabled ? 'fast' : 'standard'}
              onValueChange={(value) => props.selectFastMode(value === 'fast')}
            >
              <DropdownMenu.RadioItem class="flex min-h-8 w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-[13px] text-[color-mix(in_srgb,var(--text)_90%,transparent)] outline-none hover:bg-panel-active hover:text-ink data-[highlighted]:bg-panel-active data-[highlighted]:text-ink [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate [&>svg]:shrink-0 [&>svg]:text-ink-soft" value="standard" title="Standard speed">
                {#snippet children({ checked })}
                  <span>Standard</span>
                  {#if checked}<IconCheck size={14} stroke={2} />{/if}
                {/snippet}
              </DropdownMenu.RadioItem>
              <DropdownMenu.RadioItem
                class="flex min-h-8 w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-[13px] text-[color-mix(in_srgb,var(--text)_90%,transparent)] outline-none hover:bg-panel-active hover:text-ink data-[highlighted]:bg-panel-active data-[highlighted]:text-ink [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate [&>svg]:shrink-0 [&>svg]:text-ink-soft"
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
