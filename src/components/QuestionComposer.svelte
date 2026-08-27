<script lang="ts">
  import { tick } from 'svelte';
  import { ToggleGroup } from 'bits-ui';
  import type { QuestionAnswer, QuestionRequest } from '../types';
  import MotionFly from './motion/MotionFly.svelte';

  interface Props {
    request: QuestionRequest;
    reducedMotion: boolean;
    answer: (answer: QuestionAnswer) => void;
    dismiss: () => void;
  }

  const { request, reducedMotion, answer, dismiss }: Props = $props();
  let composer = $state<HTMLElement>();
  let selectedOptionIds = $state<string[]>([]);
  let customAnswer = $state('');
  const needsSubmit = $derived(
    !request.required ||
      request.allowMultiple ||
      request.allowCustomAnswer ||
      request.options.some((option) => option.preview),
  );
  const hasAnswer = $derived(selectedOptionIds.length > 0 || customAnswer.trim().length > 0);

  const composerShell =
    'mx-auto w-[min(var(--content-width,720px),100%)] rounded-[18px] border border-line-strong bg-raised p-3.5 shadow-overlay backdrop-blur-overlay';
  const optionClass =
    'grid rounded-lg border border-line bg-panel p-[9px_10px] text-left text-text-soft hover:border-line-strong hover:bg-panel-hover focus-visible:border-line-strong focus-visible:bg-panel-hover data-[state=on]:border-line-strong data-[state=on]:bg-panel-hover';

  $effect(() => {
    void tick().then(() => {
      const target = composer?.querySelector<HTMLInputElement>('input') ??
        composer?.querySelector<HTMLButtonElement>('button');
      target?.focus();
    });
  });

  function submit() {
    if (request.required && !hasAnswer) return;
    answer({ selectedOptionIds, customAnswer: customAnswer.trim() || undefined });
  }
</script>

<svelte:window onkeydown={(event) => {
  if (event.key === 'Escape') dismiss();
}} />

<MotionFly y={reducedMotion ? 0 : 6} duration={reducedMotion ? 0 : 180}>
  <section class="composer-wrap relative z-[3] shrink-0 bg-transparent px-4 pb-3 pt-2 [.thread-view:not(.empty)_&]:pt-[22px] [.thread-view:not(.empty)_&]:before:pointer-events-none [.thread-view:not(.empty)_&]:before:absolute [.thread-view:not(.empty)_&]:before:inset-x-0 [.thread-view:not(.empty)_&]:before:top-0 [.thread-view:not(.empty)_&]:before:h-[22px] [.thread-view:not(.empty)_&]:before:bg-gradient-to-b [.thread-view:not(.empty)_&]:before:from-transparent [.thread-view:not(.empty)_&]:before:to-shell [.thread-view:not(.empty)_&]:before:content-['']">
    <form
      bind:this={composer}
      class={composerShell}
      aria-labelledby="question-title"
      onsubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <header>
        <h2 id="question-title" class="m-0 text-base font-semibold tracking-tight text-text">{request.title || 'Question'}</h2>
      </header>

      <p class="my-3 whitespace-pre-wrap text-xs leading-normal text-muted">{request.detail}</p>

      {#snippet optionItems()}
        {#each request.options as option}
          {@const selected = selectedOptionIds.includes(option.optionId)}
          <ToggleGroup.Item class={optionClass} value={option.optionId}>
            <strong class="text-[11px] font-semibold">{option.name}</strong>
            {#if option.description}<span class="mt-[3px] text-[11px] leading-snug text-muted">{option.description}</span>{/if}
            {#if selected && option.preview}<pre class="mt-2 max-h-[180px] overflow-auto rounded-md bg-recessed p-2 font-mono text-[11px] leading-snug text-text-soft whitespace-pre-wrap">{option.preview}</pre>{/if}
          </ToggleGroup.Item>
        {/each}
      {/snippet}

      {#if needsSubmit}
        {#if request.allowMultiple}
          <ToggleGroup.Root
            class="grid gap-1.5"
            aria-label="Answer options"
            type="multiple"
            value={selectedOptionIds}
            onValueChange={(value) => { selectedOptionIds = value; }}
          >
            {@render optionItems()}
          </ToggleGroup.Root>
        {:else}
          <ToggleGroup.Root
            class="grid gap-1.5"
            aria-label="Answer options"
            type="single"
            value={selectedOptionIds[0] ?? ''}
            onValueChange={(value) => { selectedOptionIds = value ? [value] : []; }}
          >
            {@render optionItems()}
          </ToggleGroup.Root>
        {/if}
      {:else}
        <ToggleGroup.Root
          class="grid gap-1.5"
          aria-label="Answer options"
          type="single"
          value={selectedOptionIds[0] ?? ''}
          onValueChange={(value) => {
            selectedOptionIds = value ? [value] : [];
            if (value) answer({ selectedOptionIds: [value] });
          }}
        >
          {@render optionItems()}
        </ToggleGroup.Root>
      {/if}

      {#if request.allowCustomAnswer}
        <label class="mt-2.5 grid gap-[5px] text-[11px] text-muted">
          <span>Other answer</span>
          <input
            bind:value={customAnswer}
            placeholder="Type another answer…"
            class="min-w-0 rounded-lg border border-line bg-panel px-2.5 py-[9px] text-text outline-none focus-visible:border-line-strong focus-visible:bg-panel-hover"
          />
        </label>
      {/if}

      <div class="mt-2.5 flex justify-end gap-1.5">
        <button
          type="button"
          class="min-h-7 rounded-[7px] border border-line bg-transparent px-2.5 py-[5px] text-[11px] text-muted hover:border-line-strong hover:bg-panel-hover hover:text-text-soft"
          onclick={dismiss}
        >Cancel</button>
        {#if needsSubmit || request.options.length === 0}
          <button
            type="submit"
            class="min-h-7 rounded-[7px] border border-line-strong bg-accent px-2.5 py-[5px] text-[11px] font-semibold text-accent-contrast hover:bg-accent-hover disabled:cursor-default disabled:opacity-[0.38]"
            disabled={request.required && !hasAnswer}
          >
            {hasAnswer ? 'Submit' : 'Skip'}
          </button>
        {/if}
      </div>
    </form>
  </section>
</MotionFly>
