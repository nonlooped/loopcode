<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import { IconBolt, IconCheck } from '@tabler/icons-svelte';

  import type { ProviderSessionState } from '../types';
  import { fastModeAvailable } from '../utils/fast-mode';
  import MotionScaleIn from './motion/MotionScaleIn.svelte';

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

  const trigger =
    'flex h-7 max-w-[210px] items-center gap-1 rounded-[7px] border border-transparent bg-transparent px-[7px] text-[11px] font-medium text-muted enabled:hover:border-line enabled:hover:bg-panel enabled:hover:text-text-soft aria-expanded:border-line aria-expanded:bg-panel aria-expanded:text-text-soft disabled:cursor-default disabled:opacity-[0.46] [&_span]:min-w-0 [&_span]:truncate [&_svg]:shrink-0 [&_svg]:text-muted';
  const menuShell =
    'w-60 max-w-[calc(100vw-42px)] overflow-hidden rounded-overlay border border-line bg-floating p-1 text-left whitespace-normal text-text shadow-overlay';
  const menuLabel =
    'px-2 pt-[5px] pb-[3px] text-[11px] font-semibold tracking-wide text-muted uppercase';
  const optionClass =
    'flex min-h-8 w-full cursor-pointer items-center gap-2.5 rounded-lg px-2 py-1.5 text-[13px] text-[color-mix(in_srgb,var(--text)_90%,transparent)] outline-none data-[highlighted]:bg-panel-active data-[highlighted]:text-text hover:bg-panel-active hover:text-text focus-visible:bg-panel-active focus-visible:text-text [&_span]:min-w-0 [&_span]:flex-1 [&_span]:truncate [&_svg]:shrink-0 [&_svg]:text-text-soft';
</script>

<DropdownMenu.Root open={props.open} onOpenChange={props.setOpen}>
  <DropdownMenu.Trigger
    class={trigger}
    disabled={props.provider.turnStatus !== 'idle' || props.provider.connectionStatus === 'connecting'}
    aria-label={`Reasoning: ${selectedName}${fastModeEnabled ? ', Fast mode' : ''}`}
    title={`Reasoning: ${selectedName}${fastModeEnabled ? ' · Fast mode' : ''}`}
  >
    <span>{selectedName}</span>
    {#if fastModeEnabled}<IconBolt size={13} stroke={1.55} aria-hidden="true" />{/if}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <MotionScaleIn duration={130}>
      <DropdownMenu.Content
        class={menuShell}
        side="top"
        align="end"
        sideOffset={10}
        collisionPadding={12}
        aria-label="Reasoning and speed"
      >
        {#if props.provider.reasoningOptions.length > 0}
          <DropdownMenu.Group>
            <DropdownMenu.GroupHeading class={menuLabel}>Reasoning</DropdownMenu.GroupHeading>
            <DropdownMenu.RadioGroup value={selected?.id} onValueChange={props.select}>
              <div class="flex flex-col gap-0.5 py-0.5">
                {#each props.provider.reasoningOptions as reasoningOption (reasoningOption.id)}
                  <DropdownMenu.RadioItem
                    class={optionClass}
                    value={reasoningOption.id}
                    title={reasoningOption.description ?? reasoningOption.name}
                  >
                    {#snippet children({ checked })}
                      <span>{reasoningOption.name}</span>
                      {#if checked}<IconCheck size={14} stroke={2} />{/if}
                    {/snippet}
                  </DropdownMenu.RadioItem>
                {/each}
              </div>
            </DropdownMenu.RadioGroup>
          </DropdownMenu.Group>
        {/if}
        {#if fastModeAvailable(props.provider)}
          <DropdownMenu.Separator class="mx-[3px] my-1 h-px bg-line" />
          <DropdownMenu.Group>
            <DropdownMenu.GroupHeading class={menuLabel}>Speed</DropdownMenu.GroupHeading>
            <div class="flex flex-col gap-0.5 py-0.5">
              <DropdownMenu.RadioGroup
                value={fastModeEnabled ? 'fast' : 'standard'}
                onValueChange={(value) => props.selectFastMode(value === 'fast')}
              >
                <DropdownMenu.RadioItem class={optionClass} value="standard" title="Standard speed">
                  {#snippet children({ checked })}
                    <span>Standard</span>
                    {#if checked}<IconCheck size={14} stroke={2} />{/if}
                  {/snippet}
                </DropdownMenu.RadioItem>
                <DropdownMenu.RadioItem
                  class={optionClass}
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
    </MotionScaleIn>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
