<script lang="ts">
  import { onDestroy } from 'svelte';

  import ContextMenu from './ContextMenu.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import { readAttachment } from '../services/native';
  import type { MessageImage } from '../types';
  import { copyImage, saveImage } from '../utils/clipboard';
  import { menuFromEvent, type ContextMenuState } from '../utils/context-menu';

  interface Props {
    image: MessageImage;
    reducedMotion: boolean;
  }

  const { image, reducedMotion }: Props = $props();
  let src = $state('');
  let error = $state(false);
  let contextMenu = $state<ContextMenuState>();
  let previewOpen = $state(false);
  let loadToken = 0;

  $effect(() => {
    void load(image);
  });

  onDestroy(clear);

  async function load(requested: MessageImage) {
    const token = ++loadToken;
    clearUrl();
    error = false;
    if ('data' in requested) {
      src = `data:${requested.mimeType};base64,${requested.data}`;
      return;
    }
    try {
      const bytes = new Uint8Array(await readAttachment(requested.attachmentId));
      if (token !== loadToken) return;
      src = URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: requested.mimeType }));
    } catch {
      if (token === loadToken) error = true;
    }
  }

  function clearUrl() {
    if (src.startsWith('blob:')) URL.revokeObjectURL(src);
    src = '';
  }

  function clear() {
    loadToken += 1;
    clearUrl();
  }

  function openMenu(event: MouseEvent) {
    contextMenu = menuFromEvent(event, [
      { label: 'Open preview', action: () => { previewOpen = true; } },
      { label: 'Copy image', action: () => copyImage(src) },
      { label: 'Save image', action: () => saveImage(src, image.name) },
    ]);
  }
</script>

{#if src}
  <img {src} alt={image.name} title={image.name} oncontextmenu={openMenu} />
{:else if error}
  <span class="missing-image" role="img" aria-label={`${image.name} is unavailable`}>Image unavailable</span>
{/if}

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}

{#if previewOpen}
  <ImagePreview {src} name={image.name} {reducedMotion} close={() => { previewOpen = false; }} />
{/if}

<style>
  .missing-image {
    padding: 10px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    color: var(--text-dim);
    font-size: 12px;
  }
</style>
