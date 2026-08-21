<script lang="ts">
  import { Dialog } from 'bits-ui';

  interface Props {
    src: string;
    name: string;
    close: () => void;
  }

  const { src, name, close }: Props = $props();

  $effect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => previous?.focus();
  });
</script>

<Dialog.Root open onOpenChange={(open) => { if (!open) close(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="modal-overlay image-preview-overlay" />
    <Dialog.Content class="image-preview">
      <Dialog.Title class="visually-hidden-title">{name}</Dialog.Title>
      <img {src} alt={name} />
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
