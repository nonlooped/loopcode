<script lang="ts">
  import { AlertDialog } from 'bits-ui';
  import { z } from 'zod';

  import MarkdownMessage from './markdown/MarkdownMessage.svelte';
  import type { PermissionOption, PermissionRequest } from '../types';
  import { fileChangeDiffLines } from '../utils/text-diff';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, answer, decline }: Props = $props();
  let dialogElement = $state<HTMLElement | null>(null);
  const primaryOptionId = $derived(
    request.options.find((option) => option.kind === 'allow_once')?.optionId ??
      request.options.find((option) => option.kind?.startsWith('allow'))?.optionId,
  );
  const detail = $derived(parsePermissionDetail(request.detail));
  const fileChanges = $derived(
    request.type === 'permission' ? (request.fileChanges ?? []) : [],
  );
  const planMarkdown = $derived(request.type === 'permission' ? request.planMarkdown : undefined);
  const changeDiffs = $derived(
    fileChanges.map((change) => ({
      change,
      lines: fileChangeDiffLines(change.diff, change.kind),
    })),
  );
  const title = $derived(detail.command ? 'Run this command?' : request.title);
  const description = $derived(describe(detail.command, fileChanges.length, planMarkdown));

  function describe(command: string | undefined, changeCount: number, plan: string | undefined) {
    if (command) return 'A coding agent is asking to run the command below.';
    if (plan) return 'Review the proposed plan before continuing.';
    if (changeCount > 0) return 'A coding agent is asking to write the changes below.';
    return 'Review this request before continuing.';
  }

  const changeVerb = { add: 'Create', update: 'Edit', delete: 'Delete' };

  const diffLineClass = (kind: string) =>
    `block px-2.5 break-anywhere whitespace-pre-wrap ${
      kind === 'add' ? 'bg-[color-mix(in_srgb,var(--success)_13%,transparent)]' : ''
    } ${kind === 'del' ? 'bg-[color-mix(in_srgb,var(--danger)_13%,transparent)]' : ''} ${
      kind === 'hunk' ? 'bg-[color-mix(in_srgb,var(--muted)_8%,transparent)] text-faint' : ''
    }`;
  const options = $derived([...request.options].sort(byPromptOrder));

  const permissionDetailSchema = z.object({
    command: z.string().optional(),
    cwd: z.string().optional(),
  }).loose();

  const modalShell =
    'fixed top-1/2 left-1/2 z-[91] w-[min(520px,calc(100vw-40px))] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-overlay border border-line-strong bg-decision p-[18px] shadow-overlay outline-0 backdrop-blur-overlay';
  const modalHeading =
    '[&_[role=heading]]:text-[15px] [&_[role=heading]]:leading-snug [&_[role=heading]]:font-semibold [&_[role=heading]]:tracking-tight [&_[role=heading]]:text-text';
  const modalDescription =
    '[&_[data-alert-dialog-description]]:mt-[5px] [&_[data-alert-dialog-description]]:text-xs [&_[data-alert-dialog-description]]:leading-snug [&_[data-alert-dialog-description]]:text-muted';
  const actionFooter =
    'mt-[18px] flex flex-wrap justify-end gap-[7px] [&_button]:rounded-[7px] [&_button]:border [&_button]:border-line [&_button]:bg-transparent [&_button]:px-3 [&_button]:py-[7px] [&_button]:text-text-soft hover:[&_button]:border-line-strong hover:[&_button]:bg-panel-hover hover:[&_button]:text-text [&_button.primary]:border-transparent [&_button.primary]:bg-accent [&_button.primary]:font-semibold [&_button.primary]:text-accent-contrast hover:[&_button.primary]:border-transparent hover:[&_button.primary]:bg-accent-hover hover:[&_button.primary]:text-accent-contrast';

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

  function optionDescription(option: PermissionOption) {
    if (option.description) return option.description;
    if (option.kind === 'allow_once') return 'Run it this time only.';
    if (option.kind === 'allow_always') return 'Remember this permission for future matching requests.';
    if (option.kind === 'reject_once') return 'Do not run it this time.';
    if (option.kind === 'reject_always') return 'Block future matching requests too.';
    return undefined;
  }

  function byPromptOrder(left: PermissionOption, right: PermissionOption) {
    return promptRank(left) - promptRank(right);
  }

  function promptRank(option: PermissionOption) {
    if (option.optionId === primaryOptionId) return 3;
    if (option.kind?.startsWith('allow')) return 2;
    return 1;
  }

  function focusPreferredOption(event: Event) {
    const option = dialogElement?.querySelector<HTMLButtonElement>('footer button.primary');
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
    <AlertDialog.Overlay class="fixed inset-0 z-[90] bg-overlay backdrop-blur-scrim" />
    <AlertDialog.Content
      bind:ref={dialogElement}
      class="{modalShell} {modalHeading} {modalDescription}"
      onOpenAutoFocus={focusPreferredOption}
    >
      <AlertDialog.Title>{title}</AlertDialog.Title>
      <AlertDialog.Description>{description}</AlertDialog.Description>

      <div class="mt-[14px] max-h-[min(300px,42vh)] overflow-auto">
        {#if detail.command}
          <section class="overflow-hidden rounded-[7px] border border-line bg-recessed">
            {#if detail.cwd}
              <header
                class="overflow-hidden text-ellipsis whitespace-nowrap border-b border-line px-2.5 py-1.5 font-mono text-[10px] leading-snug text-faint"
                title={detail.cwd}
              >{detail.cwd}</header>
            {/if}
            <pre class="m-0 overflow-auto whitespace-pre-wrap break-anywhere p-2.5 font-mono text-[11.5px] leading-relaxed text-text-soft">{detail.command}</pre>
          </section>
          {#if detail.extra}
            <details class="mt-2 [&_pre]:mt-1.5 [&_pre]:overflow-auto [&_pre]:whitespace-pre-wrap [&_pre]:break-anywhere [&_pre]:rounded-[7px] [&_pre]:border [&_pre]:border-line [&_pre]:bg-recessed [&_pre]:p-2.5 [&_pre]:font-mono [&_pre]:text-[11.5px] [&_pre]:leading-relaxed [&_pre]:text-text-soft">
              <summary class="cursor-pointer py-1 text-[11px] text-faint hover:text-muted">Additional details</summary>
              <pre>{detail.extra}</pre>
            </details>
          {/if}
        {:else if planMarkdown}
          <div class="rounded-[7px] border border-line bg-recessed p-2.5 text-[13px] leading-[1.55] text-text-soft [&>:last-child]:mb-0">
            <MarkdownMessage id={`plan-${String(request.requestId)}`} source={planMarkdown} />
          </div>
        {:else if changeDiffs.length > 0}
          <div class="grid gap-2">
            {#each changeDiffs as entry (entry.change.path)}
              <section class="overflow-hidden rounded-[7px] border border-line bg-recessed">
                <header
                  class="overflow-hidden text-ellipsis whitespace-nowrap border-b border-line px-2.5 py-1.5 font-mono text-[10px] leading-snug text-faint"
                  title={entry.change.path}
                >{changeVerb[entry.change.kind]} {entry.change.path}</header>
                <div class="max-h-[220px] overflow-auto py-1 font-mono text-[11.5px] leading-relaxed tabular-nums">
                  {#each entry.lines as line, lineIndex (`${lineIndex}:${line.kind}`)}
                    <span class={diffLineClass(line.kind)}>{line.text || ' '}</span>
                  {/each}
                </div>
              </section>
            {/each}
          </div>
        {:else}
          <pre class="m-0 overflow-auto whitespace-pre-wrap break-anywhere rounded-[7px] border border-line bg-recessed p-2.5 font-mono text-[11.5px] leading-relaxed text-muted">{detail.raw}</pre>
        {/if}
      </div>

      <footer class={actionFooter}>
        {#each options as option}
          <button
            class:primary={option.optionId === primaryOptionId}
            title={optionDescription(option)}
            onclick={() => answer(option.optionId)}
          >
            {option.name}
          </button>
        {/each}
        {#if options.length === 0}
          <button class="primary" onclick={decline}>Dismiss</button>
        {/if}
      </footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
