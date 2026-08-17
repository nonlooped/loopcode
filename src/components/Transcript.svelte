<script lang="ts">
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { IconAlertTriangle, IconChevronDown, IconChevronRight, IconTool } from '@tabler/icons-svelte';

  import MarkdownMessage from './markdown/MarkdownMessage.svelte';
  import type { ThreadState } from '../types';
  import type { TimelineDisplayEntry } from '../types/timeline';
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
</script>

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
        <details class:active={entry.active} class="work-group">
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
                <details class="tool-item" open={tool.status === 'in_progress'}>
                  <summary>
                    <span class="tool-icon"><IconTool size={14} stroke={1.8} /></span>
                    <span class="tool-title"><strong>{tool.title}</strong><small>{toolStatus(tool.status)}</small></span>
                  </summary>
                  {#if tool.detail}<pre>{tool.detail}</pre>{/if}
                  {#if tool.locations.length > 0}
                    <div class="tool-locations">{#each tool.locations as location}<span>{location}</span>{/each}</div>
                  {/if}
                </details>
              {:else}
                <div class:thought={workEntry.message.role === 'thought'} class="work-message">
                  <MarkdownMessage id={workEntry.message.id} source={workEntry.message.text.trim()} streaming={entry.active} />
                </div>
              {/if}
            {/each}
          </div>
        </details>
      {:else}
        {@const message = entry.message}
        {#if message.role === 'notice' || message.role === 'error'}
          <div class:error={message.role === 'error'} class="notice-message">
            {#if message.role === 'error'}<IconAlertTriangle size={15} stroke={1.8} />{/if}
            <span>{message.text}</span>
          </div>
        {:else}
          {@const streaming = isStreamingMessage(thread, message)}
          <article class:from-user={message.role === 'user'} class:streaming={streaming} class="message">
            <header>{message.role === 'user' ? 'You' : threadHarness(thread)}</header>
            <div class="message-body">
              <MarkdownMessage id={message.id} source={message.text} {streaming} />
              {#if message.images && message.images.length > 0}
                <div class="message-images" aria-label="Attached images">
                  {#each message.images as image, index (`${message.id}-image-${index}`)}
                    <img src={imageUrl(image)} alt={image.name} title={image.name} />
                  {/each}
                </div>
              {/if}
            </div>
          </article>
        {/if}
      {/if}
    {/each}
    {#if threadStatus(thread) === 'running' && !entries.some((entry) => entry.type === 'work' && entry.active)}
      <div class="running-indicator"><span class="activity-spark"></span><span>Working…</span></div>
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
