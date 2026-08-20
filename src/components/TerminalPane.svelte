<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { IconRefresh, IconTerminal2, IconX } from '@tabler/icons-svelte';
  import { FitAddon } from '@xterm/addon-fit';
  import { Terminal } from '@xterm/xterm';

  import {
    resizeTerminal,
    startTerminal,
    stopTerminal,
    writeTerminal,
    type TerminalEvent,
  } from '../services/native';
  import type { ThreadState } from '../types';
  import { folderName } from '../utils/threads';

  interface Props {
    thread: ThreadState;
    active: boolean;
    reducedMotion: boolean;
    close: () => void;
    exited: () => void;
  }

  type TerminalStatus =
    | { kind: 'starting' }
    | { kind: 'running' }
    | { kind: 'exited'; code: number }
    | { kind: 'error'; message: string };

  const { thread, active, reducedMotion, close, exited }: Props = $props();
  let host = $state<HTMLDivElement>();
  let terminal: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  let terminalId: string | undefined;
  let status = $state<TerminalStatus>({ kind: 'starting' });
  let disposed = false;
  let launchGeneration = 0;
  let writeQueue = Promise.resolve();
  let resizeQueue = Promise.resolve();

  onMount(() => {
    if (!host) return;
    terminal = new Terminal({
      allowTransparency: false,
      cursorBlink: !reducedMotion,
      fontFamily: '"Cascadia Mono", "Cascadia Code", Consolas, "Liberation Mono", monospace',
      fontSize: 12,
      lineHeight: 1.25,
      minimumContrastRatio: 4.5,
      screenReaderMode: true,
      scrollback: 5000,
      theme: terminalTheme(),
    });
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(host);

    const inputSubscription = terminal.onData((data) => {
      const id = terminalId;
      if (!id) return;
      writeQueue = writeQueue
        .then(() => writeTerminal(id, data))
        .catch((error) => {
          if (!disposed && id === terminalId) {
            status = { kind: 'error', message: errorMessage(error) };
          }
        });
    });
    const resizeSubscription = terminal.onResize(({ cols, rows }) => {
      const id = terminalId;
      if (!id) return;
      resizeQueue = resizeQueue
        .then(() => resizeTerminal(id, cols, rows))
        .catch(() => {});
    });
    const resizeObserver = new ResizeObserver(() => {
      if (active) fitTerminal(false);
    });
    resizeObserver.observe(host);
    fitTerminal(true);
    void launch();

    return () => {
      disposed = true;
      launchGeneration += 1;
      resizeObserver.disconnect();
      inputSubscription.dispose();
      resizeSubscription.dispose();
      terminal?.dispose();
      terminal = undefined;
      const runningId = terminalId;
      terminalId = undefined;
      if (runningId) void stopTerminal(runningId).catch(() => {});
    };
  });

  $effect(() => {
    if (!active) return;
    void tick().then(() => fitTerminal(true));
  });

  async function launch() {
    const generation = ++launchGeneration;
    status = { kind: 'starting' };
    fitTerminal(false);
    try {
      const id = await startTerminal({
        threadId: thread.id,
        cwd: thread.cwd,
        cols: terminal?.cols ?? 80,
        rows: terminal?.rows ?? 24,
      }, (event) => handleEvent(generation, event));
      if (disposed || generation !== launchGeneration) {
        await stopTerminal(id);
        return;
      }
      terminalId = id;
      status = { kind: 'running' };
      fitTerminal(true);
    } catch (error) {
      if (!disposed && generation === launchGeneration) {
        status = { kind: 'error', message: errorMessage(error) };
      }
    }
  }

  async function restart() {
    const generation = ++launchGeneration;
    const runningId = terminalId;
    terminalId = undefined;
    if (runningId) await stopTerminal(runningId).catch(() => {});
    if (disposed || generation !== launchGeneration) return;
    terminal?.reset();
    await launch();
  }

  function handleEvent(generation: number, event: TerminalEvent) {
    if (disposed || generation !== launchGeneration) return;
    switch (event.event) {
      case 'output':
        terminal?.write(new Uint8Array(event.data.bytes));
        break;
      case 'exited':
        terminalId = undefined;
        status = { kind: 'exited', code: event.data.code };
        exited();
        break;
      case 'error':
        status = { kind: 'error', message: event.data.message };
        break;
    }
  }

  function fitTerminal(focus: boolean) {
    if (!active || !terminal || !fitAddon || !host?.offsetParent) return;
    fitAddon.fit();
    if (focus) terminal.focus();
  }

  function terminalTheme() {
    const styles = getComputedStyle(document.documentElement);
    const color = (name: string) => styles.getPropertyValue(name).trim();
    return {
      background: color('--shell-solid'),
      foreground: color('--text-soft'),
      cursor: color('--text'),
      cursorAccent: color('--shell-solid'),
      selectionBackground: 'rgba(210, 213, 220, 0.24)',
      black: color('--shell-solid'),
      red: color('--danger'),
      green: color('--success'),
      yellow: color('--warning'),
      blue: color('--muted'),
      magenta: color('--text-soft'),
      cyan: color('--muted'),
      white: color('--text'),
      brightBlack: color('--faint'),
      brightRed: color('--danger'),
      brightGreen: color('--success'),
      brightYellow: color('--warning'),
      brightBlue: color('--text-soft'),
      brightMagenta: color('--text'),
      brightCyan: color('--text-soft'),
      brightWhite: color('--text'),
    };
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<section class:active class="terminal-pane" aria-label={`Terminal for ${thread.title}`}>
  <header class="terminal-header">
    <IconTerminal2 size={14} stroke={1.55} />
    <strong>Terminal</strong>
    <small title={thread.cwd}>{folderName(thread.cwd)}</small>
    <span
      class:error={status.kind === 'error'}
      class="terminal-status"
      aria-live="polite"
      title={status.kind === 'error' ? status.message : undefined}
    >
      {#if status.kind === 'starting'}Starting…{:else if status.kind === 'exited'}Exited {status.code}{:else if status.kind === 'error'}{status.message}{/if}
    </span>
    {#if status.kind === 'exited' || status.kind === 'error'}
      <button class="chrome-button" aria-label="Restart terminal" title="Restart terminal" onclick={() => { void restart(); }}>
        <IconRefresh size={14} stroke={1.55} />
      </button>
    {/if}
    <button class="chrome-button" aria-label="Close terminal drawer" title="Close terminal drawer" onclick={close}>
      <IconX size={15} stroke={1.55} />
    </button>
  </header>
  <div bind:this={host} class="terminal-host" aria-label="Terminal input and output"></div>
</section>
