<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { IconAlertTriangle, IconChevronDown, IconChevronRight, IconTool } from '@tabler/icons-svelte';
  import { Collapsible } from 'bits-ui';

  import ContextMenu, { type ContextMenuItem } from './ContextMenu.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import MarkdownMessage from './markdown/MarkdownMessage.svelte';
  import type {
    ComposerReference,
    HarnessProfile,
    MessageImage,
    ThreadState,
    TimelineMessage,
    ToolActivity,
  } from '../types';
  import type { TimelineDisplayEntry } from '../types/timeline';
  import type { SendShortcut } from '../utils/app-settings';
  import { copyImage, copyText, saveImage } from '../utils/clipboard';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';
  import { composerEnterAction } from '../utils/prompt-content';
  import { formatElapsedDuration, isStreamingMessage } from '../utils/timeline';
  import { threadHarness, threadStatus } from '../utils/threads';

  interface Props {
    thread: ThreadState;
    entries: TimelineDisplayEntry[];
    profiles: HarnessProfile[];
    reducedMotion: boolean;
    sendShortcut: SendShortcut;
    openFile: (path: string) => void;
    resendPrompt: (text: string, references: ComposerReference[]) => boolean;
  }

  const { thread, entries, profiles, reducedMotion, sendShortcut, openFile, resendPrompt }: Props =
    $props();
  let transcriptElement = $state<HTMLElement>();
  let canScrollUp = $state(false);
  let canScrollDown = $state(false);
  let pinnedToBottom = true;
  let renderedThreadId: string | undefined;
  let renderedUserMessageId: string | undefined;
  let animateEntries = $state(false);
  let imagePreview = $state<{ src: string; name: string }>();
  let editingMessageId = $state<string>();
  let editDraft = $state('');
  let editReferences = $state.raw<ComposerReference[]>([]);
  let editEditor = $state<HTMLTextAreaElement>();
  let workDurationNow = $state(Date.now());
  let workGroupOpen = $state<Record<string, boolean>>({});
  let toolOpen = $state<Record<string, boolean>>({});
  // ponytail: DOM nodes kept out of deep reactivity; read at action time, not render time
  let messageBodies = $state.raw<Record<string, HTMLElement | undefined>>({});
  const entryMotion = $derived(animateEntries && !reducedMotion);
  const latestUserMessage = $derived(
    thread.messages.filter((message) => message.role === 'user').at(-1),
  );
  const transcriptStatus = $derived(threadStatus(thread));
  const awaitingAnswer = $derived(
    transcriptStatus === 'connecting' || transcriptStatus === 'running',
  );
  // Only the newest prompt is editable: turns are append-only, so an edit resends rather
  // than rewinds, and appending only reads correctly at the end of the thread.
  const editablePromptId = $derived(
    latestUserMessage
      && (transcriptStatus === 'disconnected' || transcriptStatus === 'ready')
      && !(latestUserMessage.images?.length)
      ? latestUserMessage.id
      : undefined,
  );
  const activeWorkStartedAt = $derived.by(() => {
    for (const entry of entries) {
      if (entry.type === 'work' && entry.active) return entry.startedAt;
    }
    return undefined;
  });
  const timeFormatter = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' });

  onMount(() => {
    const frame = requestAnimationFrame(() => { animateEntries = true; });
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    if (activeWorkStartedAt === undefined) return;
    workDurationNow = Date.now();
    const interval = window.setInterval(() => { workDurationNow = Date.now(); }, 1_000);
    return () => window.clearInterval(interval);
  });

  $effect(() => {
    const updatedAt = thread.updatedAt;
    const threadId = thread.id;
    const transcript = transcriptElement;
    const userMessageId = latestUserMessage?.id;
    if (updatedAt >= 0 && transcript) {
      const threadChanged = renderedThreadId !== threadId;
      const userMessageChanged = renderedUserMessageId !== userMessageId;
      let latestActivityAt = 0;
      for (const message of thread.messages) latestActivityAt = Math.max(latestActivityAt, message.createdAt);
      for (const tool of thread.tools) latestActivityAt = Math.max(latestActivityAt, tool.createdAt);
      const promptAnchorId = userMessageChanged
        && userMessageId
        && (awaitingAnswer || latestUserMessage.createdAt === latestActivityAt)
        ? userMessageId
        : undefined;
      if (promptAnchorId) pinnedToBottom = false;
      const shouldFollow = !promptAnchorId && (threadChanged || pinnedToBottom);
      renderedThreadId = threadId;
      renderedUserMessageId = userMessageId;
      void tick().then(() => {
        if (!transcriptElement) return;
        if (promptAnchorId) anchorPromptNearTop(promptAnchorId);
        else if (shouldFollow) scrollToBottom('auto');
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

  function anchorPromptNearTop(messageId: string) {
    if (!transcriptElement) return;
    const prompt = messageBodies[messageId];
    if (!prompt) return;
    const top = prompt.getBoundingClientRect().top
      - transcriptElement.getBoundingClientRect().top
      + transcriptElement.scrollTop
      - 13;
    pinnedToBottom = false;
    transcriptElement.scrollTo({ top: Math.max(0, top), behavior: 'auto' });
    updateScrollState();
    pinnedToBottom = false;
  }

  // Drop out of editing whenever the prompt stops being the editable one: thread switch,
  // a new turn starting, or the agent replying.
  $effect(() => {
    if (editingMessageId && editingMessageId !== editablePromptId) editingMessageId = undefined;
  });

  // Rendered prompts may contain their own links, so activation ignores clicks and Enter
  // presses that belong to a nested control.
  function isNestedControl(event: Event) {
    const target = event.composedPath?.().find((node) => node instanceof Element) ?? event.target;
    return target instanceof Element && target.closest('a, button') !== null;
  }

  function startEditingPrompt(message: TimelineMessage, event: Event) {
    if (isNestedControl(event)) return;
    if (window.getSelection()?.isCollapsed === false) return;
    editingMessageId = message.id;
    editDraft = message.text;
    editReferences = (message.content ?? []).flatMap((part) =>
      part.type === 'reference' ? [part.reference] : [],
    );
    void tick().then(() => {
      if (!editEditor) return;
      growEditor(editEditor);
      editEditor.focus();
      editEditor.setSelectionRange(editDraft.length, editDraft.length);
    });
  }

  function growEditor(editor: HTMLTextAreaElement) {
    editor.style.height = 'auto';
    editor.style.height = `${editor.scrollHeight}px`;
  }

  function submitEditedPrompt() {
    const text = editDraft.trim();
    if (!text) return;
    if (resendPrompt(text, editReferences)) editingMessageId = undefined;
  }

  function handleEditKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      editingMessageId = undefined;
      return;
    }
    if (composerEnterAction(sendShortcut, event) === 'send') {
      event.preventDefault();
      submitEditedPrompt();
    }
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

{#snippet promptContent(message: TimelineMessage, streaming: boolean)}
  {#if message.content}
    <div class="contents">
      {#each message.content as part}
        {#if part.type === 'text'}
          <span>{part.text}</span>
        {:else}
          {@const reference = part.reference}
          <span class="inline-flex h-[1.45em] max-w-[180px] mx-px items-center gap-1 rounded bg-panel-active px-1.5 align-[-.18em] text-[13px] leading-none whitespace-nowrap text-ink-soft [&>img]:size-3 [&>img]:shrink-0 [&>img]:opacity-65 [&>img]:[filter:var(--provider-filter)] [&>span:last-child]:min-w-0 [&>span:last-child]:truncate" title={reference.relativePath}>
            {#if reference.kind === 'skill'}
              <span class="font-semibold text-muted">$</span>
            {:else}
              {@const icon = reference.kind === 'folder' ? materialFolderIcon(reference.name, false) : materialFileIcon(reference.name)}
              {#if icon}<img class="size-3 shrink-0 opacity-65 [filter:var(--provider-filter)]" src={icon} alt="" />{/if}
            {/if}
            <span>{reference.name}</span>
          </span>
        {/if}
      {/each}
    </div>
  {:else}
    <MarkdownMessage id={message.id} source={message.text} {streaming} />
  {/if}
{/snippet}

<div class="relative flex min-h-0 min-w-0 flex-1 flex-col" in:fade|global={{ duration: reducedMotion ? 0 : 150 }}>
  <section
    class="min-h-0 min-w-0 flex-1 overflow-y-auto scroll-smooth"
    bind:this={transcriptElement}
    aria-live="polite"
    onscroll={updateScrollState}
  >
    <div
      class="mx-auto flex w-[min(var(--content-width,720px),100%)] flex-col gap-4 px-4 py-6"
    >
    {#each entries as entry (entry.type === 'message' ? `message-${entry.message.id}` : entry.id)}
      {#if entry.type === 'work'}
        {@const workGroupExpanded = entry.active || workGroupOpen[entry.id] === true}
        <div
          class="rounded-lg border border-line bg-panel p-2"
          in:fly={{ y: entryMotion ? 4 : 0, duration: entryMotion ? 180 : 0 }}
        >
          <div class="flex min-h-6 items-center">
            {#if entry.active}
              <div class="text-xs text-muted">
                Working for {formatElapsedDuration(workDurationNow - entry.startedAt)}
              </div>
            {:else}
              <button
                type="button"
                class="flex w-full items-center justify-between text-left text-xs text-muted hover:text-ink-soft"
                aria-expanded={workGroupExpanded}
                onclick={() => { workGroupOpen[entry.id] = !workGroupExpanded; }}
              >
                <span>Worked for {formatElapsedDuration(entry.durationMs ?? 0)}</span>
                <span class="shrink-0">
                  {#if workGroupExpanded}
                    <IconChevronDown size={14} stroke={1.55} />
                  {:else}
                    <IconChevronRight size={14} stroke={1.55} />
                  {/if}
                </span>
              </button>
            {/if}
          </div>
          {#if workGroupExpanded}
            <div class="mt-2 space-y-2">
            {#each entry.entries as workEntry (workEntry.type === 'tool' ? `tool-${workEntry.tool.id}` : `message-${workEntry.message.id}`)}
              {#if workEntry.type === 'tool'}
                {@const tool = workEntry.tool}
                <ContextMenu items={toolMenuItems(tool)}>
                  {#snippet children({ props })}
                    <Collapsible.Root
                      {...props}
                      class="rounded-md border border-line bg-recessed"
                      bind:open={
                        () => toolOpen[tool.id] ?? tool.status === 'in_progress',
                        (value) => { toolOpen[tool.id] = value; }
                      }
                    >
                      <div in:fly={{ y: entryMotion ? 3 : 0, duration: entryMotion ? 160 : 0 }}>
                        <Collapsible.Trigger class="flex w-full items-center gap-2 p-2 text-left">
                          <span class="grid size-6 shrink-0 place-items-center rounded bg-panel text-muted"><IconTool size={16} stroke={1.55} /></span>
                          <span class="grid min-w-0 gap-0.5"><strong class="truncate text-xs font-semibold text-ink-soft">{tool.title}</strong><small class="text-[11px] text-muted">{toolStatus(tool.status)}</small></span>
                        </Collapsible.Trigger>
                        <Collapsible.Content>
                          {#if tool.detail}<pre class="m-2 overflow-auto rounded bg-shell p-2 text-[11px] leading-5 whitespace-pre-wrap text-ink-soft">{tool.detail}</pre>{/if}
                          {#if tool.locations.length > 0}
                            <div class="flex flex-wrap gap-1 p-2 pt-0">
                              {#each tool.locations as location (location)}
                                <ContextMenu items={[{ label: 'Copy path', action: () => copyText(location) }]}>
                                  {#snippet children({ props: locationProps })}
                                    <span class="rounded bg-panel px-1.5 py-0.5 font-mono text-[10px] text-muted" role="presentation" {...locationProps}>{location}</span>
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
                      class="rounded px-2 text-sm leading-6 text-ink-soft [&_p]:my-2 [&_pre]:overflow-auto"
                      in:fly={{ y: entryMotion ? 3 : 0, duration: entryMotion ? 160 : 0 }}
                    >
                      <MarkdownMessage
                        id={workEntry.message.id}
                        source={workEntry.message.text.trim()}
                        streaming={entry.active}
                        fileLinks={{ projectRoot: thread.cwd, open: openFile }}
                      />
                    </div>
                  {/snippet}
                </ContextMenu>
              {/if}
            {/each}
            </div>
          {/if}
        </div>
      {:else}
        {@const message = entry.message}
        {#if message.role === 'notice' || message.role === 'error'}
          <div
            class={`flex items-center gap-2 rounded-md border px-2.5 py-2 text-xs ${message.role === 'error' ? 'border-danger/30 bg-danger/10 text-danger' : 'border-line bg-panel text-muted'}`}
            in:fly={{ y: entryMotion ? 4 : 0, duration: entryMotion ? 170 : 0 }}
          >
            {#if message.role === 'error'}<IconAlertTriangle size={15} stroke={1.55} />{/if}
            <span>{message.text}</span>
          </div>
        {:else}
          {@const streaming = isStreamingMessage(thread, message)}
          <ContextMenu items={messageMenuItems(message)}>
            {#snippet children({ props })}
              <article
                {...props}
                bind:this={messageBodies[message.id]}
                class={`rounded-lg px-3 py-2.5 text-sm leading-6 ${message.role === 'user' ? 'ml-auto w-fit max-w-[85%] bg-panel' : 'text-ink'}`}
                in:fly={{ y: entryMotion ? (message.role === 'user' ? 10 : 4) : 0, duration: entryMotion ? 180 : 0 }}
              >
            <header class="mb-1 hidden items-center justify-between text-[11px] text-faint">
              <span>{message.role === 'user' ? 'You' : threadHarness(thread, profiles)}</span>
              <time datetime={new Date(message.createdAt).toISOString()} title={new Date(message.createdAt).toLocaleString()}>
                {timeFormatter.format(message.createdAt)}
              </time>
            </header>
            <div class="message-body [&_a]:text-accent [&_a]:underline [&_blockquote]:my-3 [&_blockquote]:border-l-2 [&_blockquote]:border-line-strong [&_blockquote]:pl-3 [&_code]:font-mono [&_h1]:my-4 [&_h1]:text-xl [&_h1]:font-semibold [&_h2]:my-3 [&_h2]:text-lg [&_h2]:font-semibold [&_h3]:my-3 [&_h3]:font-semibold [&_li]:my-1 [&_ol]:my-3 [&_ol]:list-decimal [&_ol]:pl-6 [&_p]:my-3 [&_pre]:my-3 [&_pre]:overflow-auto [&_pre]:rounded-lg [&_pre]:bg-recessed [&_pre]:p-3 [&_table]:my-3 [&_table]:w-full [&_ul]:my-3 [&_ul]:list-disc [&_ul]:pl-6">
              {#if message.role === 'user'}
                {#if editingMessageId === message.id}
                  <div class="prompt-edit">
                    <textarea
                      bind:this={editEditor}
                      bind:value={editDraft}
                    class="block w-full resize-none rounded-md border border-line bg-recessed p-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--focus-ring)]"
                      aria-label="Edit prompt"
                      rows="1"
                      oninput={(event) => growEditor(event.currentTarget)}
                      onkeydown={handleEditKeydown}
                    ></textarea>
                    <div class="mt-2 flex justify-end gap-1.5 [&_button]:rounded-md [&_button]:border [&_button]:border-line [&_button]:px-2 [&_button]:py-1 [&_button]:text-xs [&_button]:text-muted [&_button:hover]:bg-panel-hover">
                      <button type="button" onclick={() => { editingMessageId = undefined; }}>Cancel</button>
                      <button
                        type="button"
                        class="!border-line-strong !bg-accent !text-accent-contrast disabled:opacity-40"
                        disabled={editDraft.trim().length === 0}
                        onclick={submitEditedPrompt}
                      >Send</button>
                    </div>
                  </div>
                {:else if editablePromptId === message.id}
                  <div
                    role="button"
                    tabindex="0"
                    class="cursor-text"
                    aria-label="Edit prompt"
                    onclick={(event) => startEditingPrompt(message, event)}
                    onkeydown={(event) => {
                      if (event.key !== 'Enter' && event.key !== ' ') return;
                      if (isNestedControl(event)) return;
                      event.preventDefault();
                      startEditingPrompt(message, event);
                    }}
                  >{@render promptContent(message, streaming)}</div>
                {:else}
                  {@render promptContent(message, streaming)}
                {/if}
              {:else}
                <MarkdownMessage
                  id={message.id}
                  source={message.text}
                  {streaming}
                  fileLinks={message.role === 'agent'
                    ? { projectRoot: thread.cwd, open: openFile }
                    : undefined}
                />
              {/if}
              {#if message.images && message.images.length > 0}
                <div class="mt-2 flex flex-wrap gap-2" aria-label="Attached images">
                  {#each message.images as image, index (`${message.id}-image-${index}`)}
                    {@const src = imageUrl(image)}
                    <ContextMenu items={imageMenuItems(image, src)}>
                      {#snippet children({ props: imageProps })}
                        <button
                          {...imageProps}
                          type="button"
                          class="size-20 overflow-hidden rounded-md border border-line"
                          aria-label={`Preview ${image.name}`}
                          title={image.name}
                          onclick={() => { imagePreview = { src, name: image.name }; }}
                        ><img class="size-full object-cover" {src} alt="" /></button>
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
      <div class="flex items-center gap-2 text-xs text-muted" in:fade={{ duration: entryMotion ? 140 : 0 }}><span class="size-1.5 rounded-full bg-success"></span><span>Working…</span></div>
    {/if}
    </div>
  </section>

  {#if canScrollDown}
    <button
      class="absolute right-4 bottom-4 flex items-center gap-1 rounded-full border border-line-strong bg-floating px-2.5 py-1.5 text-[11px] text-ink-soft shadow-overlay hover:bg-panel-hover"
      type="button"
      aria-label="Scroll to latest message"
      onclick={() => scrollToBottom()}
      transition:fade={{ duration: reducedMotion ? 0 : 120 }}
    >
      <IconChevronDown size={13} stroke={1.55} />
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
