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
    <AlertDialog.Overlay class="fixed inset-0 z-[90] bg-overlay backdrop-blur-[3px]" />
    <AlertDialog.Content
      bind:ref={dialogElement}
      class="fixed top-1/2 left-1/2 z-[91] w-[min(540px,calc(100vw_-_40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-xl border border-line-strong bg-[var(--decision-surface)] p-[18px] outline-none shadow-overlay backdrop-blur-xl backdrop-saturate-[115%] [&>h2]:text-base [&>h2]:leading-[1.4] [&>h2]:font-semibold [&>h2]:tracking-[-.01em] [&>p]:mt-1 [&>p]:text-xs [&>p]:leading-[1.45] [&>p]:text-muted"
      aria-describedby="permission-description"
      onOpenAutoFocus={focusPreferredOption}
    >
      <AlertDialog.Title>{title}</AlertDialog.Title>
      {#if detail.command}
        <p>A coding agent is asking to run the command below.</p>
      {/if}

      <div id="permission-description" class="my-4 max-h-[min(300px,42vh)] overflow-auto [&_details]:mt-3 [&_pre]:mt-1 [&_pre]:overflow-auto [&_pre]:rounded-md [&_pre]:border [&_pre]:border-line [&_pre]:bg-recessed [&_pre]:px-2.5 [&_pre]:py-2 [&_pre]:font-mono [&_pre]:text-[11px] [&_pre]:leading-[1.55] [&_pre]:text-ink-soft [&_pre]:whitespace-pre-wrap [&_pre]:wrap-anywhere [&_section+section]:mt-3 [&_section>span]:text-[11px] [&_section>span]:font-semibold [&_section>span]:tracking-[.04em] [&_section>span]:text-muted [&_section>span]:uppercase [&_summary]:cursor-pointer [&_summary]:text-[11px] [&_summary]:font-semibold [&_summary]:tracking-[.04em] [&_summary]:text-muted [&_summary]:uppercase">
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
          <pre class="text-muted">{detail.raw}</pre>
        {/if}
      </div>

      {#if request.options.length > 0}
        <p class="mb-2 text-[11px] font-semibold tracking-[.04em] text-muted uppercase">Choose how to proceed</p>
      {/if}
      <footer class="grid grid-cols-2 gap-2">
        {#each request.options as option}
          <button
            class={`flex min-h-12 min-w-0 flex-col items-start justify-center rounded-md border border-line px-2.5 py-2 text-left text-ink-soft hover:border-line-strong hover:bg-panel-hover hover:text-ink disabled:cursor-default disabled:opacity-45 [&_strong]:text-[11px] [&_strong]:leading-[1.35] [&_strong]:font-semibold [&_span]:mt-0.5 [&_span]:text-[11px] [&_span]:leading-[1.35] [&_span]:font-normal [&_span]:text-muted ${option.optionId === primaryOptionId ? 'border-line-strong bg-accent text-accent-contrast hover:bg-accent-hover hover:text-accent-contrast [&_span]:text-[var(--accent-contrast-soft)]' : ''} ${option.kind?.startsWith('reject') ? 'text-muted' : ''}`}
            onclick={() => answer(option.optionId)}
          >
            <strong>{option.name}</strong>
            {#if option.description ?? optionDescription(option.kind)}
              <span>{option.description ?? optionDescription(option.kind)}</span>
            {/if}
          </button>
        {/each}
        {#if request.options.length === 0}
          <button class="flex min-h-12 min-w-0 flex-col items-start justify-center rounded-md border border-line-strong bg-accent px-2.5 py-2 text-left text-accent-contrast hover:bg-accent-hover" onclick={decline}><strong class="text-[11px] font-semibold">Dismiss</strong></button>
        {/if}
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
