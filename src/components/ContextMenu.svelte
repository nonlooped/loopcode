<script lang="ts">
  import { tick } from 'svelte';

  import type { ContextMenuState } from '../utils/context-menu';
  import { contextMenuPosition } from '../utils/context-menu';

  interface Props {
    menu: ContextMenuState;
    close: () => void;
  }

  const { menu, close }: Props = $props();
  let element = $state<HTMLDivElement>();
  let left = $state(0);
  let top = $state(0);

  $effect(() => {
    void tick().then(() => {
      if (!element) return;
      const position = contextMenuPosition(
        menu.x,
        menu.y,
        element.offsetWidth,
        element.offsetHeight,
        window.innerWidth,
        window.innerHeight,
      );
      left = position.x;
      top = position.y;
      element.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    });
  });

  function choose(action: () => void | Promise<void>) {
    close();
    void action();
  }

  function closeOutside(event: Event) {
    if (!element?.contains(event.target as Node)) close();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    const buttons = Array.from(element?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? []);
    if (buttons.length === 0) return;
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const step = event.key === 'ArrowDown' ? 1 : -1;
    buttons[(current + step + buttons.length) % buttons.length].focus();
  }
</script>

<svelte:window onpointerdown={closeOutside} oncontextmenu={closeOutside} onblur={close} />

<div
  bind:this={element}
  class="context-menu"
  role="menu"
  tabindex="-1"
  style={`left: ${left}px; top: ${top}px`}
  onkeydown={handleKeydown}
  oncontextmenu={(event) => event.preventDefault()}
>
  {#each menu.items as item}
    {#if item.separatorBefore}<div class="context-menu-separator" role="separator"></div>{/if}
    <button
      type="button"
      role="menuitem"
      class:danger={item.danger}
      disabled={item.disabled}
      onclick={() => choose(item.action)}
    >
      <span>{item.label}</span>
      {#if item.shortcut}<kbd>{item.shortcut}</kbd>{/if}
    </button>
  {/each}
</div>
