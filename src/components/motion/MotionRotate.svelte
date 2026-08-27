<script lang="ts">
  import { untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  interface Props {
    active: boolean;
    degrees?: number;
    duration?: number;
    reducedMotion?: boolean;
    class?: string;
    children: Snippet;
  }

  let {
    active,
    degrees = 90,
    duration = 140,
    reducedMotion = false,
    class: className = '',
    children,
  }: Props = $props();

  const rotation = new Tween(untrack(() => active ? degrees : 0), { duration: 0, easing: cubicOut });

  $effect(() => {
    void rotation.set(active ? degrees : 0, { duration: reducedMotion ? 0 : duration });
  });
</script>

<span class="inline-flex {className}" style:transform="rotate({rotation.current}deg)">
  {@render children()}
</span>
