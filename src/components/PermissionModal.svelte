<script lang="ts">
  import { IconAlertTriangle } from '@tabler/icons-svelte';

  import type { PermissionRequest } from '../types';

  interface Props {
    request: PermissionRequest;
    answer: (optionId: string) => void;
    decline: () => void;
  }

  const { request, answer, decline }: Props = $props();
</script>

<div class="modal-backdrop" role="presentation">
  <div class="permission-modal" role="dialog" aria-modal="true" aria-labelledby="permission-title" tabindex="-1">
    <span class="permission-icon"><IconAlertTriangle size={20} stroke={1.8} /></span>
    <div>
      <h2 id="permission-title">{request.title}</h2>
      <pre>{request.detail}</pre>
    </div>
    <div class="permission-actions">
      {#each request.options as option}
        <button class:approve={option.kind?.startsWith('allow')} onclick={() => answer(option.optionId)}>{option.name}</button>
      {/each}
      {#if request.options.length === 0}<button onclick={decline}>Cancel request</button>{/if}
    </div>
  </div>
</div>
