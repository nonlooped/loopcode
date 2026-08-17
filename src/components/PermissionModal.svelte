<script lang="ts">
  import { onMount } from 'svelte';

  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, answer, decline }: Props = $props();
  let dialog = $state<HTMLDivElement>();

  const primaryOptionId = $derived(
    request.options.find((option) => option.kind === 'allow_once')?.optionId ??
      request.options.find((option) => option.kind?.startsWith('allow'))?.optionId,
  );
  const detail = $derived(parsePermissionDetail(request.detail));
  const title = $derived(
    detail.command && request.title === 'Allow the harness to continue?'
      ? 'Run this command?'
      : request.title,
  );

  function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
  }

  function parsePermissionDetail(value: string) {
    try {
      const parsed: unknown = JSON.parse(value);
      if (!isRecord(parsed)) throw new Error('Permission detail is not an object');

      const command = typeof parsed.command === 'string' ? parsed.command : undefined;
      const cwd = typeof parsed.cwd === 'string' ? parsed.cwd : undefined;
      const remaining = Object.fromEntries(
        Object.entries(parsed).filter(([key]) => key !== 'command' && key !== 'cwd'),
      );

      return {
        command,
        cwd,
        extra: Object.keys(remaining).length ? JSON.stringify(remaining, null, 2) : undefined,
        raw: command || cwd ? undefined : value,
      };
    } catch {
      return { command: undefined, cwd: undefined, extra: undefined, raw: value };
    }
  }

  function optionDescription(kind?: string) {
    if (kind === 'allow_once') return 'Run it this time only.';
    if (kind === 'allow_always') return 'Remember this permission for future matching requests.';
    if (kind === 'reject_once') return 'Do not run it this time.';
    if (kind === 'reject_always') return 'Block future matching requests too.';
    return undefined;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      decline();
      return;
    }

    if (event.key !== 'Tab' || !dialog) return;
    const controls = [...dialog.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
    if (controls.length === 0) return;

    const first = controls[0];
    const last = controls.at(-1)!;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  onMount(() => {
    if (!dialog) return;
    const preferred = dialog.querySelector<HTMLButtonElement>('[data-primary="true"]');
    (preferred ?? dialog).focus();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-backdrop" role="presentation">
  <div
    bind:this={dialog}
    class="permission-modal"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="permission-title"
    aria-describedby="permission-description"
    tabindex="-1"
  >
    <header class="permission-header">
      <h2 id="permission-title">{title}</h2>
      {#if detail.command}
        <p>A coding agent is asking to run the command below.</p>
      {/if}
    </header>

    <div id="permission-description" class="permission-detail">
      {#if detail.command}
        <section class="permission-field">
          <span>Command</span>
          <pre>{detail.command}</pre>
        </section>
        {#if detail.cwd}
          <section class="permission-field">
            <span>Working folder</span>
            <pre>{detail.cwd}</pre>
          </section>
        {/if}
        {#if detail.extra}
          <details>
            <summary>Additional details</summary>
            <pre>{detail.extra}</pre>
          </details>
        {/if}
      {:else}
        <pre class="permission-raw">{detail.raw}</pre>
      {/if}
    </div>

    {#if request.options.length > 0}
      <p class="permission-prompt">Choose how to proceed</p>
    {/if}
    <footer class="permission-actions">
      {#each request.options as option}
        <button
          class:primary={option.optionId === primaryOptionId}
          class:reject={option.kind?.startsWith('reject')}
          data-primary={option.optionId === primaryOptionId}
          onclick={() => answer(option.optionId)}
        >
          <strong>{option.name}</strong>
          {#if option.description ?? optionDescription(option.kind)}
            <span>{option.description ?? optionDescription(option.kind)}</span>
          {/if}
        </button>
      {/each}
      {#if request.options.length === 0}
        <button class="primary" data-primary="true" onclick={decline}><strong>Dismiss</strong></button>
      {/if}
    </footer>
  </div>
</div>
