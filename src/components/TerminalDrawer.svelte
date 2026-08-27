<script lang="ts">
  import { untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';

  import TerminalPane from './TerminalPane.svelte';
  import type { ThreadState } from '../types';
  import { shellLayoutDuration } from '../utils/shell-layout-motion';

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

  let layoutSettled = $state(false);
  let mounted = $state(untrack(() => open));

  const drawerHeight = new Tween(0, { duration: 0, easing: cubicOut });
  const drawerOpacity = new Tween(0, { duration: 0, easing: cubicOut });

  const expandedHeight = $derived(
    `clamp(140px, ${height}px, calc(100dvh - var(--titlebar-height) - 120px))`,
  );

  $effect(() => {
    const duration = shellLayoutDuration(reducedMotion);
    if (open) {
      mounted = true;
      void drawerOpacity.set(1, { duration });
      void drawerHeight.set(height, { duration }).then(() => {
        if (open) layoutSettled = true;
      });
      return;
    }
    layoutSettled = reducedMotion;
    void drawerOpacity.set(0, { duration });
    void drawerHeight.set(0, { duration }).then(() => {
      if (!open) mounted = false;
    });
  });

  $effect(() => {
    if (open && drawerHeight.current > 0) {
      void drawerHeight.set(height, { duration: 0 });
    }
  });
</script>

<section
  class="relative min-h-0 min-w-0 overflow-hidden border-t bg-transparent {open ? 'pointer-events-auto visible border-line' : 'pointer-events-none invisible border-transparent'}"
  style:height="{drawerHeight.current}px"
  style:opacity={drawerOpacity.current}
  style:max-height={expandedHeight}
  aria-label="Terminal drawer"
  aria-hidden={!open}
>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="absolute top-[-4px] right-0 left-0 z-[4] h-[9px] cursor-ns-resize touch-none after:absolute after:top-1 after:right-0 after:left-0 after:h-px after:bg-transparent after:content-[''] hover:after:bg-line-strong focus-visible:after:bg-line-strong [body.resizing-terminal_&]:after:bg-line-strong"
    role="separator"
    aria-label="Resize terminal drawer"
    aria-orientation="horizontal"
    aria-valuemin="140"
    aria-valuemax="520"
    aria-valuenow={height}
    tabindex={open ? 0 : -1}
    onpointerdown={startResize}
    onkeydown={(event) => {
      const delta = event.key === 'ArrowUp' ? 10 : event.key === 'ArrowDown' ? -10 : 0;
      if (!delta) return;
      event.preventDefault();
      resizeBy(delta);
    }}
  ></div>
  {#if mounted}
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
  {/if}
</section>
