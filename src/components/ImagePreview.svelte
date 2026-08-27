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
    <Dialog.Overlay class="fixed inset-0 z-[90] bg-[var(--overlay-strong)] backdrop-blur-[14px]" />
    <Dialog.Content class="fixed top-1/2 left-1/2 z-[91] grid -translate-x-1/2 -translate-y-1/2 place-items-center bg-transparent p-11 outline-none">
      <Dialog.Title class="absolute size-px overflow-hidden whitespace-nowrap [clip-path:inset(50%)]">{name}</Dialog.Title>
      <button class="absolute top-2 right-2 grid size-[25px] place-items-center rounded-md border border-line bg-floating text-muted shadow-overlay hover:border-line-strong hover:text-ink-soft" type="button" aria-label="Close image preview" title="Close" onclick={close}>
        <IconX size={16} stroke={1.55} />
      </button>
      <img class="block max-h-[calc(100vh_-_88px)] max-w-[calc(100vw_-_88px)] rounded-lg object-contain" {src} alt={name} />
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
