<script lang="ts">
  import { Dialog } from 'bits-ui';

  interface Props {
    title: string;
    cancel: () => void;
    save: (title: string) => void;
  }

  const { title, cancel, save }: Props = $props();
  let input = $state('');
  let inputElement = $state<HTMLInputElement>();

  $effect(() => {
    input = title;
  });

  $effect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => previous?.focus();
  });

  function submit() {
    const trimmed = input.trim();
    if (trimmed) save(trimmed);
  }
</script>

<Dialog.Root open onOpenChange={(open) => { if (!open) cancel(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="modal-overlay" />
    <Dialog.Content
      class="permission-modal confirmation-modal rename-thread-modal"
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        inputElement?.focus();
      }}
    >
      <Dialog.Title>Rename thread</Dialog.Title>
      <Dialog.Description>Choose a name for this thread.</Dialog.Description>
      <form onsubmit={(event) => { event.preventDefault(); submit(); }}>
        <label class="rename-thread-field">
          <span>Thread name</span>
          <input bind:this={inputElement} bind:value={input} />
        </label>
        <footer class="confirmation-actions">
          <button type="button" onclick={cancel}>Cancel</button>
          <button type="submit" disabled={!input.trim()}>Save</button>
        </footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
