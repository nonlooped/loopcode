<script lang="ts">
  import { AlertDialog } from 'bits-ui';
  import { z } from 'zod';

  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, answer, decline }: Props = $props();
  let dialogElement = $state<HTMLElement>();
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

  function focusPreferredOption(event: Event) {
    const option = dialogElement?.querySelector<HTMLButtonElement>('.permission-actions button.primary');
    if (!option) return;
    event.preventDefault();
    option.focus();
  }

  $effect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => previous?.focus();
  });
</script>

<AlertDialog.Root open onOpenChange={(open) => { if (!open) decline(); }}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="modal-overlay" />
    <AlertDialog.Content
      bind:ref={dialogElement}
      class="permission-modal"
      aria-describedby="permission-description"
      onOpenAutoFocus={focusPreferredOption}
    >
      <AlertDialog.Title>{title}</AlertDialog.Title>
      {#if detail.command}
        <p>A coding agent is asking to run the command below.</p>
      {/if}

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
            onclick={() => answer(option.optionId)}
          >
            <strong>{option.name}</strong>
            {#if option.description ?? optionDescription(option.kind)}
              <span>{option.description ?? optionDescription(option.kind)}</span>
            {/if}
          </button>
        {/each}
        {#if request.options.length === 0}
          <button class="primary" onclick={decline}><strong>Dismiss</strong></button>
        {/if}
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
