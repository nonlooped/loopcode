<script lang="ts">
  import { Tween } from 'svelte/motion';
  import { linear } from 'svelte/easing';
  import type { Snippet } from 'svelte';

  interface Props {
    class?: string;
    children: Snippet;
  }

  let { class: className = '', children }: Props = $props();
  const rotation = new Tween(0, { duration: 0, easing: linear });

  $effect(() => {
    let frame = 0;
    let last = performance.now();
    const step = (now: number) => {
      const delta = now - last;
      last = now;
      frame = requestAnimationFrame(step);
      if (document.body.classList.contains('reduced-motion')) return;
      void rotation.set((rotation.current + (delta / 800) * 360) % 360, { duration: 0 });
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });
</script>

<span class={className} style:transform="rotate({rotation.current}deg)">
  {@render children()}
</span>
