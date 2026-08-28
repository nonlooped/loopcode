<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import { IconCheck, IconListCheck } from '@tabler/icons-svelte';

  import type { ProviderSessionState } from '../types';
  import type { SessionSelectId } from '../services/provider-runtime';
  import MotionScaleIn from './motion/MotionScaleIn.svelte';

  interface Props {
    provider: ProviderSessionState;
    open: boolean;
    setOpen: (open: boolean) => void;
    select: (option: SessionSelectId, valueId: string) => void;
  }

  const props: Props = $props();
  const modes = $derived(props.provider.modes ?? []);
  const collaborationModes = $derived(props.provider.collaborationModes ?? []);
  const selected = $derived(
    modes.find((option) => option.id === props.provider.selectedModeId) ?? modes[0],
  );
  const selectedName = $derived(selected?.name ?? 'Mode');
  const collaboration = $derived(
    collaborationModes.find((option) => option.id === props.provider.selectedCollaborationModeId),
  );
  // "Default" is the resting state; only a deliberate mode is worth marking on the trigger.
  const planning = $derived(Boolean(collaboration) && collaboration?.id !== 'default');

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
    disabled={props.provider.turnStatus !== 'idle' ||
      props.provider.connectionStatus === 'connecting'}
    aria-label={`Mode: ${selectedName}${planning ? `, ${collaboration?.name}` : ''}`}
    title={`Mode: ${selectedName}${planning ? ` · ${collaboration?.name}` : ''}`}
  >
    <span>{selectedName}</span>
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
        aria-label="Approval mode"
      >
        {#if modes.length > 0}
          <DropdownMenu.Group>
            <DropdownMenu.GroupHeading class={menuLabel}>Mode</DropdownMenu.GroupHeading>
            <DropdownMenu.RadioGroup
              value={selected?.id}
              onValueChange={(value) => props.select('mode', value)}
            >
              <div class="flex flex-col gap-0.5 py-0.5">
                {#each modes as mode (mode.id)}
                  <DropdownMenu.RadioItem
                    class={optionClass}
                    value={mode.id}
                    title={mode.description ?? mode.name}
                  >
                    {#snippet children({ checked })}
                      <span>{mode.name}</span>
                      {#if checked}<IconCheck size={14} stroke={2} />{/if}
                    {/snippet}
                  </DropdownMenu.RadioItem>
                {/each}
              </div>
            </DropdownMenu.RadioGroup>
          </DropdownMenu.Group>
        {/if}
        {#if collaborationModes.length > 0}
          {#if modes.length > 0}
            <DropdownMenu.Separator class="mx-[3px] my-1 h-px bg-line" />
          {/if}
          <DropdownMenu.Group>
            <DropdownMenu.GroupHeading class={menuLabel}>Collaboration</DropdownMenu.GroupHeading>
            <DropdownMenu.RadioGroup
              value={collaboration?.id}
              onValueChange={(value) => props.select('collaboration', value)}
            >
              <div class="flex flex-col gap-0.5 py-0.5">
                {#each collaborationModes as mode (mode.id)}
                  <DropdownMenu.RadioItem
                    class={optionClass}
                    value={mode.id}
                    title={mode.description ?? mode.name}
                  >
                    {#snippet children({ checked })}
                      <span>{mode.name}</span>
                      {#if checked}<IconCheck size={14} stroke={2} />{/if}
                    {/snippet}
                  </DropdownMenu.RadioItem>
                {/each}
              </div>
            </DropdownMenu.RadioGroup>
          </DropdownMenu.Group>
        {/if}
      </DropdownMenu.Content>
    </MotionScaleIn>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
