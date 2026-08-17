<script lang="ts">
  import { onMount } from 'svelte';

  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, answer, decline }: Props = $props();
  let dialog = $state<HTMLDivElement>();

  const primaryOptionId = $derived(
    request.options.find((option) => option.kind?.startsWith('allow'))?.optionId,
  );

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      decline();
      return;
    }

    if (event.key !== 'Tab' || !dialog) return;
    const controls = [...dialog.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
    if (controls.length === 0) return;

    const first = controls[0];
    const last = controls.at(-1)!;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  onMount(() => {
    if (!dialog) return;
    const preferred = dialog.querySelector<HTMLButtonElement>('[data-primary="true"]');
    (preferred ?? dialog).focus();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="modal-backdrop" role="presentation">
  <div
    bind:this={dialog}
    class="permission-modal"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="permission-title"
    aria-describedby="permission-description"
    tabindex="-1"
  >
    <header class="permission-header">
      <h2 id="permission-title">{request.title}</h2>
    </header>

    <pre id="permission-description" class="permission-detail">{request.detail}</pre>

    <footer class="permission-actions">
      {#each request.options as option}
        <button
          class:primary={option.optionId === primaryOptionId}
          data-primary={option.optionId === primaryOptionId}
          title={option.name}
          onclick={() => answer(option.optionId)}
        ><span>{option.name}</span></button>
      {/each}
      {#if request.options.length === 0}
        <button class="primary" data-primary="true" onclick={decline}>Dismiss</button>
      {/if}
    </footer>
  </div>
</div>
