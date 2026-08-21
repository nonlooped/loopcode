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

    {#snippet optionItems()}
      {#each request.options as option}
        {@const selected = selectedOptionIds.includes(option.optionId)}
        <ToggleGroup.Item class="question-composer-option" value={option.optionId}>
          <strong>{option.name}</strong>
          {#if option.description}<span>{option.description}</span>{/if}
          {#if selected && option.preview}<pre>{option.preview}</pre>{/if}
        </ToggleGroup.Item>
      {/each}
    {/snippet}

    {#if needsSubmit}
      {#if request.allowMultiple}
        <ToggleGroup.Root
          class="question-composer-options"
          aria-label="Answer options"
          type="multiple"
          value={selectedOptionIds}
          onValueChange={(value) => { selectedOptionIds = value; }}
        >
          {@render optionItems()}
        </ToggleGroup.Root>
      {:else}
        <ToggleGroup.Root
          class="question-composer-options"
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
        class="question-composer-options"
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
