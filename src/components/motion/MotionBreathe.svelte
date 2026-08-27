<script lang="ts">
  import { untrack } from 'svelte';
  import { Spring } from 'svelte/motion';
  import type { Snippet } from 'svelte';

  interface Props {
    min?: number;
    max?: number;
    class?: string;
    children?: Snippet;
  }

  let { min = 0.35, max = 1, class: className = '', children }: Props = $props();
  const pulse = new Spring(untrack(() => min), { stiffness: 0.04, damping: 0.4 });
  let direction = 1;

  $effect(() => {
    const interval = window.setInterval(() => {
      direction *= -1;
      pulse.target = document.body.classList.contains('reduced-motion') ? max : direction > 0 ? max : min;
    }, 700);
    pulse.target = max;
    return () => window.clearInterval(interval);
  });
</script>

<span class={className} style:opacity={pulse.current}>
  {#if children}{@render children()}{/if}
</span>
