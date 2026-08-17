<script lang="ts">
  import { onMount } from 'svelte';
  import { IconAlertTriangle } from '@tabler/icons-svelte';

  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, answer, decline }: Props = $props();
  let dialog = $state<HTMLDivElement>();

  const firstAllowOptionId = $derived(request.options.find((option) => option.kind?.startsWith('allow'))?.optionId);

  function isAllow(kind?: string) {
    return kind?.startsWith('allow') ?? false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      decline();
      return;
    }

    if (event.key !== 'Tab') return;
    if (!dialog) return;
    const controls = [...dialog.querySelectorAll<HTMLElement>('button:not(:disabled), [tabindex]:not([tabindex="-1"])')];
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
    const preferred = dialog.querySelector<HTMLElement>('[data-primary="true"]');
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
      <span class="permission-icon" aria-hidden="true"><IconAlertTriangle size={16} stroke={1.7} /></span>
      <div class="permission-heading">
        <span class="permission-label">Permission request</span>
        <h2 id="permission-title">{request.title}</h2>
      </div>
    </header>

    <section class="permission-detail" aria-label="Request details">
      <span class="permission-detail-label">Request details</span>
      <pre id="permission-description">{request.detail}</pre>
    </section>

    <div class="permission-actions">
      {#each request.options as option}
        <button
          class:approve={isAllow(option.kind)}
          class:primary={option.optionId === firstAllowOptionId}
          class:reject={!isAllow(option.kind)}
          data-primary={option.optionId === firstAllowOptionId}
          onclick={() => answer(option.optionId)}
        >{option.name}</button>
      {/each}
      {#if request.options.length === 0}<button class="primary" data-primary="true" onclick={decline}>Dismiss</button>{/if}
    </div>
  </div>
</div>
