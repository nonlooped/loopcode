<script lang="ts">
  import { tick } from 'svelte';
  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    dismiss: () => void;
  }

  const { request, answer, dismiss }: Props = $props();
  let composer = $state<HTMLElement>();

  $effect(() => {
    const requestId = request.requestId;
    void requestId;
    void tick().then(() => composer?.querySelector<HTMLButtonElement>('button')?.focus());
  });
</script>

<section class="composer-wrap">
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
