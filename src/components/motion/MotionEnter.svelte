<script lang="ts">
  import { untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  interface Props {
    y?: number;
    duration?: number;
    class?: string;
    children: Snippet;
  }

  let { y = 4, duration = 180, class: className = '', children }: Props = $props();
  const opacity = new Tween(0, { duration: 0, easing: cubicOut });
  const offsetY = new Tween(untrack(() => y), { duration: 0, easing: cubicOut });

  $effect(() => {
    const ms = document.body.classList.contains('reduced-motion') ? 0 : duration;
    void opacity.set(1, { duration: ms });
    void offsetY.set(0, { duration: ms });
  });
</script>

<div
  class={className}
  style:opacity={opacity.current}
  style:transform="translateY({offsetY.current}px)"
>
  {@render children()}
</div>
