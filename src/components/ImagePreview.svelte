<script lang="ts">
  import { IconX } from '@tabler/icons-svelte';

  interface Props {
    src: string;
    name: string;
    close: () => void;
  }

  const { src, name, close }: Props = $props();
</script>

<div class="image-preview" role="dialog" aria-modal="true" aria-label={name}>
  <button class="image-preview-backdrop" aria-label="Close image preview" onclick={close}></button>
  <figure>
    <img {src} alt={name} />
    <button class="image-preview-close" aria-label="Close image preview" onclick={close}><IconX size={16} stroke={1.8} /></button>
    <figcaption>{name}</figcaption>
  </figure>
</div>

<svelte:window onkeydown={(event) => { if (event.key === 'Escape') close(); }} />
