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
    <Dialog.Overlay class="fixed inset-0 z-[90] bg-overlay backdrop-blur-[3px]" />
    <Dialog.Content
      class="fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw_-_40px))] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-line-strong bg-[var(--decision-surface)] p-[18px] shadow-overlay backdrop-blur-xl [&>h2]:text-base [&>h2]:font-semibold [&>p]:mt-1 [&>p]:text-xs [&>p]:text-muted"
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        inputElement?.focus();
      }}
    >
      <Dialog.Title>Rename thread</Dialog.Title>
      <Dialog.Description>Choose a name for this thread.</Dialog.Description>
      <form onsubmit={(event) => { event.preventDefault(); submit(); }}>
        <label class="mt-3.5 grid gap-1 text-[11px] text-muted">
          <span>Thread name</span>
          <input class="min-w-0 rounded-md border border-line bg-recessed px-2 py-2 text-ink outline-none focus-visible:border-line-strong focus-visible:ring-2 focus-visible:ring-[var(--focus-ring)]" bind:this={inputElement} bind:value={input} />
        </label>
        <footer class="mt-[18px] flex justify-end gap-2 [&_button]:rounded-md [&_button]:border [&_button]:border-line [&_button]:px-3 [&_button]:py-1.5 [&_button]:text-ink-soft [&_button:hover]:border-line-strong [&_button:hover]:bg-panel-hover [&_button:hover]:text-ink">
          <button type="button" onclick={cancel}>Cancel</button>
          <button type="submit" disabled={!input.trim()}>Save</button>
        </footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
