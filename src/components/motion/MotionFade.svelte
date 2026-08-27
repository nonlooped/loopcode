<script lang="ts">
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  interface Props {
    show: boolean;
    duration?: number;
    class?: string;
    children: Snippet;
  }

  let { show, duration = 150, class: className = '', children }: Props = $props();
  let mounted = $state(false);
  const opacity = new Tween(0, { duration: 0, easing: cubicOut });

  $effect(() => {
    if (show) {
      mounted = true;
      void opacity.set(1, { duration });
      return;
    }
    void opacity.set(0, { duration }).then(() => {
      if (!show) mounted = false;
    });
  });
</script>

{#if mounted}
  <div class={className} style:opacity={opacity.current}>
    {@render children()}
  </div>
{/if}
