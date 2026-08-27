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
  import type { Snippet } from 'svelte';

  type TriggerRenderProps = Parameters<NonNullable<ContextMenuTriggerProps['child']>>[0];

  interface Props {
    items: ContextMenuItem[];
    children: Snippet<[TriggerRenderProps]>;
  }

  const { items, children }: Props = $props();
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger>
    {#snippet child({ props })}
      {@render children({ props })}
    {/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="z-[91] grid min-w-[186px] max-w-[min(280px,calc(100vw_-_16px))] w-max rounded-xl border border-line-strong bg-floating p-1.5 shadow-overlay backdrop-blur-xl backdrop-saturate-[115%] [transform-origin:top_left]" collisionPadding={8}>
      {#each items as item}
        {#if item.separatorBefore}<ContextMenu.Separator class="mx-1 my-1 h-px bg-line" />{/if}
        <ContextMenu.Item
          class={`flex min-h-[29px] w-full items-center justify-between gap-5 rounded-md px-2 text-left text-xs whitespace-nowrap text-ink-soft outline-none data-[highlighted]:bg-panel-active data-[highlighted]:text-ink data-[disabled]:opacity-40 ${item.danger ? 'text-[color-mix(in_srgb,var(--danger)_88%,var(--text-soft))] data-[highlighted]:bg-[color-mix(in_srgb,var(--danger)_12%,transparent)]' : ''}`}
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
