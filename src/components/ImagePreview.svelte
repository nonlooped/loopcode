<script lang="ts">
  import { IconX } from '@tabler/icons-svelte';
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
      <button class="chrome-button image-preview-close" type="button" aria-label="Close image preview" title="Close" onclick={close}>
        <IconX size={16} stroke={1.55} />
      </button>
      <img {src} alt={name} />
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
