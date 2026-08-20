<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { z } from 'zod';

  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    reducedMotion: boolean;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, reducedMotion, answer, decline }: Props = $props();
  let dialog = $state<HTMLDialogElement>();
  let returnFocus: HTMLElement | null = null;

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

  const permissionDetailSchema = z.object({
    command: z.string().optional(),
    cwd: z.string().optional(),
  }).loose();

  function parsePermissionDetail(value: string) {
    try {
      const { command, cwd, ...remaining } = permissionDetailSchema.parse(JSON.parse(value));

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

  $effect(() => {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog?.showModal();
    return () => returnFocus?.focus();
  });

  function handleCancel(event: Event) {
    event.preventDefault();
    decline();
  }
</script>

<dialog
  bind:this={dialog}
  class="permission-modal"
  role="alertdialog"
  aria-labelledby="permission-title"
  aria-describedby="permission-description"
  transition:scale|global={{ start: reducedMotion ? 1 : 0.985, duration: reducedMotion ? 0 : 170 }}
  oncancel={handleCancel}
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
    <!-- svelte-ignore a11y_autofocus --><!-- Modal autofocus is the spec-recommended way to move focus into a showModal() dialog. -->
    {#each request.options as option}
      <button
        class:primary={option.optionId === primaryOptionId}
        class:reject={option.kind?.startsWith('reject')}
        data-primary={option.optionId === primaryOptionId}
        autofocus={option.optionId === primaryOptionId || null}
        onclick={() => answer(option.optionId)}
      >
        <strong>{option.name}</strong>
        {#if option.description ?? optionDescription(option.kind)}
          <span>{option.description ?? optionDescription(option.kind)}</span>
        {/if}
      </button>
    {/each}
    {#if request.options.length === 0}
      <!-- svelte-ignore a11y_autofocus --><!-- Modal autofocus is the spec-recommended way to move focus into a showModal() dialog. -->
      <button class="primary" data-primary="true" autofocus onclick={decline}><strong>Dismiss</strong></button>
    {/if}
  </footer>
</dialog>
