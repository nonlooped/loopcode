<script lang="ts">
  import { untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  interface Props {
    duration?: number;
    y?: number;
    class?: string;
    children: Snippet;
  }

  let { duration = 90, y = -2, class: className = '', children }: Props = $props();
  const opacity = new Tween(0, { duration: 0, easing: cubicOut });
  const scale = new Tween(0.97, { duration: 0, easing: cubicOut });
  const offsetY = new Tween(untrack(() => y), { duration: 0, easing: cubicOut });

  $effect(() => {
    void opacity.set(1, { duration });
    void scale.set(1, { duration });
    void offsetY.set(0, { duration });
  });
</script>

<div
  class={className}
  style:opacity={opacity.current}
  style:transform="scale({scale.current}) translateY({offsetY.current}px)"
  style:transform-origin="top left"
>
  {@render children()}
</div>
