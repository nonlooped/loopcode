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
      class="fixed top-1/2 left-1/2 z-[91] w-[min(400px,calc(100vw-40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-overlay border border-line-strong bg-decision p-[18px] shadow-overlay outline-0 backdrop-blur-[20px] backdrop-saturate-[115%] [&_[role=heading]]:text-[15px] [&_[role=heading]]:leading-snug [&_[role=heading]]:font-semibold [&_[role=heading]]:tracking-tight [&_[role=heading]]:text-text [&_[data-dialog-description]]:mt-[5px] [&_[data-dialog-description]]:text-xs [&_[data-dialog-description]]:leading-snug [&_[data-dialog-description]]:text-muted"
      onOpenAutoFocus={(event) => {
        event.preventDefault();
        inputElement?.focus();
      }}
    >
      <Dialog.Title>Rename thread</Dialog.Title>
      <Dialog.Description>Choose a name for this thread.</Dialog.Description>
      <form onsubmit={(event) => { event.preventDefault(); submit(); }}>
        <label class="mt-[14px] grid gap-[5px] text-[11px] text-muted">
          <span>Thread name</span>
          <input
            bind:this={inputElement}
            bind:value={input}
            class="min-w-0 rounded-[7px] border border-line bg-recessed px-[9px] py-2 text-text focus-visible:border-line-strong focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring"
          />
        </label>
        <footer class="mt-[18px] flex flex-wrap justify-end gap-[7px] [&_button]:rounded-[7px] [&_button]:border [&_button]:border-line [&_button]:bg-transparent [&_button]:px-3 [&_button]:py-[7px] [&_button]:text-text-soft hover:[&_button]:border-line-strong hover:[&_button]:bg-panel-hover hover:[&_button]:text-text [&_button[type=submit]]:border-transparent [&_button[type=submit]]:bg-accent [&_button[type=submit]]:font-semibold [&_button[type=submit]]:text-accent-contrast hover:[&_button[type=submit]]:bg-accent-hover [&_button[type=submit]:disabled]:opacity-50">
          <button type="button" onclick={cancel}>Cancel</button>
          <button type="submit" disabled={!input.trim()}>Save</button>
        </footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
