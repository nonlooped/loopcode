<script module lang="ts">
  export interface ContextMenuItem {
    label: string;
    action: () => void | Promise<void>;
    danger?: boolean;
    disabled?: boolean;
    separatorBefore?: boolean;
    shortcut?: string;
  }
</script>

<script lang="ts">
  import { ContextMenu, type ContextMenuTriggerProps } from 'bits-ui';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  type TriggerRenderProps = Parameters<NonNullable<ContextMenuTriggerProps['child']>>[0];

  interface Props {
    items: ContextMenuItem[];
    children: Snippet<[TriggerRenderProps]>;
  }

  const { items, children }: Props = $props();

  const menuShell =
    'z-[91] grid w-max min-w-[186px] max-w-[min(280px,calc(100vw-16px))] rounded-overlay border border-line-strong bg-floating p-[5px] shadow-overlay backdrop-blur-[20px] backdrop-saturate-[115%]';
  const menuItem =
    'flex min-h-[29px] w-full items-center justify-between gap-[22px] rounded-md px-2 text-left text-xs whitespace-nowrap text-text-soft outline-0 data-[highlighted]:bg-panel-active data-[highlighted]:text-text hover:bg-panel-active hover:text-text data-[disabled]:opacity-[0.42]';
  const menuItemDanger =
    'text-[color-mix(in_srgb,var(--danger)_88%,var(--text-soft))] data-[highlighted]:bg-[color-mix(in_srgb,var(--danger)_12%,transparent)] hover:bg-[color-mix(in_srgb,var(--danger)_12%,transparent)]';

  const menuOpacity = new Tween(0, { duration: 0, easing: cubicOut });
  const menuScale = new Tween(0.97, { duration: 0, easing: cubicOut });
  const menuY = new Tween(-2, { duration: 0, easing: cubicOut });

  $effect(() => {
    void menuOpacity.set(1, { duration: 90 });
    void menuScale.set(1, { duration: 90 });
    void menuY.set(0, { duration: 90 });
  });
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger>
    {#snippet child({ props })}
      {@render children({ props })}
    {/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content
      class={menuShell}
      style={`opacity: ${menuOpacity.current}; transform: scale(${menuScale.current}) translateY(${menuY.current}px); transform-origin: top left`}
      collisionPadding={8}
    >
        {#each items as item}
          {#if item.separatorBefore}<ContextMenu.Separator class="mx-[5px] my-1 h-px bg-line" />{/if}
          <ContextMenu.Item
            class="{menuItem} {item.danger ? menuItemDanger : ''}"
            disabled={item.disabled}
            textValue={item.label}
            onSelect={() => void item.action()}
          >
            <span>{item.label}</span>
            {#if item.shortcut}<kbd class="text-[11px] text-faint">{item.shortcut}</kbd>{/if}
          </ContextMenu.Item>
        {/each}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
