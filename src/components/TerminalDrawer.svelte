<script lang="ts">
  import TerminalPane from './TerminalPane.svelte';
  import type { ThreadState } from '../types';

  interface Props {
    threads: ThreadState[];
    selectedThreadId: string;
    open: boolean;
    height: number;
    reducedMotion: boolean;
    close: () => void;
    terminalExited: (threadId: string) => void;
    startResize: (event: PointerEvent) => void;
    resizeBy: (delta: number) => void;
  }

  const {
    threads,
    selectedThreadId,
    open,
    height,
    reducedMotion,
    close,
    terminalExited,
    startResize,
    resizeBy,
  }: Props = $props();

  function handleResizeKeydown(event: KeyboardEvent) {
    const delta = event.key === 'ArrowUp' ? 10 : event.key === 'ArrowDown' ? -10 : 0;
    if (!delta) return;
    event.preventDefault();
    resizeBy(delta);
  }
</script>

<section class:open class="terminal-drawer" aria-label="Terminal drawer" aria-hidden={!open}>
  <!-- The ARIA separator is keyboard-resizable, though Svelte treats it as non-interactive. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="terminal-resize-handle"
    role="separator"
    aria-label="Resize terminal drawer"
    aria-orientation="horizontal"
    aria-valuemin="140"
    aria-valuemax="520"
    aria-valuenow={height}
    tabindex={open ? 0 : -1}
    onpointerdown={startResize}
    onkeydown={handleResizeKeydown}
  ></div>
  {#each threads as thread (thread.id)}
    <TerminalPane
      {thread}
      active={open && thread.id === selectedThreadId}
      {reducedMotion}
      {close}
      exited={() => terminalExited(thread.id)}
    />
  {/each}
</section>
