<script lang="ts">
  import { fade, scale } from 'svelte/transition';

  interface Props {
    title: string;
    reducedMotion: boolean;
    cancel: () => void;
    confirm: () => void;
  }

  const { title, reducedMotion, cancel, confirm }: Props = $props();
  let dialog = $state<HTMLDialogElement>();
  let returnFocus: HTMLElement | null = null;

  $effect(() => {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog?.showModal();
    return () => returnFocus?.focus();
  });

  function handleCancel(event: Event) {
    event.preventDefault();
    cancel();
  }
</script>

<dialog
  bind:this={dialog}
  class="permission-modal confirmation-modal"
  role="alertdialog"
  aria-labelledby="delete-thread-title"
  aria-describedby="delete-thread-description"
  transition:scale|global={{ start: reducedMotion ? 1 : 0.985, duration: reducedMotion ? 0 : 170 }}
  oncancel={handleCancel}
>
  <header class="permission-header">
    <h2 id="delete-thread-title">Delete thread?</h2>
    <p id="delete-thread-description">“{title}” and its history will be permanently deleted.</p>
  </header>
  <footer class="confirmation-actions">
    <!-- svelte-ignore a11y_autofocus --><!-- Modal autofocus is the spec-recommended way to move focus into a showModal() dialog. -->
    <button data-primary="true" autofocus onclick={cancel}>Cancel</button>
    <button class="danger" onclick={confirm}>Delete</button>
  </footer>
</dialog>
