<script lang="ts">
  import { IconFlag, IconPlayerPause, IconPlayerPlay, IconTrash } from '@tabler/icons-svelte';
  import type { GoalAction, ProviderSessionState } from '../types';

  interface Props {
    provider: ProviderSessionState;
    control: (action: GoalAction, objective?: string) => void;
  }

  const { provider, control }: Props = $props();
  let editing = $state(false);
  let objective = $state('');
  const goal = $derived(provider.goal);
  const actions = $derived(provider.goalActions ?? []);

  function submit() {
    const value = objective.trim();
    if (!value) return;
    control('set', value);
    objective = '';
    editing = false;
  }

  function number(value: number) {
    return new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 }).format(value);
  }
</script>

{#if actions.length > 0}
  <section class="mx-auto mb-2 flex w-[min(var(--content-width,720px),calc(100%-32px))] items-center gap-2 rounded-lg border border-line bg-panel px-2.5 py-2 text-[11px] text-muted" aria-label="Long-running goal">
    <IconFlag class="shrink-0" size={14} stroke={1.55} />
    {#if editing}
      <form class="flex min-w-0 flex-1 gap-1.5" onsubmit={(event) => { event.preventDefault(); submit(); }}>
        <input bind:value={objective} class="min-w-0 flex-1 rounded-md border border-line-strong bg-recessed px-2 py-1 text-text outline-none" placeholder="Goal objective" aria-label="Goal objective" />
        <button class="rounded-md border border-line-strong bg-accent px-2 text-accent-contrast" type="submit">Set</button>
        <button class="rounded-md border border-line bg-transparent px-2" type="button" onclick={() => { editing = false; }}>Cancel</button>
      </form>
    {:else if goal}
      <div class="min-w-0 flex-1">
        <strong class="block truncate font-medium text-text-soft" title={goal.objective}>{goal.objective}</strong>
        <span class="flex flex-wrap gap-x-2 text-faint">
          <span class="capitalize">{goal.status}</span>
          {#if goal.iterations !== undefined}<span>{goal.iterations} iterations</span>{/if}
          {#if goal.tokenBudget != null}<span>{number(goal.tokensUsed ?? 0)} / {number(goal.tokenBudget)} tokens</span>{:else if goal.tokensUsed !== undefined}<span>{number(goal.tokensUsed)} tokens</span>{/if}
          {#if goal.timeBudgetSeconds != null}<span>{goal.timeUsedSeconds ?? 0}s / {goal.timeBudgetSeconds}s</span>{:else if goal.timeUsedSeconds !== undefined}<span>{goal.timeUsedSeconds}s</span>{/if}
          {#if goal.createdAt}<span>created {new Date(goal.createdAt).toLocaleString()}</span>{/if}
          {#if goal.lastReason}<span title={goal.lastReason}>{goal.lastReason}</span>{/if}
        </span>
      </div>
      {#if goal.status === 'active' && actions.includes('pause')}
        <button class="grid size-7 place-items-center rounded-md border border-line bg-transparent hover:bg-panel-hover" aria-label="Pause goal" title="Pause goal" onclick={() => control('pause')}><IconPlayerPause size={13} /></button>
      {:else if goal.status === 'paused' && actions.includes('resume')}
        <button class="grid size-7 place-items-center rounded-md border border-line bg-transparent hover:bg-panel-hover" aria-label="Resume goal" title="Resume goal" onclick={() => control('resume')}><IconPlayerPlay size={13} /></button>
      {/if}
      {#if actions.includes('clear')}
        <button class="grid size-7 place-items-center rounded-md border border-line bg-transparent hover:bg-panel-hover" aria-label="Clear goal" title="Clear goal" onclick={() => control('clear')}><IconTrash size={13} /></button>
      {/if}
    {:else if actions.includes('set')}
      <button class="rounded-md border border-line bg-transparent px-2 py-1 text-text-soft hover:bg-panel-hover" type="button" onclick={() => { editing = true; }}>Set goal</button>
    {/if}
  </section>
{/if}
