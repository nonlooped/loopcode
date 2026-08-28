<script lang="ts">
  import {
    IconCheck,
    IconCircle,
    IconCircleDot,
    IconFileDiff,
    IconFileMinus,
    IconFilePlus,
  } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import type { PlanEntry, ToolActivity, ToolDiff } from '../types';
  import { copyText } from '../utils/clipboard';
  import { materialFileIcon } from '../utils/material-file-icons';
  import { diffLineStats, unifiedDiffLines } from '../utils/text-diff';

  interface Props {
    tool: ToolActivity;
    openFile: (path: string) => void;
  }

  const { tool, openFile }: Props = $props();

  // eslint-disable-next-line no-control-regex
  const ansiEscape = /\u001b\[[0-9;?]*[A-Za-z]/g;

  const renderedDiffs = $derived(
    (tool.diffs ?? []).map((diff) => {
      const lines = unifiedDiffLines(diff.oldText, diff.newText);
      return { diff, lines, stats: diffLineStats(lines) };
    }),
  );
  // Command output is captured, not attached to a live TTY, so colour codes would render raw.
  const terminalOutput = $derived(tool.terminal?.output.replace(ansiEscape, '') ?? '');
  const exitCode = $derived(tool.terminal?.exitCode);

  const diffKindIcon = {
    add: IconFilePlus,
    delete: IconFileMinus,
    update: IconFileDiff,
  };

  const indent = 'mx-[5px] mb-[5px] ml-[27px]';
  const panel = 'min-w-0 overflow-hidden rounded-[5px] border border-line bg-recessed';
  const monospace = 'font-mono text-[11px] leading-snug';

  function fileName(path: string) {
    return path.split(/[/\\]/).pop() || path;
  }

  function diffMenuItems(entry: { diff: ToolDiff; lines: { text: string }[] }) {
    return [
      { label: 'Open file', action: () => openFile(entry.diff.path) },
      { label: 'Copy path', action: () => copyText(entry.diff.path) },
      {
        label: 'Copy diff',
        action: () => copyText(entry.lines.map((line) => line.text).join('\n')),
      },
    ];
  }

  function planIcon(status: PlanEntry['status']) {
    if (status === 'completed') return IconCheck;
    return status === 'in_progress' ? IconCircleDot : IconCircle;
  }

  const diffLineClass = (kind: string) =>
    `block px-2 break-anywhere whitespace-pre-wrap ${
      kind === 'add' ? 'bg-[color-mix(in_srgb,var(--success)_13%,transparent)]' : ''
    } ${kind === 'del' ? 'bg-[color-mix(in_srgb,var(--danger)_13%,transparent)]' : ''} ${
      kind === 'hunk' ? 'bg-[color-mix(in_srgb,var(--muted)_8%,transparent)] text-faint' : ''
    }`;
</script>

{#if tool.plan}
  <div class="{indent} grid gap-0.5">
    {#each tool.plan as entry, index (`${index}:${entry.content}`)}
      {@const Icon = planIcon(entry.status)}
      <div
        class="flex min-w-0 items-baseline gap-2 text-[13px] leading-[1.55]"
        class:text-muted={entry.status === 'pending'}
        class:text-text-soft={entry.status === 'in_progress'}
        class:text-faint={entry.status === 'completed'}
      >
        <span class="grid size-3.5 shrink-0 translate-y-[3px] place-items-center">
          <Icon size={13} stroke={entry.status === 'pending' ? 1.4 : 2} />
        </span>
        <span class="min-w-0 break-anywhere" class:line-through={entry.status === 'completed'}
          >{entry.content}</span
        >
      </div>
    {/each}
  </div>
{/if}

{#each renderedDiffs as entry (entry.diff.path)}
  {@const KindIcon = diffKindIcon[entry.diff.kind ?? 'update']}
  <ContextMenu items={diffMenuItems(entry)}>
    {#snippet children({ props })}
      <div {...props} role="presentation" class="{indent} {panel}">
        <button
          type="button"
          class="flex w-full min-w-0 cursor-pointer items-center gap-1.5 border-0 border-b border-line bg-transparent px-2 py-1.5 text-left hover:bg-panel-hover"
          title={entry.diff.path}
          onclick={() => openFile(entry.diff.path)}
        >
          <img
            class="size-3.5 shrink-0 opacity-[0.64] [filter:var(--provider-filter)]"
            src={materialFileIcon(entry.diff.path)}
            alt=""
          />
          <span
            class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11.5px] text-text-soft"
            >{fileName(entry.diff.path)}</span
          >
          {#if entry.stats.additions > 0}
            <span class="shrink-0 {monospace} text-success">+{entry.stats.additions}</span>
          {/if}
          {#if entry.stats.deletions > 0}
            <span class="shrink-0 {monospace} text-danger">−{entry.stats.deletions}</span>
          {/if}
          <KindIcon size={13} stroke={1.55} class="shrink-0 text-faint" />
        </button>
        {#if entry.lines.length === 0}
          <p class="m-0 p-2 text-[11px] leading-snug text-faint">No text changes to display.</p>
        {:else}
          <div class="max-h-[280px] overflow-auto py-1 {monospace} tabular-nums">
            {#each entry.lines as line, lineIndex (`${lineIndex}:${line.kind}`)}
              <span class={diffLineClass(line.kind)}>{line.text || ' '}</span>
            {/each}
          </div>
        {/if}
      </div>
    {/snippet}
  </ContextMenu>
{/each}

{#if tool.terminal}
  <ContextMenu items={[{ label: 'Copy output', action: () => copyText(terminalOutput) }]}>
    {#snippet children({ props })}
      <div {...props} role="presentation" class="{indent} {panel}">
        {#if terminalOutput}
          <pre
            class="m-0 max-h-[280px] overflow-auto p-[7px_8px] {monospace} whitespace-pre-wrap break-anywhere text-muted">{terminalOutput}</pre>
        {:else}
          <p class="m-0 p-2 text-[11px] leading-snug text-faint">
            {tool.status === 'in_progress' ? 'Waiting for output…' : 'No output.'}
          </p>
        {/if}
        {#if exitCode !== undefined && exitCode !== null && exitCode !== 0}
          <p
            class="m-0 border-t border-line px-2 py-1 {monospace} text-danger"
            aria-label={`Exit code ${exitCode}`}
          >
            Exited with code {exitCode}
          </p>
        {/if}
      </div>
    {/snippet}
  </ContextMenu>
{/if}

{#if tool.detail}
  <pre
    class="{indent} max-h-[260px] overflow-auto rounded-[5px] border border-line bg-recessed p-[7px_8px] {monospace} whitespace-pre-wrap break-anywhere text-muted">{tool.detail}</pre>
{/if}
