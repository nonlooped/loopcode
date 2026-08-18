<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { IconX } from '@tabler/icons-svelte';

  interface Props {
    src: string;
    name: string;
    reducedMotion: boolean;
    close: () => void;
  }

  const { src, name, reducedMotion, close }: Props = $props();
  let dialog = $state<HTMLElement>();
  let closeButton = $state<HTMLButtonElement>();
  let returnFocus: HTMLElement | null = null;

  onMount(() => {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    void tick().then(() => closeButton?.focus());
  });

  onDestroy(() => returnFocus?.focus());

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== 'Tab' || !dialog) return;
    const items = Array.from(dialog.querySelectorAll<HTMLElement>('button:not(:disabled)'));
    const first = items[0];
    const last = items.at(-1);
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  }
</script>

<div bind:this={dialog} class="image-preview" role="dialog" aria-modal="true" aria-label={name} tabindex="-1" onkeydown={handleKeydown}>
  <button
    class="image-preview-backdrop"
    aria-label="Close image preview"
    onclick={close}
    transition:fade|global={{ duration: reducedMotion ? 0 : 150 }}
  ></button>
  <figure transition:scale|global={{ start: reducedMotion ? 1 : 0.985, duration: reducedMotion ? 0 : 170 }}>
    <img {src} alt={name} />
    <button bind:this={closeButton} class="image-preview-close" aria-label="Close image preview" onclick={close}><IconX size={16} stroke={1.8} /></button>
    <figcaption>{name}</figcaption>
  </figure>
</div>
