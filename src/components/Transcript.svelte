<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { IconAlertTriangle, IconChevronDown, IconChevronRight, IconTool } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import MarkdownMessage from './markdown/MarkdownMessage.svelte';
  import type { MessageImage, ThreadState, TimelineMessage, ToolActivity } from '../types';
  import type { TimelineDisplayEntry } from '../types/timeline';
  import { copyImage, copyText, saveImage } from '../utils/clipboard';
  import { menuFromEvent, type ContextMenuState } from '../utils/context-menu';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';
  import { isStreamingMessage, workGroupMeta } from '../utils/timeline';
  import { threadHarness, threadStatus } from '../utils/threads';

  interface Props {
    thread: ThreadState;
    entries: TimelineDisplayEntry[];
    reducedMotion: boolean;
  }

  const { thread, entries, reducedMotion }: Props = $props();
  let transcriptElement = $state<HTMLElement>();
  let canScrollUp = $state(false);
  let canScrollDown = $state(false);
  let pinnedToBottom = true;
  let renderedThreadId: string | undefined;
  let animateEntries = $state(false);
  let contextMenu = $state<ContextMenuState>();
  let imagePreview = $state<{ src: string; name: string }>();
  const entryMotion = $derived(animateEntries && !reducedMotion);

  onMount(() => {
    const frame = requestAnimationFrame(() => { animateEntries = true; });
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    const updatedAt = thread.updatedAt;
    const threadId = thread.id;
    const transcript = transcriptElement;
    if (updatedAt >= 0 && transcript) {
      const threadChanged = renderedThreadId !== threadId;
      const shouldFollow = threadChanged || pinnedToBottom;
      renderedThreadId = threadId;
      void tick().then(() => {
        if (!transcriptElement) return;
        if (shouldFollow) scrollToBottom('auto');
        else updateScrollState();
      });
    }
  });

  function updateScrollState() {
    if (!transcriptElement) return;
    const distanceFromBottom = Math.max(
      0,
      transcriptElement.scrollHeight - transcriptElement.clientHeight - transcriptElement.scrollTop,
    );
    pinnedToBottom = distanceFromBottom <= 36;
    canScrollUp = transcriptElement.scrollTop > 12;
    canScrollDown = distanceFromBottom > 36;
  }

  function scrollToBottom(behavior: ScrollBehavior = 'smooth') {
    if (!transcriptElement) return;
    pinnedToBottom = true;
    transcriptElement.scrollTo({
      top: transcriptElement.scrollHeight,
      behavior: reducedMotion ? 'auto' : behavior,
    });
    if (behavior === 'auto' || reducedMotion) updateScrollState();
  }

  function imageUrl(image: { mimeType: string; data: string }) {
    return `data:${image.mimeType};base64,${image.data}`;
  }

  function toolStatus(status: string) {
    return status.replaceAll('_', ' ');
  }

  function openMessageMenu(event: MouseEvent, message: TimelineMessage) {
    const renderedText = (event.currentTarget as HTMLElement).innerText.trim();
    contextMenu = menuFromEvent(event, [
      { label: 'Copy message', action: () => copyText(renderedText || message.text) },
      { label: 'Copy as Markdown', action: () => copyText(message.text) },
    ]);
  }

  function openToolMenu(event: MouseEvent, tool: ToolActivity) {
    const details = event.currentTarget as HTMLDetailsElement;
    contextMenu = menuFromEvent(event, [
      { label: details.open ? 'Collapse' : 'Expand', action: () => { details.open = !details.open; } },
      { label: 'Copy details', action: () => copyText(tool.detail ?? ''), disabled: !tool.detail },
      {
        label: tool.locations.length === 1 ? 'Copy location' : 'Copy locations',
        action: () => copyText(tool.locations.join('\n')),
        disabled: tool.locations.length === 0,
      },
    ]);
  }

  function openLocationMenu(event: MouseEvent, location: string) {
    contextMenu = menuFromEvent(event, [{ label: 'Copy path', action: () => copyText(location) }]);
  }

  function openImageMenu(event: MouseEvent, image: MessageImage, src: string) {
    contextMenu = menuFromEvent(event, [
      { label: 'Open preview', action: () => { imagePreview = { src, name: image.name }; } },
      { label: 'Copy image', action: () => copyImage(src) },
      { label: 'Save image', action: () => saveImage(src, image.name) },
    ]);
  }
</script>

