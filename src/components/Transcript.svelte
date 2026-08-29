<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    IconAlertTriangle,
    IconArrowRight,
    IconBulb,
    IconChevronDown,
    IconChevronRight,
    IconFileText,
    IconListCheck,
    IconPencil,
    IconSearch,
    IconTerminal2,
    IconTool,
    IconTrash,
    IconWorld,
  } from '@tabler/icons-svelte';
  import { Collapsible } from 'bits-ui';

  import ContextMenu, { type ContextMenuItem } from './ContextMenu.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import ToolContent from './ToolContent.svelte';
  import MarkdownMessage from './markdown/MarkdownMessage.svelte';
  import MotionBreathe from './motion/MotionBreathe.svelte';
  import MotionEnter from './motion/MotionEnter.svelte';
  import MotionFade from './motion/MotionFade.svelte';
  import type {
    ComposerReference,
    HarnessProfile,
    MessageImage,
    ThreadState,
    TimelineMessage,
    ToolActivity,
    SessionFailureAction,
  } from '../types';
  import type { TimelineDisplayEntry } from '../types/timeline';
  import type { SendShortcut } from '../utils/app-settings';
  import { copyImage, copyText, saveImage } from '../utils/clipboard';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';
  import { composerEnterAction } from '../utils/prompt-content';
  import { formatElapsedDuration, isStreamingMessage, streamingMessageId } from '../utils/timeline';
  import { threadHarness, threadStatus } from '../utils/threads';

  interface Props {
    thread: ThreadState;
    entries: TimelineDisplayEntry[];
    profiles: HarnessProfile[];
    reducedMotion: boolean;
    sendShortcut: SendShortcut;
    openFile: (path: string) => void;
    resendPrompt: (text: string, references: ComposerReference[]) => boolean;
    recoverFailure: (action: SessionFailureAction, messageId?: string) => void;
  }

  const { thread, entries, profiles, reducedMotion, sendShortcut, openFile, resendPrompt, recoverFailure }: Props =
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
  let expandedPrompts = $state<Record<string, boolean>>({});
  let overflowingPrompts = $state<Record<string, boolean>>({});
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
    const promptBody = messageBodies[messageId];
    if (!promptBody) return;
    const prompt = promptBody.parentElement ?? promptBody;
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

  function measurePrompt(node: HTMLElement, messageId: string) {
    const update = () => {
      const overflowing = node.scrollHeight > 174;
      if (overflowingPrompts[messageId] === overflowing) return;
      overflowingPrompts[messageId] = overflowing;
      void tick().then(updateScrollState);
    };
    const observer = new ResizeObserver(update);
    observer.observe(node);
    update();
    return { destroy: () => observer.disconnect() };
  }

  function togglePrompt(messageId: string) {
    expandedPrompts[messageId] = !expandedPrompts[messageId];
    void tick().then(updateScrollState);
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

  const toolIcons: Record<string, typeof IconTool> = {
    delete: IconTrash,
    edit: IconPencil,
    execute: IconTerminal2,
    fetch: IconWorld,
    move: IconArrowRight,
    read: IconFileText,
    search: IconSearch,
    think: IconBulb,
  };

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
  const entryFly = (y: number) => ({
    y: entryMotion ? y : 0,
    duration: entryMotion ? (y >= 4 ? 180 : y >= 3 ? 160 : 170) : 0,
  });

  const messageReferenceClass = (skill: boolean) =>
    `inline-flex max-w-[180px] h-[1.45em] mx-px px-[5px] items-center gap-1 rounded-[5px] bg-panel-active text-text-soft text-[13px] leading-none whitespace-nowrap ${
      skill ? 'align-baseline' : 'align-[-0.18em]'
    }`;

  const workMessageTypography =
    'min-w-0 overflow-wrap-anywhere text-[13px] leading-[1.55] text-text-soft [&_p]:mb-[0.45em] [&_ul]:mb-[0.45em] [&_ol]:mb-[0.45em] [&_blockquote]:mb-[0.45em] [&_pre]:mb-[0.45em] [&_table]:mb-[0.45em] [&_h1]:mb-[0.45em] [&_h2]:mb-[0.45em] [&_h3]:mb-[0.45em] [&_h4]:mb-[0.45em] [&_h5]:mb-[0.45em] [&_h6]:mb-[0.45em] [&_h1]:text-base [&_h2]:text-base [&_h3]:text-base [&_h4]:text-base [&_h5]:text-base [&_h6]:text-base [&_h1]:font-[650] [&_h2]:font-[650] [&_h3]:font-[650] [&_h4]:font-[650] [&_h5]:font-[650] [&_h6]:font-[650] [&_ul]:pl-[1.45em] [&_ol]:pl-[1.45em] [&>:last-child]:mb-0 [&_p:last-child]:mb-0 [&_ul:last-child]:mb-0 [&_ol:last-child]:mb-0 [&_blockquote:last-child]:mb-0 [&_pre:last-child]:mb-0 [&_table:last-child]:mb-0 [&_h1:last-child]:mb-0 [&_h2:last-child]:mb-0 [&_h3:last-child]:mb-0 [&_h4:last-child]:mb-0 [&_h5:last-child]:mb-0 [&_h6:last-child]:mb-0';

  const toolItemClass =
    'm-0 overflow-hidden rounded-md border border-transparent bg-transparent data-[state=open]:border-line data-[state=open]:bg-panel';
  const toolTriggerClass =
    'flex w-full min-h-[30px] items-center gap-1.5 border-0 bg-transparent p-px py-px pr-0.5 pl-0.5 text-inherit list-none select-none hover:bg-panel-hover';
  const collapsibleContentClass = 'overflow-hidden';
</script>

{#snippet promptContent(message: TimelineMessage, streaming: boolean)}
  {#if message.content}
    <div class="whitespace-pre-wrap">
      {#each message.content as part}
        {#if part.type === 'text'}
          <span>{part.text}</span>
        {:else}
          {@const reference = part.reference}
          <span class={messageReferenceClass(reference.kind === 'skill')} title={reference.relativePath}>
            {#if reference.kind === 'skill'}
              <span class="font-semibold text-muted">$</span>
            {:else}
              {@const icon = reference.kind === 'folder' ? materialFolderIcon(reference.name, false) : materialFileIcon(reference.name)}
              {#if icon}<img class="size-3 shrink-0 opacity-[0.64] [filter:var(--provider-filter)]" src={icon} alt="" />{/if}
            {/if}
            <span class="min-w-0 overflow-hidden text-ellipsis">{reference.name}</span>
          </span>
        {/if}
      {/each}
    </div>
  {:else}
    <MarkdownMessage id={message.id} source={message.text} {streaming} />
  {/if}
{/snippet}

{#snippet failureActions(message: TimelineMessage)}
  {#if message.failure?.actions.length}
    <div class="mt-2 flex flex-wrap gap-1.5">
      {#each message.failure.actions as action (action)}
        <button type="button" class="rounded-md border border-line px-2 py-1 text-[11px] font-medium text-text-soft hover:bg-panel-hover" onclick={() => recoverFailure(action, message.role === 'user' ? message.id : undefined)}>{action === 'new_session' ? 'New session' : action[0].toUpperCase() + action.slice(1)}</button>
      {/each}
    </div>
  {/if}
{/snippet}

<div class="relative min-h-0 flex-1 [container-type:size]">
  <MotionEnter class="h-full min-h-0" duration={reducedMotion ? 0 : 150}>
  <section
    bind:this={transcriptElement}
    class="h-full min-h-0 overflow-y-auto bg-transparent select-text [scrollbar-gutter:stable]"
    class:can-scroll-up={canScrollUp}
    class:can-scroll-down={canScrollDown}
    class:[mask-image:linear-gradient(to_bottom,transparent_0,#000_22px,#000_100%)]={canScrollUp && !canScrollDown}
    class:[mask-image:linear-gradient(to_bottom,#000_0,#000_calc(100%-58px),transparent_100%)]={canScrollDown && !canScrollUp}
    class:[mask-image:linear-gradient(to_bottom,transparent_0,#000_22px,#000_calc(100%-58px),transparent_100%)]={canScrollUp && canScrollDown}
    aria-live="polite"
    onscroll={updateScrollState}
  >
    <div
      class="mx-auto min-h-full w-[min(var(--content-width,720px),calc(100%-56px))] py-[13px] pb-8 empty:p-0 after:content-['']"
      class:after:block={awaitingAnswer}
      class:after:min-h-[160px]={awaitingAnswer}
      class:after:h-[calc(100cqh-64px)]={awaitingAnswer}
    >
    {#each entries as entry (entry.type === 'message' ? `message-${entry.message.id}` : entry.id)}
      {#if entry.type === 'work'}
        {@const workGroupExpanded = entry.active || workGroupOpen[entry.id] === true}
        {@const fly = entryFly(4)}
        <MotionEnter y={fly.y} duration={fly.duration}>
        <div class="mb-3.5 text-muted [.compact-transcript_&]:mb-[9px]">
          <div class="border-b border-line px-0 py-1 pb-2">
            {#if entry.active}
              <div class="flex w-fit min-h-6 items-center gap-1 rounded-md px-1 text-[13px] tabular-nums leading-[1.55] text-muted">
                Working for {formatElapsedDuration(workDurationNow - entry.startedAt)}
              </div>
            {:else}
              <button
                type="button"
                class="flex w-fit min-h-6 cursor-pointer items-center gap-1 rounded-md border-0 bg-transparent px-1 text-[13px] tabular-nums leading-[1.55] text-muted select-none hover:text-text-soft"
                aria-expanded={workGroupExpanded}
                onclick={() => { workGroupOpen[entry.id] = !workGroupExpanded; }}
              >
                <span>Worked for {formatElapsedDuration(entry.durationMs ?? 0)}</span>
                <span class="grid size-3.5 place-items-center">
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
            <div class="my-1.5 mb-0.5 grid content-start gap-0.5 px-1">
            {#each entry.entries as workEntry (workEntry.type === 'tool' ? `tool-${workEntry.tool.id}` : `message-${workEntry.message.id}`)}
              {#if workEntry.type === 'tool'}
                {@const tool = workEntry.tool}
                {@const toolFly = entryFly(3)}
                <ContextMenu items={toolMenuItems(tool)}>
                  {#snippet children({ props })}
                    <Collapsible.Root
                      {...props}
                      class={toolItemClass}
                      bind:open={
                        () => toolOpen[tool.id] ?? tool.status === 'in_progress',
                        (value) => { toolOpen[tool.id] = value; }
                      }
                    >
                      <MotionEnter y={toolFly.y} duration={toolFly.duration}>
                        <Collapsible.Trigger class={toolTriggerClass}>
                          {@const ToolIcon = tool.plan ? IconListCheck : (toolIcons[tool.kind] ?? IconTool)}
                          <span class="grid size-6 shrink-0 place-items-center border-0 text-faint"><ToolIcon size={16} stroke={1.55} /></span>
                          <span class="flex min-w-0 items-baseline gap-2">
                            <strong class="min-w-0 flex-[1_1_auto] overflow-hidden text-ellipsis whitespace-nowrap text-[13px] font-medium text-text-soft">{tool.title}</strong>
                            <small class="shrink-0 text-[11px] capitalize text-faint">{tool.status.replaceAll('_', ' ')}</small>
                          </span>
                        </Collapsible.Trigger>
                        <Collapsible.Content class={collapsibleContentClass}>
                          <ToolContent {tool} {openFile} />
                          {#if tool.locations.length > 0}
                            <div class="mx-[5px] mb-[5px] ml-[27px] grid gap-1 font-mono text-[11px] text-muted">
                              {#each tool.locations as location (location)}
                                <ContextMenu items={[{ label: 'Copy path', action: () => copyText(location) }]}>
                                  {#snippet children({ props: locationProps })}
                                    <span role="presentation" class="overflow-hidden text-ellipsis whitespace-nowrap" {...locationProps}>{location}</span>
                                  {/snippet}
                                </ContextMenu>
                              {/each}
                            </div>
                          {/if}
                        </Collapsible.Content>
                      </MotionEnter>
                    </Collapsible.Root>
                  {/snippet}
                </ContextMenu>
              {:else}
                {@const workFly = entryFly(3)}
                <ContextMenu items={messageMenuItems(workEntry.message)}>
                  {#snippet children({ props })}
                    <MotionEnter y={workFly.y} duration={workFly.duration}>
                    <div
                      {...props}
                      bind:this={messageBodies[workEntry.message.id]}
                      role="presentation"
                      class="{workMessageTypography} m-0 p-[3px_4px]"
                      class:text-muted={workEntry.message.role === 'thought'}
                    >
                      <MarkdownMessage
                        id={workEntry.message.id}
                        source={workEntry.message.text.trim()}
                        streaming={workEntry.message.id === streamingMessageId(entry)}
                        fileLinks={{ projectRoot: thread.cwd, open: openFile }}
                      />
                    </div>
                    </MotionEnter>
                  {/snippet}
                </ContextMenu>
              {/if}
            {/each}
            </div>
          {/if}
        </div>
        </MotionEnter>
      {:else}
        {@const message = entry.message}
        {#if message.role === 'notice' || message.role === 'error'}
          {@const noticeFly = entryFly(4)}
          <MotionEnter y={noticeFly.y} duration={noticeFly.duration}>
          <div
            class="mb-6 flex items-start gap-2 text-xs leading-snug text-muted"
            class:text-danger={message.role === 'error'}
          >
            {#if message.role === 'error'}<IconAlertTriangle size={15} stroke={1.55} />{/if}
            <div class="min-w-0">
              <span>{message.text}</span>
              {#if message.failure?.details}<p class="mt-1 mb-0 whitespace-pre-wrap text-faint">{message.failure.details}</p>{/if}
              {@render failureActions(message)}
            </div>
          </div>
          </MotionEnter>
        {:else}
          {@const streaming = isStreamingMessage(thread, message)}
          {@const promptExpanded = expandedPrompts[message.id] === true}
          {@const promptOverflowing = overflowingPrompts[message.id] === true}
          {@const messageFly = entryFly(message.role === 'user' ? 10 : 4)}
          <ContextMenu items={messageMenuItems(message)}>
            {#snippet children({ props })}
              <MotionEnter y={messageFly.y} duration={messageFly.duration}>
              <article
                {...props}
                class="mb-[17px] [.compact-transcript_&]:mb-2.5"
                class:my-[18px]={message.role === 'user'}
                class:mb-[26px]={message.role === 'user'}
                class:rounded-xl={message.role === 'user'}
                class:border={message.role === 'user'}
                class:border-line={message.role === 'user'}
                class:bg-panel={message.role === 'user'}
                class:p-[11px_14px]={message.role === 'user'}
                class:[.compact-transcript_&]:mt-2={message.role === 'user'}
                class:[.compact-transcript_&]:mb-4={message.role === 'user'}
                class:[.compact-transcript_&]:p-2={message.role === 'user'}
                class:[.compact-transcript_&]:px-3={message.role === 'user'}
                class:hover:border-line-strong={message.role === 'user' && editablePromptId === message.id && editingMessageId !== message.id}
                class:hover:bg-panel-hover={message.role === 'user' && editablePromptId === message.id && editingMessageId !== message.id}
                class:border-line-strong={message.role === 'user' && editingMessageId === message.id}
                class:bg-panel-hover={message.role === 'user' && editingMessageId === message.id}
              >
            <div bind:this={messageBodies[message.id]}>
            <header class="hidden [.show-message-timestamps_&]:mb-[5px] [.show-message-timestamps_&]:flex [.show-message-timestamps_&]:items-center [.show-message-timestamps_&]:justify-between [.show-message-timestamps_&]:gap-2.5 [.show-message-timestamps_&]:text-[11px] [.show-message-timestamps_&]:font-medium [.show-message-timestamps_&]:text-faint [&_time]:font-normal [&_time]:tabular-nums">
              <span>{message.role === 'user' ? 'You' : threadHarness(thread, profiles)}</span>
              <time datetime={new Date(message.createdAt).toISOString()} title={new Date(message.createdAt).toLocaleString()}>
                {timeFormatter.format(message.createdAt)}
              </time>
            </header>
            <div class="break-anywhere text-sm leading-[1.55] text-text">
              {#if message.role === 'user'}
                <div
                  id={`prompt-${message.id}`}
                  use:measurePrompt={message.id}
                  class="relative"
                  class:max-h-[174px]={promptOverflowing && !promptExpanded && editingMessageId !== message.id}
                  class:overflow-hidden={promptOverflowing && !promptExpanded && editingMessageId !== message.id}
                  class:[mask-image:linear-gradient(to_bottom,#000_65%,transparent_100%)]={promptOverflowing && !promptExpanded && editingMessageId !== message.id}
                >
                  {#if editingMessageId === message.id}
                    <div>
                      <textarea
                        bind:this={editEditor}
                        bind:value={editDraft}
                        class="block w-full resize-none overflow-hidden border-0 bg-transparent p-0 font-[inherit] leading-[inherit] whitespace-pre-wrap outline-0 focus-visible:outline-0"
                        aria-label="Edit prompt"
                        rows="1"
                        oninput={(event) => growEditor(event.currentTarget)}
                        onkeydown={handleEditKeydown}
                      ></textarea>
                      <div class="mt-2.5 flex justify-end gap-[7px]">
                        <button type="button" class="h-[26px] rounded-[7px] border border-line bg-transparent px-[11px] text-xs font-medium text-text-soft hover:border-line-strong hover:bg-panel-hover hover:text-text" onclick={() => { editingMessageId = undefined; }}>Cancel</button>
                        <button
                          type="button"
                          class="h-[26px] rounded-[7px] border border-line-strong bg-accent px-[11px] text-xs font-medium text-accent-contrast hover:bg-accent-hover disabled:opacity-50"
                          disabled={editDraft.trim().length === 0}
                          onclick={submitEditedPrompt}
                        >Send</button>
                      </div>
                    </div>
                  {:else if editablePromptId === message.id}
                    <div
                      role="button"
                      tabindex="0"
                      class="w-full cursor-pointer [&_:where(a,button)]:cursor-pointer"
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
                </div>
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
                <div class="mt-[9px] flex max-w-[min(420px,65vw)] flex-wrap gap-[7px]" aria-label="Attached images">
                  {#each message.images as image, index (`${message.id}-image-${index}`)}
                    {@const src = imageUrl(image)}
                    <ContextMenu items={imageMenuItems(image, src)}>
                      {#snippet children({ props: imageProps })}
                        <button
                          {...imageProps}
                          type="button"
                          class="max-w-full overflow-hidden rounded-[10px] border-0 bg-transparent p-0"
                          aria-label={`Preview ${image.name}`}
                          title={image.name}
                          onclick={() => { imagePreview = { src, name: image.name }; }}
                        ><img class="block max-h-80 max-w-full rounded-[10px] border border-line bg-recessed object-contain" {src} alt="" /></button>
                      {/snippet}
                    </ContextMenu>
                  {/each}
                </div>
              {/if}
            </div>
            </div>
            {#if message.role === 'user' && promptOverflowing && editingMessageId !== message.id}
              <button
                type="button"
                class="mt-2 flex h-6 items-center gap-1 border-0 bg-transparent p-0 text-xs font-medium text-muted hover:text-text-soft"
                aria-expanded={promptExpanded}
                aria-controls={`prompt-${message.id}`}
                onclick={() => togglePrompt(message.id)}
              >
                <span>{promptExpanded ? 'Show less' : 'Show more'}</span>
                {#if promptExpanded}
                  <IconChevronDown class="rotate-180" size={14} stroke={1.55} />
                {:else}
                  <IconChevronDown size={14} stroke={1.55} />
                {/if}
              </button>
            {/if}
            {#if message.failure}
              <div class="mt-2 flex items-start gap-2 text-xs leading-snug text-danger">
                <IconAlertTriangle size={15} stroke={1.55} />
                <div class="min-w-0">
                  <span>{message.failure.title}</span>
                  {@render failureActions(message)}
                </div>
              </div>
            {/if}
          </article>
              </MotionEnter>
            {/snippet}
          </ContextMenu>
        {/if}
      {/if}
    {/each}
    {#if threadStatus(thread) === 'running' && !entries.some((entry) => entry.type === 'work' && entry.active)}
      <MotionEnter duration={entryMotion ? 140 : 0}>
        <div class="flex min-h-[26px] items-center gap-[7px] text-[11px] text-muted">
          <MotionBreathe min={0.48} max={0.9} class="relative inline-block size-2.5 shrink-0 opacity-[0.72] before:absolute before:size-0.5 before:rounded-[1px] before:bg-[color-mix(in_srgb,currentColor_82%,transparent)] before:shadow-[4px_0_currentColor,8px_0_currentColor,0_4px_currentColor,8px_4px_currentColor,0_8px_currentColor,4px_8px_currentColor,8px_8px_currentColor] before:content-[''] after:absolute after:left-1 after:top-1 after:size-0.5 after:rounded-[1px] after:bg-[color-mix(in_srgb,currentColor_82%,transparent)] after:opacity-[0.35] after:content-['']" />
          <span>Working…</span>
        </div>
      </MotionEnter>
    {/if}
    </div>
  </section>
  </MotionEnter>

  <MotionFade show={canScrollDown} duration={reducedMotion ? 0 : 120} class="absolute bottom-3.5 left-1/2 z-[5] -translate-x-1/2">
    <button
      class="flex h-[30px] items-center gap-1.5 rounded-full border border-line-strong bg-raised px-[11px] text-[11px] font-medium text-text-soft shadow-overlay backdrop-blur-overlay hover:bg-raised-hover hover:text-text"
      type="button"
      aria-label="Scroll to latest message"
      onclick={() => scrollToBottom()}
    >
      <IconChevronDown size={13} stroke={1.55} />
      <span>Scroll to bottom</span>
    </button>
  </MotionFade>
</div>

{#if imagePreview}
  <ImagePreview
    src={imagePreview.src}
    name={imagePreview.name}
    close={() => { imagePreview = undefined; }}
  />
{/if}
