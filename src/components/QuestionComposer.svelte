<script lang="ts">
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { ToggleGroup } from 'bits-ui';
  import type { QuestionAnswer, QuestionRequest } from '../types';

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

<section
  class="relative z-3 shrink-0 px-4 pt-2 pb-3"
  in:fly|global={{ y: reducedMotion ? 0 : 6, duration: reducedMotion ? 0 : 180 }}
>
  <form
    bind:this={composer}
    class="mx-auto w-[min(var(--content-width,720px),100%)] rounded-[18px] border border-line-strong bg-raised p-3.5 shadow-overlay backdrop-blur-xl [&_h2]:text-base [&_h2]:leading-[1.4] [&_h2]:font-semibold [&_h2]:tracking-[-.01em]"
    aria-labelledby="question-title"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <header>
      <h2 id="question-title">{request.title || 'Question'}</h2>
    </header>

    <p class="my-3 text-xs leading-6 whitespace-pre-wrap text-muted">{request.detail}</p>

    {#snippet optionItems()}
      {#each request.options as option}
        {@const selected = selectedOptionIds.includes(option.optionId)}
        <ToggleGroup.Item class="grid rounded-lg border border-line bg-panel px-2.5 py-2 text-left text-ink-soft hover:border-line-strong hover:bg-panel-hover data-[state=on]:border-line-strong data-[state=on]:bg-panel-hover [&_strong]:text-[11px] [&_strong]:font-semibold [&_span]:mt-0.5 [&_span]:text-[11px] [&_span]:leading-[1.45] [&_span]:text-muted [&_pre]:mt-2 [&_pre]:max-h-[180px] [&_pre]:overflow-auto [&_pre]:rounded-md [&_pre]:bg-recessed [&_pre]:p-2 [&_pre]:font-mono [&_pre]:text-[11px] [&_pre]:leading-[1.45] [&_pre]:whitespace-pre-wrap" value={option.optionId}>
          <strong>{option.name}</strong>
          {#if option.description}<span>{option.description}</span>{/if}
          {#if selected && option.preview}<pre>{option.preview}</pre>{/if}
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
      <label class="mt-2.5 grid gap-1 text-[11px] text-muted [&_input]:min-w-0 [&_input]:rounded-lg [&_input]:border [&_input]:border-line [&_input]:bg-panel [&_input]:px-2.5 [&_input]:py-2 [&_input]:text-ink [&_input]:outline-none [&_input:focus-visible]:border-line-strong [&_input:focus-visible]:bg-panel-hover">
        <span>Other answer</span>
        <input bind:value={customAnswer} placeholder="Type another answer…" />
      </label>
    {/if}

    <div class="mt-2.5 flex justify-end gap-1.5">
      <button type="button" class="min-h-7 rounded-md border border-line px-2.5 py-1 text-[11px] text-muted hover:border-line-strong hover:bg-panel-hover hover:text-ink-soft" onclick={dismiss}>Cancel</button>
      {#if needsSubmit || request.options.length === 0}
        <button
          type="submit"
          class="min-h-7 rounded-md border border-line-strong bg-accent px-2.5 py-1 text-[11px] font-semibold text-accent-contrast hover:bg-accent-hover disabled:cursor-default disabled:opacity-40"
          disabled={request.required && !hasAnswer}
        >
          {hasAnswer ? 'Submit' : 'Skip'}
        </button>
      {/if}
    </div>
  </form>
</section>
