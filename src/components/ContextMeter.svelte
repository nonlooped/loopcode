<script lang="ts">
  import type { ProviderSessionState } from '../types';

  interface Props {
    provider: ProviderSessionState;
    /** A thread with no history is at full context before Codex reports any usage. */
    fresh: boolean;
  }

  const { provider, fresh }: Props = $props();
  const used = $derived(provider.contextUsed ?? 0);
  const size = $derived(provider.contextSize ?? 0);
  const remaining = $derived(size > 0 ? Math.max(0, Math.round(((size - used) / size) * 100)) : 100);
  const known = $derived(size > 0);
  // Below a fifth remaining, Codex is close to compacting, which is worth flagging early.
  const low = $derived(remaining <= 20);
  const caution = $derived(!low && remaining <= 40);

  function compact(count: number) {
    if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
    if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
    return String(count);
  }
</script>

{#if known || fresh}
  <span
    class="flex shrink-0 items-center gap-1.5 tabular-nums"
    class:text-danger={low}
    class:text-warning={caution}
    title={known ? `${compact(used)} of ${compact(size)} context tokens used` : 'No context used yet'}
  >
    <span
      class="h-1 w-8 overflow-hidden rounded-full bg-panel-active"
      role="progressbar"
      aria-valuenow={remaining}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label="Context remaining"
    >
      <span
        class="block h-full rounded-full transition-[width] duration-300 {low
          ? 'bg-danger'
          : caution
            ? 'bg-warning'
            : 'bg-muted'}"
        style:width={`${remaining}%`}
      ></span>
    </span>
    <span>{remaining}% context</span>
  </span>
{/if}
