<script lang="ts">
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
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

  function choose(optionId: string) {
    if (!needsSubmit) {
      answer({ selectedOptionIds: [optionId] });
      return;
    }
    selectedOptionIds = request.allowMultiple
      ? selectedOptionIds.includes(optionId)
        ? selectedOptionIds.filter((id) => id !== optionId)
        : [...selectedOptionIds, optionId]
      : [optionId];
  }

  function submit() {
    if (request.required && !hasAnswer) return;
    answer({ selectedOptionIds, customAnswer: customAnswer.trim() || undefined });
  }
</script>

<svelte:window onkeydown={(event) => {
  if (event.key === 'Escape') dismiss();
}} />

<section
  class="composer-wrap"
  in:fly|global={{ y: reducedMotion ? 0 : 6, duration: reducedMotion ? 0 : 180 }}
>
  <form
    bind:this={composer}
    class="question-composer"
    aria-labelledby="question-title"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <header class="question-composer-header">
      <h2 id="question-title">{request.title || 'Question'}</h2>
    </header>

    <p class="question-composer-detail">{request.detail}</p>

    <div class="question-composer-options" role="group" aria-label="Answer options">
      {#each request.options as option}
        {@const selected = selectedOptionIds.includes(option.optionId)}
        <button
          type="button"
          class:selected
          class="question-composer-option"
          aria-pressed={needsSubmit ? selected : undefined}
          onclick={() => choose(option.optionId)}
        >
          <strong>{option.name}</strong>
          {#if option.description}<span>{option.description}</span>{/if}
          {#if selected && option.preview}<pre>{option.preview}</pre>{/if}
        </button>
      {/each}
    </div>

    {#if request.allowCustomAnswer}
      <label class="question-composer-custom">
        <span>Other answer</span>
        <input bind:value={customAnswer} placeholder="Type another answer…" />
      </label>
    {/if}

    <div class="question-composer-actions">
      <button type="button" class="question-composer-cancel" onclick={dismiss}>Cancel</button>
      {#if needsSubmit || request.options.length === 0}
        <button
          type="submit"
          class="question-composer-submit"
          disabled={request.required && !hasAnswer}
        >
          {hasAnswer ? 'Submit' : 'Skip'}
        </button>
      {/if}
    </div>
  </form>
</section>
