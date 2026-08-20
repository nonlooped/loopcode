<script lang="ts">
  import { fade } from 'svelte/transition';

  interface Props {
    src: string;
    name: string;
    reducedMotion: boolean;
    close: () => void;
  }

  const { src, name, reducedMotion, close }: Props = $props();
  let dialog = $state<HTMLDialogElement>();
  let returnFocus: HTMLElement | null = null;

  $effect(() => {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog?.showModal();
    return () => returnFocus?.focus();
  });

  function handleCancel(event: Event) {
    event.preventDefault();
    close();
  }

  function handleClick(event: MouseEvent) {
    if (event.target === dialog) close();
  }
</script>

<dialog
  bind:this={dialog}
  class="image-preview"
  aria-label={name}
  transition:fade|global={{ duration: reducedMotion ? 0 : 150 }}
  oncancel={handleCancel}
  onclick={handleClick}
>
  <img {src} alt={name} />
</dialog>
