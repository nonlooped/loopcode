<script lang="ts">
  import { untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  interface Props {
    show?: boolean;
    enter?: boolean;
    y?: number;
    duration?: number;
    class?: string;
    children: Snippet;
  }

  let {
    show = true,
    enter = show,
    y = 4,
    duration = 180,
    class: className = '',
    children,
  }: Props = $props();

  let mounted = $state(untrack(() => enter));
  const opacity = new Tween(untrack(() => enter ? 1 : 0), { duration: 0, easing: cubicOut });
  const offsetY = new Tween(untrack(() => enter ? 0 : y), { duration: 0, easing: cubicOut });

  $effect(() => {
    if (show) {
      mounted = true;
      void opacity.set(1, { duration });
      void offsetY.set(0, { duration });
      return;
    }
    void Promise.all([
      opacity.set(0, { duration }),
      offsetY.set(y, { duration }),
    ]).then(() => {
      if (!show) mounted = false;
    });
  });
</script>

{#if mounted}
  <div
    class={className}
    style:opacity={opacity.current}
    style:transform="translateY({offsetY.current}px)"
  >
    {@render children()}
  </div>
{/if}
