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
    <AlertDialog.Content class="fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw_-_40px))] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-line-strong bg-[var(--decision-surface)] p-[18px] shadow-overlay backdrop-blur-xl [&>h2]:text-base [&>h2]:font-semibold [&>p]:mt-1 [&>p]:text-xs [&>p]:text-muted">
      <AlertDialog.Title>Delete thread?</AlertDialog.Title>
      <AlertDialog.Description>“{title}” and its history will be permanently deleted.</AlertDialog.Description>
      <footer class="mt-[18px] flex justify-end gap-2 [&_button]:rounded-md [&_button]:border [&_button]:border-line [&_button]:px-3 [&_button]:py-1.5 [&_button]:text-ink-soft [&_button:hover]:border-line-strong [&_button:hover]:bg-panel-hover [&_button:hover]:text-ink">
        <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
        <button class="!border-[color-mix(in_srgb,var(--danger)_30%,transparent)] !text-danger" onclick={confirm}>Delete</button>
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