<div class="transcript-shell" in:fade|global={{ duration: reducedMotion ? 0 : 150 }}>
  <section
    class:can-scroll-up={canScrollUp}
    class:can-scroll-down={canScrollDown}
    class="transcript"
    bind:this={transcriptElement}
    aria-live="polite"
    onscroll={updateScrollState}
  >
    <div class="message-stack" class:empty={thread.messages.length === 0 && thread.tools.length === 0}>
    {#each entries as entry (entry.type === 'message' ? `message-${entry.message.id}` : entry.id)}
      {#if entry.type === 'work'}
        <details
          class:active={entry.active}
          class="work-group"
          in:fly={{ y: entryMotion ? 4 : 0, duration: entryMotion ? 180 : 0 }}
        >
          <summary>
            <span class="work-chevron"><IconChevronRight size={14} stroke={1.8} /></span>
            <span class="activity-spark"></span>
            <strong>{entry.active ? 'Working…' : 'Working'}</strong>
            <small>{workGroupMeta(entry.entries)}</small>
          </summary>
          <div class="work-group-content">
            {#each entry.entries as workEntry (workEntry.type === 'tool' ? `tool-${workEntry.tool.id}` : `message-${workEntry.message.id}`)}
              {#if workEntry.type === 'tool'}
                {@const tool = workEntry.tool}
                <details
                  class="tool-item"
                  open={tool.status === 'in_progress'}
                  oncontextmenu={(event) => openToolMenu(event, tool)}
                  in:fly={{ y: entryMotion ? 3 : 0, duration: entryMotion ? 160 : 0 }}
                >
                  <summary>
                    <span class="tool-icon"><IconTool size={14} stroke={1.8} /></span>
                    <span class="tool-title"><strong>{tool.title}</strong><small>{toolStatus(tool.status)}</small></span>
                  </summary>
                  {#if tool.detail}<pre>{tool.detail}</pre>{/if}
                  {#if tool.locations.length > 0}
                    <div class="tool-locations">{#each tool.locations as location}<span role="presentation" oncontextmenu={(event) => openLocationMenu(event, location)}>{location}</span>{/each}</div>
                  {/if}
                </details>
              {:else}
                <div
                  role="presentation"
                  class:thought={workEntry.message.role === 'thought'}
                  class="work-message"
                  oncontextmenu={(event) => openMessageMenu(event, workEntry.message)}
                  in:fly={{ y: entryMotion ? 3 : 0, duration: entryMotion ? 160 : 0 }}
                >
                  <MarkdownMessage id={workEntry.message.id} source={workEntry.message.text.trim()} streaming={entry.active} />
                </div>
              {/if}
            {/each}
          </div>
        </details>
      {:else}
        {@const message = entry.message}
        {#if message.role === 'notice' || message.role === 'error'}
          <div
            class:error={message.role === 'error'}
            class="notice-message"
            in:fly={{ y: entryMotion ? 4 : 0, duration: entryMotion ? 170 : 0 }}
          >
            {#if message.role === 'error'}<IconAlertTriangle size={15} stroke={1.8} />{/if}
            <span>{message.text}</span>
          </div>
        {:else}
          {@const streaming = isStreamingMessage(thread, message)}
          <article
            class:from-user={message.role === 'user'}
            class:streaming={streaming}
            class="message"
            oncontextmenu={(event) => openMessageMenu(event, message)}
            in:fly={{ y: entryMotion ? (message.role === 'user' ? 10 : 4) : 0, duration: entryMotion ? 180 : 0 }}
          >
            <header>{message.role === 'user' ? 'You' : threadHarness(thread)}</header>
            <div class="message-body">
              {#if message.role === 'user' && message.content}
                <div class="message-prompt-content">
                  {#each message.content as part}
                    {#if part.type === 'text'}
                      <span>{part.text}</span>
                    {:else}
                      {@const reference = part.reference}
                      <span class:skill={reference.kind === 'skill'} class="message-reference" title={reference.relativePath}>
                        {#if reference.kind === 'skill'}
                          <span class="composer-reference-mark">$</span>
                        {:else}
                          {@const icon = reference.kind === 'folder' ? materialFolderIcon(reference.name, false) : materialFileIcon(reference.name)}
                          {#if icon}<img src={icon} alt="" />{/if}
                        {/if}
                        <span>{reference.name}</span>
                      </span>
                    {/if}
                  {/each}
                </div>
              {:else}
                <MarkdownMessage id={message.id} source={message.text} {streaming} />
              {/if}
              {#if message.images && message.images.length > 0}
                <div class="message-images" aria-label="Attached images">
                  {#each message.images as image, index (`${message.id}-image-${index}`)}
                    {@const src = imageUrl(image)}
                    <button
                      type="button"
                      class="message-image-button"
                      aria-label={`Preview ${image.name}`}
                      title={image.name}
                      onclick={() => { imagePreview = { src, name: image.name }; }}
                      oncontextmenu={(event) => openImageMenu(event, image, src)}
                    ><img {src} alt="" /></button>
                  {/each}
                </div>
              {/if}
            </div>
          </article>
        {/if}
      {/if}
    {/each}
    {#if threadStatus(thread) === 'running' && !entries.some((entry) => entry.type === 'work' && entry.active)}
      <div class="running-indicator" in:fade={{ duration: entryMotion ? 140 : 0 }}><span class="activity-spark"></span><span>Working…</span></div>
    {/if}
    </div>
  </section>

  {#if canScrollDown}
    <button
      class="scroll-to-bottom"
      type="button"
      aria-label="Scroll to latest message"
      onclick={() => scrollToBottom()}
      transition:fade={{ duration: reducedMotion ? 0 : 120 }}
    >
      <IconChevronDown size={13} stroke={1.8} />
      <span>Scroll to bottom</span>
    </button>
  {/if}
</div>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}

{#if imagePreview}
  <ImagePreview
    src={imagePreview.src}
    name={imagePreview.name}
    {reducedMotion}
    close={() => { imagePreview = undefined; }}
  />
{/if}
