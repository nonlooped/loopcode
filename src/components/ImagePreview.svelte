<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import IconX from '@tabler/icons-svelte/icons/x';

  interface Props {
    src: string;
    name: string;
    reducedMotion: boolean;
    close: () => void;
  }

  const { src, name, reducedMotion, close }: Props = $props();
</script>

<div class="image-preview" role="dialog" aria-modal="true" aria-label={name}>
  <button
    class="image-preview-backdrop"
    aria-label="Close image preview"
    onclick={close}
    transition:fade|global={{ duration: reducedMotion ? 0 : 150 }}
  ></button>
  <figure transition:scale|global={{ start: reducedMotion ? 1 : 0.985, duration: reducedMotion ? 0 : 170 }}>
    <img {src} alt={name} />
    <button class="image-preview-close" aria-label="Close image preview" onclick={close}><IconX size={16} stroke={1.8} /></button>
    <figcaption>{name}</figcaption>
  </figure>
</div>

<svelte:window onkeydown={(event) => { if (event.key === 'Escape') close(); }} />
