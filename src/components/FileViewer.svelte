<script lang="ts">
  import { onDestroy } from 'svelte';
  import IconArrowLeft from '@tabler/icons-svelte/icons/arrow-left';
  import IconArrowRight from '@tabler/icons-svelte/icons/arrow-right';
  import IconX from '@tabler/icons-svelte/icons/x';

  import { readProjectFile } from '../services/native';
  import { highlightFile, languageForPath } from '../utils/syntax-highlighter';

  interface Props {
    path: string;
    projectRoot: string;
    revision: number;
    canGoForward: boolean;
    back: () => void;
    forward: () => void;
    close: () => void;
  }

  const { path, projectRoot, revision, canGoForward, back, forward, close }: Props = $props();
  let loading = $state(true);
  let error = $state('');
  let highlighted = $state('');
  let imageUrl = $state('');
  let loadToken = 0;

  const name = $derived(path.split(/[\\/]/).pop() || path);
  const mediaType = $derived(imageMediaType(path));
  const language = $derived(languageForPath(path) || 'Plain text');

  $effect(() => {
    void loadFile(path, revision);
  });

  onDestroy(clearImage);

  async function loadFile(requestedPath: string, requestedRevision: number) {
    const token = ++loadToken;
    void requestedRevision;
    loading = true;
    error = '';
    highlighted = '';
    clearImage();
    try {
      const bytes = new Uint8Array(await readProjectFile(projectRoot, requestedPath));
      if (token !== loadToken) return;
      const requestedMediaType = imageMediaType(requestedPath);
      if (requestedMediaType) {
        imageUrl = URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: requestedMediaType }));
      } else {
        let content: string;
        try {
          content = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
        } catch {
          throw new Error('Binary files cannot be previewed.');
        }
        const rendered = await highlightFile(content, requestedPath);
        if (token !== loadToken) return;
        highlighted = rendered;
      }
    } catch (cause) {
      if (token === loadToken) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (token === loadToken) loading = false;
    }
  }

  function clearImage() {
    if (!imageUrl) return;
    URL.revokeObjectURL(imageUrl);
    imageUrl = '';
  }

  function imageMediaType(filePath: string): string {
    const extension = filePath.split('.').pop()?.toLowerCase();
    return {
      avif: 'image/avif', bmp: 'image/bmp', gif: 'image/gif', ico: 'image/x-icon',
      jpeg: 'image/jpeg', jpg: 'image/jpeg', png: 'image/png', svg: 'image/svg+xml', webp: 'image/webp',
    }[extension ?? ''] ?? '';
  }
</script>

<section class="file-viewer" aria-label={`File viewer: ${name}`}>
  <header class="file-viewer-toolbar">
    <div class="file-viewer-navigation">
      <button type="button" class="chrome-button" aria-label="Back" title="Back" onclick={back}>
        <IconArrowLeft size={15} stroke={1.6} />
      </button>
      <button
        type="button"
        class="chrome-button"
        aria-label="Forward"
        title="Forward"
        disabled={!canGoForward}
        onclick={forward}
      >
        <IconArrowRight size={15} stroke={1.6} />
      </button>
    </div>
    <div class="file-viewer-title" title={path}>
      <strong>{name}</strong>
      {#if !mediaType}<small>{language}</small>{/if}
    </div>
    <button type="button" class="chrome-button" aria-label="Close file viewer" title="Back to conversation" onclick={close}>
      <IconX size={15} stroke={1.6} />
    </button>
  </header>

  <div class="file-viewer-content">
    {#if loading}
      <p class="file-viewer-state">Loading file…</p>
    {:else if error}
      <p class="file-viewer-state error">{error}</p>
    {:else if imageUrl}
      <div class="file-viewer-image"><img src={imageUrl} alt={name} /></div>
    {:else}
      <div class="file-viewer-code">{@html highlighted}</div>
    {/if}
  </div>
</section>
