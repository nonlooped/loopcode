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
    <AlertDialog.Overlay class="fixed inset-0 z-[90] bg-overlay backdrop-blur-[3px]" />
    <AlertDialog.Content
      class="fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw-40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-overlay border border-line-strong bg-decision p-[18px] shadow-overlay outline-0 backdrop-blur-[20px] backdrop-saturate-[115%] [&_[role=heading]]:text-[15px] [&_[role=heading]]:leading-snug [&_[role=heading]]:font-semibold [&_[role=heading]]:tracking-tight [&_[role=heading]]:text-text [&_[data-alert-dialog-description]]:mt-[5px] [&_[data-alert-dialog-description]]:text-xs [&_[data-alert-dialog-description]]:leading-snug [&_[data-alert-dialog-description]]:text-muted"
    >
      <AlertDialog.Title>Delete thread?</AlertDialog.Title>
      <AlertDialog.Description>“{title}” and its history will be permanently deleted.</AlertDialog.Description>
      <footer class="mt-[18px] flex flex-wrap justify-end gap-[7px] [&_button]:rounded-[7px] [&_button]:border [&_button]:border-line [&_button]:bg-transparent [&_button]:px-3 [&_button]:py-[7px] [&_button]:text-text-soft hover:[&_button]:border-line-strong hover:[&_button]:bg-panel-hover hover:[&_button]:text-text [&_button.danger]:border-[color-mix(in_srgb,var(--danger)_30%,transparent)] [&_button.danger]:text-danger hover:[&_button.danger]:bg-[color-mix(in_srgb,var(--danger)_12%,transparent)]">
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button class="danger" onclick={confirm}>Delete</button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
