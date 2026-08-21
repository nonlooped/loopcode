<script lang="ts">
  import { AlertDialog } from 'bits-ui';

  interface Props {
    title: string;
    cancel: () => void;
    confirm: () => void;
  }

  const { title, cancel, confirm }: Props = $props();

  $effect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => previous?.focus();
  });
</script>

<AlertDialog.Root open onOpenChange={(open) => { if (!open) cancel(); }}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="modal-overlay" />
    <AlertDialog.Content class="permission-modal confirmation-modal">
      <AlertDialog.Title>Delete thread?</AlertDialog.Title>
      <AlertDialog.Description>“{title}” and its history will be permanently deleted.</AlertDialog.Description>
      <footer class="confirmation-actions">
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button class="danger" onclick={confirm}>Delete</button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
