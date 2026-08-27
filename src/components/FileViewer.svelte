<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { IconArrowLeft, IconArrowRight, IconX } from '@tabler/icons-svelte';

  import { readProjectFile } from '../services/native';
  import { documentTypeForPath } from '../utils/file-preview';
  import { highlightFile, languageForPath } from '../utils/syntax-highlighter';
  import MarkdownMessage from './markdown/MarkdownMessage.svelte';

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
  let content = $state('');
  let highlighted = $state('');
  let imageUrl = $state('');
  let loadToken = 0;

  const name = $derived(path.split(/[\\/]/).pop() || path);
  const mediaType = $derived(imageMediaType(path));
  const language = $derived(languageForPath(path) || 'Plain text');
  const documentType = $derived(documentTypeForPath(path));

  $effect(() => {
    const request = [projectRoot, path, revision] as const;
    untrack(() => { void loadFile(...request); });
  });

  onDestroy(clearImage);

  async function loadFile(requestedRoot: string, requestedPath: string, requestedRevision: number) {
    const token = ++loadToken;
    void requestedRevision;
    loading = true;
    error = '';
    content = '';
    highlighted = '';
    clearImage();
    try {
      const bytes = new Uint8Array(await readProjectFile(requestedRoot, requestedPath));
      if (token !== loadToken) return;
      const requestedMediaType = imageMediaType(requestedPath);
      if (requestedMediaType) {
        imageUrl = URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: requestedMediaType }));
      } else {
        let decodedContent: string;
        try {
          decodedContent = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
        } catch {
          throw new Error('Binary files cannot be previewed.');
        }
        if (documentTypeForPath(requestedPath)) {
          content = decodedContent;
        } else {
          highlighted = highlightFile(decodedContent, requestedPath);
        }
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

<section class="flex min-h-0 min-w-0 flex-1 flex-col" aria-label={`File viewer: ${name}`}>
  <header class="grid min-h-[39px] grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 border-b border-line px-2.5 py-1.5">
    <div class="flex gap-0.5">
      <button type="button" class="grid size-[25px] place-items-center rounded-md border-0 bg-transparent text-muted hover:bg-panel-hover hover:text-text-soft" aria-label="Back" title="Back" onclick={back}>
        <IconArrowLeft size={15} stroke={1.55} />
      </button>
      <button
        type="button"
        class="grid size-[25px] place-items-center rounded-md border-0 bg-transparent text-muted hover:bg-panel-hover hover:text-text-soft disabled:opacity-[0.35]"
        aria-label="Forward"
        title="Forward"
        disabled={!canGoForward}
        onclick={forward}
      >
        <IconArrowRight size={15} stroke={1.55} />
      </button>
    </div>
    <div class="flex min-w-0 items-baseline justify-center gap-2 overflow-hidden whitespace-nowrap" title={path}>
      <strong class="truncate text-xs font-semibold text-text-soft">{name}</strong>
      {#if !mediaType}<small class="truncate text-[11px] text-faint">{language}</small>{/if}
    </div>
    <button type="button" class="grid size-[25px] place-items-center rounded-md border-0 bg-transparent text-muted hover:bg-panel-hover hover:text-text-soft" aria-label="Close file viewer" title="Back to conversation" onclick={close}>
      <IconX size={15} stroke={1.55} />
    </button>
  </header>

  <div class="min-h-0 min-w-0 flex-1 overflow-auto">
    {#if loading}
      <p class="m-0 p-6 text-xs text-faint">Loading file…</p>
    {:else if error}
      <p class="m-0 p-6 text-xs text-danger">{error}</p>
    {:else if imageUrl}
      <div class="grid min-h-full min-w-full place-items-center p-6">
        <img class="block max-h-full max-w-full object-contain" src={imageUrl} alt={name} />
      </div>
    {:else if documentType === 'markdown'}
      <article class="message-body mx-auto w-[min(100%,816px)] px-[clamp(15px,5.8vw,48px)] pt-8 pb-14 [&_img]:max-w-full">
        <MarkdownMessage id={`file:${path}`} source={content} />
      </article>
    {:else if documentType === 'html'}
      <iframe class="block h-full w-full border-0 bg-shell [color-scheme:inherit]" title={`Preview of ${name}`} sandbox="" srcdoc={content}></iframe>
    {:else}
      <div class="[&_.line]:block [&_.line]:min-h-[1.6em] [&_.line]:pl-[52px] [&_.line]:indent-[-52px] [&_.line]:pr-5 [&_.line]:before:inline-block [&_.line]:before:w-[52px] [&_.line]:before:pr-3.5 [&_.line]:before:text-right [&_.line]:before:text-faint [&_.line]:before:content-[counter(file-line)] [&_.line]:before:select-none [&_code]:flex [&_code]:flex-col [&_code]:[counter-reset:file-line] [&_pre.shiki]:min-h-full [&_pre.shiki]:w-full [&_pre.shiki]:bg-transparent! [&_pre.shiki]:py-3.5 [&_pre.shiki]:pb-8 [&_pre.shiki]:font-mono [&_pre.shiki]:text-xs [&_pre.shiki]:leading-relaxed [&_pre.shiki]:break-anywhere [&_pre.shiki]:whitespace-pre-wrap [&_pre.shiki-fallback]:min-h-full [&_pre.shiki-fallback]:w-full [&_pre.shiki-fallback]:bg-transparent! [&_pre.shiki-fallback]:py-3.5 [&_pre.shiki-fallback]:pb-8 [&_pre.shiki-fallback]:font-mono [&_pre.shiki-fallback]:text-xs [&_pre.shiki-fallback]:leading-relaxed [&_pre.shiki-fallback]:break-anywhere [&_pre.shiki-fallback]:whitespace-pre-wrap">{@html highlighted}</div>
    {/if}
  </div>
</section>
