<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';

  interface Props {
    title: string;
    reducedMotion: boolean;
    cancel: () => void;
    confirm: () => void;
  }

  const { title, reducedMotion, cancel, confirm }: Props = $props();
  let dialog = $state<HTMLDivElement>();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      cancel();
      return;
    }

    if (event.key !== 'Tab' || !dialog) return;
    const controls = [...dialog.querySelectorAll<HTMLButtonElement>('button')];
    const first = controls[0];
    const last = controls.at(-1);
    if (!first || !last) return;

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  onMount(() => dialog?.querySelector<HTMLButtonElement>('[data-primary="true"]')?.focus());
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-backdrop" role="presentation" transition:fade|global={{ duration: reducedMotion ? 0 : 150 }}>
  <div
    bind:this={dialog}
    class="permission-modal confirmation-modal"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="delete-thread-title"
    aria-describedby="delete-thread-description"
    tabindex="-1"
    transition:scale|global={{ start: reducedMotion ? 1 : 0.985, duration: reducedMotion ? 0 : 170 }}
  >
    <header class="permission-header">
      <h2 id="delete-thread-title">Delete thread?</h2>
      <p id="delete-thread-description">“{title}” and its history will be permanently deleted.</p>
    </header>
    <footer class="confirmation-actions">
      <button data-primary="true" onclick={cancel}>Cancel</button>
      <button class="danger" onclick={confirm}>Delete</button>
    </footer>
  </div>
</div>
