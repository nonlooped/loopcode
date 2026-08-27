<script lang="ts">
  import { prefersReducedMotion, Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import TerminalPane from './TerminalPane.svelte';
  import type { ThreadState } from '../types';

  interface Props {
    threads: ThreadState[];
    selectedThreadId: string;
    open: boolean;
    height: number;
    fontSize: number;
    scrollback: number;
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
    fontSize,
    scrollback,
    reducedMotion,
    close,
    terminalExited,
    startResize,
    resizeBy,
  }: Props = $props();
  const drawerHeight = new Tween(0);
  let layoutSettled = $state(false);

  $effect(() => {
    layoutSettled = false;
    void drawerHeight.set(open ? height : 0, {
      duration: reducedMotion || prefersReducedMotion.current ? 0 : 220,
      easing: cubicOut,
    }).then(() => { layoutSettled = open; });
  });

  function handleResizeKeydown(event: KeyboardEvent) {
    const delta = event.key === 'ArrowUp' ? 10 : event.key === 'ArrowDown' ? -10 : 0;
    if (!delta) return;
    event.preventDefault();
    resizeBy(delta);
  }
</script>

<section
  class="relative min-w-0 overflow-hidden border-t border-line bg-transparent"
  style:height={`${drawerHeight.current}px`}
  class:pointer-events-none={!open}
  class:opacity-0={!open}
  aria-label="Terminal drawer"
  aria-hidden={!open}
>
  <!-- The ARIA separator is keyboard-resizable, though Svelte treats it as non-interactive. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="absolute -top-1 right-0 left-0 z-4 h-2 touch-none cursor-ns-resize before:absolute before:top-1 before:right-0 before:left-0 before:h-px before:bg-transparent hover:before:bg-line-strong focus-visible:before:bg-line-strong"
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
      ready={layoutSettled && open && thread.id === selectedThreadId}
      {fontSize}
      {scrollback}
      {reducedMotion}
      {close}
      exited={() => terminalExited(thread.id)}
    />
  {/each}
</section>
