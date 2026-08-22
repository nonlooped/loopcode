<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { IconAlertTriangle, IconChevronDown, IconChevronRight, IconTool } from '@tabler/icons-svelte';
  import { Collapsible } from 'bits-ui';

  import ContextMenu, { type ContextMenuItem } from './ContextMenu.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import MarkdownMessage from './markdown/MarkdownMessage.svelte';
  import type { MessageImage, ThreadState, TimelineMessage, ToolActivity } from '../types';
  import type { TimelineDisplayEntry } from '../types/timeline';
  import { copyImage, copyText, saveImage } from '../utils/clipboard';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';
  import { isStreamingMessage, shouldFollowTranscript, workGroupMeta } from '../utils/timeline';
  import { threadHarness, threadStatus } from '../utils/threads';

  interface Props {
    thread: ThreadState;
    entries: TimelineDisplayEntry[];
    reducedMotion: boolean;
    autoFollowOutput: boolean;
  }

  const { thread, entries, reducedMotion, autoFollowOutput }: Props = $props();
  let transcriptElement = $state<HTMLElement>();
  let canScrollUp = $state(false);
  let canScrollDown = $state(false);
  let pinnedToBottom = true;
  let renderedThreadId: string | undefined;
  let renderedUserMessageId: string | undefined;
  let animateEntries = $state(false);
  let imagePreview = $state<{ src: string; name: string }>();
  let toolOpen = $state<Record<string, boolean>>({});
  // ponytail: DOM nodes kept out of deep reactivity; read at action time, not render time
  let messageBodies = $state.raw<Record<string, HTMLElement | undefined>>({});
  const entryMotion = $derived(animateEntries && !reducedMotion);
  const timeFormatter = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' });

  onMount(() => {
    const frame = requestAnimationFrame(() => { animateEntries = true; });
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    const updatedAt = thread.updatedAt;
    const threadId = thread.id;
    const transcript = transcriptElement;
    const userMessageId = thread.messages.filter((message) => message.role === 'user').at(-1)?.id;
    if (updatedAt >= 0 && transcript) {
      const threadChanged = renderedThreadId !== threadId;
      const shouldFollow = shouldFollowTranscript(
        threadChanged,
        renderedUserMessageId !== userMessageId,
        autoFollowOutput,
        pinnedToBottom,
      );
      renderedThreadId = threadId;
      renderedUserMessageId = userMessageId;
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

  function messageMenuItems(message: TimelineMessage): ContextMenuItem[] {
    return [
      {
        label: 'Copy message',
        action: () => copyText(messageBodies[message.id]?.innerText.trim() || message.text),
      },
      { label: 'Copy as Markdown', action: () => copyText(message.text) },
    ];
  }

  function toolMenuItems(tool: ToolActivity): ContextMenuItem[] {
    const expanded = toolOpen[tool.id] ?? tool.status === 'in_progress';
    return [
      { label: expanded ? 'Collapse' : 'Expand', action: () => { toolOpen[tool.id] = !expanded; } },
      { label: 'Copy details', action: () => copyText(tool.detail ?? ''), disabled: !tool.detail },
      {
        label: tool.locations.length === 1 ? 'Copy location' : 'Copy locations',
        action: () => copyText(tool.locations.join('\n')),
        disabled: tool.locations.length === 0,
      },
    ];
  }

  function imageMenuItems(image: MessageImage, src: string): ContextMenuItem[] {
    return [
      { label: 'Open preview', action: () => { imagePreview = { src, name: image.name }; } },
      { label: 'Copy image', action: () => copyImage(src) },
      { label: 'Save image', action: () => saveImage(src, image.name) },
    ];
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
        <div
          class:active={entry.active}
          class="work-group"
          in:fly={{ y: entryMotion ? 4 : 0, duration: entryMotion ? 180 : 0 }}
        >
          <Collapsible.Root class="work-group-root">
          <Collapsible.Trigger class="work-group-trigger">
            <span class="work-chevron"><IconChevronRight size={14} stroke={1.8} /></span>
            <span class="activity-spark"></span>
            <strong>{entry.active ? 'Working…' : 'Working'}</strong>
            <small>{workGroupMeta(entry.entries)}</small>
          </Collapsible.Trigger>
          <Collapsible.Content class="work-group-content">
            {#each entry.entries as workEntry (workEntry.type === 'tool' ? `tool-${workEntry.tool.id}` : `message-${workEntry.message.id}`)}
              {#if workEntry.type === 'tool'}
                {@const tool = workEntry.tool}
                <ContextMenu items={toolMenuItems(tool)}>
                  {#snippet children({ props })}
                    <Collapsible.Root
                      {...props}
                      class="tool-item"
                      bind:open={
                        () => toolOpen[tool.id] ?? tool.status === 'in_progress',
                        (value) => { toolOpen[tool.id] = value; }
                      }
                    >
                      <div in:fly={{ y: entryMotion ? 3 : 0, duration: entryMotion ? 160 : 0 }}>
                        <Collapsible.Trigger class="tool-item-trigger">
                          <span class="tool-icon"><IconTool size={14} stroke={1.8} /></span>
                          <span class="tool-title"><strong>{tool.title}</strong><small>{toolStatus(tool.status)}</small></span>
                        </Collapsible.Trigger>
                        <Collapsible.Content>
                          {#if tool.detail}<pre>{tool.detail}</pre>{/if}
                          {#if tool.locations.length > 0}
                            <div class="tool-locations">
                              {#each tool.locations as location (location)}
                                <ContextMenu items={[{ label: 'Copy path', action: () => copyText(location) }]}>
                                  {#snippet children({ props: locationProps })}
                                    <span role="presentation" {...locationProps}>{location}</span>
                                  {/snippet}
                                </ContextMenu>
                              {/each}
                            </div>
                          {/if}
                        </Collapsible.Content>
                      </div>
                    </Collapsible.Root>
                  {/snippet}
                </ContextMenu>
              {:else}
                <ContextMenu items={messageMenuItems(workEntry.message)}>
                  {#snippet children({ props })}
                    <div
                      {...props}
                      bind:this={messageBodies[workEntry.message.id]}
                      role="presentation"
                      class:thought={workEntry.message.role === 'thought'}
                      class="work-message"
                      in:fly={{ y: entryMotion ? 3 : 0, duration: entryMotion ? 160 : 0 }}
                    >
                      <MarkdownMessage id={workEntry.message.id} source={workEntry.message.text.trim()} streaming={entry.active} />
                    </div>
                  {/snippet}
                </ContextMenu>
              {/if}
            {/each}
          </Collapsible.Content>
          </Collapsible.Root>
        </div>
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
          <ContextMenu items={messageMenuItems(message)}>
            {#snippet children({ props })}
              <article
                {...props}
                bind:this={messageBodies[message.id]}
                class:from-user={message.role === 'user'}
                class:streaming={streaming}
                class="message"
                in:fly={{ y: entryMotion ? (message.role === 'user' ? 10 : 4) : 0, duration: entryMotion ? 180 : 0 }}
              >
            <header>
              <span>{message.role === 'user' ? 'You' : threadHarness(thread)}</span>
              <time datetime={new Date(message.createdAt).toISOString()} title={new Date(message.createdAt).toLocaleString()}>
                {timeFormatter.format(message.createdAt)}
              </time>
            </header>
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
                    <ContextMenu items={imageMenuItems(image, src)}>
                      {#snippet children({ props: imageProps })}
                        <button
                          {...imageProps}
                          type="button"
                          class="message-image-button"
                          aria-label={`Preview ${image.name}`}
                          title={image.name}
                          onclick={() => { imagePreview = { src, name: image.name }; }}
                        ><img {src} alt="" /></button>
                      {/snippet}
                    </ContextMenu>
                  {/each}
                </div>
              {/if}
            </div>
          </article>
            {/snippet}
          </ContextMenu>
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

{#if imagePreview}
  <ImagePreview
    src={imagePreview.src}
    name={imagePreview.name}
    close={() => { imagePreview = undefined; }}
  />
{/if}
