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
    <ContextMenu.Content class="context-menu" collisionPadding={8}>
      {#each items as item}
        {#if item.separatorBefore}<ContextMenu.Separator class="context-menu-separator" />{/if}
        <ContextMenu.Item
          class={item.danger ? 'context-menu-item danger' : 'context-menu-item'}
          disabled={item.disabled}
          textValue={item.label}
          onSelect={() => void item.action()}
        >
          <span>{item.label}</span>
          {#if item.shortcut}<kbd>{item.shortcut}</kbd>{/if}
        </ContextMenu.Item>
      {/each}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
