<script lang="ts">
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    reducedMotion: boolean;
    answer: (optionId: string) => void;
    dismiss: () => void;
  }

  const { request, reducedMotion, answer, dismiss }: Props = $props();
  let composer = $state<HTMLElement>();

  $effect(() => {
    const requestId = request.requestId;
    void requestId;
    void tick().then(() => composer?.querySelector<HTMLButtonElement>('button')?.focus());
  });
</script>

<section
  class="composer-wrap"
  in:fly|global={{ y: reducedMotion ? 0 : 6, duration: reducedMotion ? 0 : 180 }}
>
  <section
    bind:this={composer}
    class="question-composer"
    aria-labelledby="question-title"
  >
    <header class="question-composer-header">
      <h2 id="question-title">Question</h2>
    </header>

    <p class="question-composer-detail">{request.detail}</p>

    <div class="question-composer-options">
      {#each request.options as option}
        <button class="question-composer-option" onclick={() => answer(option.optionId)}>
          <strong>{option.name}</strong>
          {#if option.description}<span>{option.description}</span>{/if}
        </button>
      {:else}
        <button class="question-composer-option" onclick={dismiss}>Dismiss</button>
      {/each}
    </div>
  </section>
</section>
