<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import { IconBolt, IconCheck, IconChevronRight, IconListCheck } from '@tabler/icons-svelte';

  import type { ModelOption, ProviderSessionState, SessionSelectId } from '../types';
  import { fastModeAvailable } from '../utils/fast-mode';
  import { sessionSelectLabels } from '../utils/model-state';
  import MotionScaleIn from './motion/MotionScaleIn.svelte';

  interface Props {
    provider: ProviderSessionState;
    open: boolean;
    setOpen: (open: boolean) => void;
    selectReasoning: (reasoningId: string) => void;
    selectFastMode: (enabled: boolean) => void;
    selectSessionOption: (option: SessionSelectId, valueId: string) => void;
  }

  const props: Props = $props();

  function chosen(id: SessionSelectId, fallbackToFirst = true) {
    const select = props.provider.selects?.[id];
    if (!select) return undefined;
    return select.options.find((option) => option.id === select.selectedId)
      ?? (fallbackToFirst ? select.options[0] : undefined);
  }

  const reasoning = $derived(
    props.provider.reasoningOptions.find((option) => option.id === props.provider.selectedReasoningId)
      ?? props.provider.reasoningOptions[0],
  );
  const mode = $derived(chosen('mode'));
  const collaboration = $derived(chosen('collaboration', false));
  const agent = $derived(chosen('agent'));
  const fastModeEnabled = $derived(props.provider.fastModeEnabled === true);
  const planning = $derived(Boolean(collaboration) && collaboration?.id !== 'default');
  const speedOptions = $derived<ModelOption[]>([
    { id: 'standard', name: 'Standard', description: 'Standard speed' },
    { id: 'fast', name: 'Fast', description: props.provider.fastModeDescription ?? 'Faster responses at a higher cost.' },
  ]);
  const label = $derived(
    [
      reasoning?.name,
      agent?.id === 'default' ? undefined : agent?.name,
      mode?.name === 'Agent (full access)' ? 'Full access' : mode?.name,
    ]
      .filter(Boolean)
      .join(' · ') || 'Session',
  );
  const title = $derived(
    [
      reasoning ? `Reasoning: ${reasoning.name}` : '',
      fastModeAvailable(props.provider) ? `Speed: ${fastModeEnabled ? 'Fast' : 'Standard'}` : '',
      mode ? `Mode: ${mode.name}` : '',
      collaboration ? `Collaboration: ${collaboration.name}` : '',
      agent ? `Agent: ${agent.name}` : '',
    ]
      .filter(Boolean)
      .join(' · '),
  );

  const trigger =
    'flex h-7 max-w-[210px] items-center gap-1 rounded-[7px] border border-transparent bg-transparent px-[7px] text-[11px] font-medium text-muted enabled:hover:border-line enabled:hover:bg-panel enabled:hover:text-text-soft aria-expanded:border-line aria-expanded:bg-panel aria-expanded:text-text-soft disabled:cursor-default disabled:opacity-[0.46] [&_span]:min-w-0 [&_span]:truncate [&_svg]:shrink-0 [&_svg]:text-muted';
  const menuShell =
    'w-60 max-w-[calc(100vw-42px)] overflow-hidden rounded-overlay border border-line bg-floating p-1 text-left whitespace-normal text-text shadow-overlay';
  const submenuTrigger =
    'grid min-h-9 w-full grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2 rounded-lg px-2 text-[13px] text-text-soft outline-none data-[highlighted]:bg-panel-active data-[state=open]:bg-panel-active [&_small]:max-w-28 [&_small]:truncate [&_small]:text-[11px] [&_small]:text-muted [&_svg]:shrink-0 [&_svg]:text-muted';
  const optionClass =
    'flex min-h-8 w-full cursor-pointer items-center gap-2.5 rounded-lg px-2 py-1.5 text-[13px] text-[color-mix(in_srgb,var(--text)_90%,transparent)] outline-none data-[highlighted]:bg-panel-active data-[highlighted]:text-text hover:bg-panel-active hover:text-text focus-visible:bg-panel-active focus-visible:text-text [&_span]:min-w-0 [&_span]:flex-1 [&_span]:truncate [&_svg]:shrink-0 [&_svg]:text-text-soft';
</script>

{#snippet submenu(name: string, options: ModelOption[], selectedId: string | undefined, choose: (value: string) => void)}
  <DropdownMenu.Sub>
    <DropdownMenu.SubTrigger class={submenuTrigger}>
      <span>{name}</span>
      <small>{options.find((option) => option.id === selectedId)?.name}</small>
      <IconChevronRight size={13} stroke={1.55} />
    </DropdownMenu.SubTrigger>
    <DropdownMenu.SubContent class={menuShell} sideOffset={6} collisionPadding={12}>
      <DropdownMenu.RadioGroup value={selectedId} onValueChange={choose}>
        {#each options as option (option.id)}
          <DropdownMenu.RadioItem class={optionClass} value={option.id} title={option.description ?? option.name}>
            {#snippet children({ checked })}
              <span>{option.name}</span>
              {#if checked}<IconCheck size={14} stroke={2} />{/if}
            {/snippet}
          </DropdownMenu.RadioItem>
        {/each}
      </DropdownMenu.RadioGroup>
    </DropdownMenu.SubContent>
  </DropdownMenu.Sub>
{/snippet}

<DropdownMenu.Root open={props.open} onOpenChange={props.setOpen}>
  <DropdownMenu.Trigger
    class={trigger}
    disabled={props.provider.turnStatus !== 'idle' || props.provider.connectionStatus === 'connecting'}
    aria-label={`Session settings: ${title}`}
    {title}
  >
    <span>{label}</span>
    {#if fastModeEnabled}<IconBolt size={13} stroke={1.55} aria-hidden="true" />{/if}
    {#if planning}<IconListCheck size={13} stroke={1.55} aria-hidden="true" />{/if}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <MotionScaleIn duration={130}>
      <DropdownMenu.Content
        class={menuShell}
        side="top"
        align="end"
        sideOffset={10}
        collisionPadding={12}
        aria-label="Session settings"
      >
        {#if props.provider.reasoningOptions.length > 1}
          {@render submenu('Reasoning', props.provider.reasoningOptions, reasoning?.id, props.selectReasoning)}
        {/if}
        {#if fastModeAvailable(props.provider)}
          {@render submenu('Speed', speedOptions, fastModeEnabled ? 'fast' : 'standard', (value) => props.selectFastMode(value === 'fast'))}
        {/if}
        {#each Object.entries(props.provider.selects ?? {}) as [id, select] (id)}
          {#if select.options.length > 1}
            {@render submenu(
              sessionSelectLabels[id as SessionSelectId],
              select.options,
              chosen(id as SessionSelectId)?.id,
              (value) => props.selectSessionOption(id as SessionSelectId, value),
            )}
          {/if}
        {/each}
      </DropdownMenu.Content>
    </MotionScaleIn>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
