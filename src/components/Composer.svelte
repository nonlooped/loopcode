<script lang="ts">
  import { tick } from 'svelte';
  import {
    IconArrowUp,
    IconChevronDown,
    IconFolder,
    IconGitBranch,
    IconPaperclip,
    IconPlayerStop,
    IconPlugConnected,
    IconX,
  } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import ModelPicker from './ModelPicker.svelte';
  import ReasoningPicker from './ReasoningPicker.svelte';
  import { profileById } from '../config/providers';
  import type { ComposerImage, ModelOption, ProviderModelCatalog, ThreadState } from '../types';
  import { copyImage, saveImage } from '../utils/clipboard';
  import { composerLayoutKeyframes, usesExpandedComposerLayout, type LayoutBox } from '../utils/composer-layout';
  import { menuFromEvent, type ContextMenuState } from '../utils/context-menu';
  import { fastModeAvailable } from '../utils/fast-mode';
  import { activeProvider, threadHarness, threadStatus } from '../utils/threads';

  interface Props {
    thread: ThreadState;
    catalogs: Record<string, ProviderModelCatalog>;
    images: ComposerImage[];
    attachmentError?: string;
    projectName: string;
    currentBranch: string | null | undefined;
    attachImages: (files: File[]) => void;
    removeImage: (imageId: string) => void;
    send: () => void;
    cancel: () => void;
    reconnect: () => void;
    selectModel: (profileId: string, model: ModelOption) => void;
    selectReasoning: (reasoningId: string) => void;
    selectFastMode: (enabled: boolean) => void;
    activateProvider: (profileId: string) => void;
    retryDiscovery: (profileId: string) => void;
  }

  const props: Props = $props();
  let modelPickerOpen = $state(false);
  let reasoningPickerOpen = $state(false);
  let expanded = $state(false);
  let composerElement = $state<HTMLElement>();
  let attachButton = $state<HTMLButtonElement>();
  let composerFooter = $state<HTMLElement>();
  let promptTextarea = $state<HTMLTextAreaElement>();
  let imageInput = $state<HTMLInputElement>();
  let contextMenu = $state<ContextMenuState>();
  let imagePreview = $state<{ src: string; name: string }>();
  const provider = $derived(activeProvider(props.thread));
  const status = $derived(threadStatus(props.thread));

  $effect(() => {
    const draft = props.thread.draft;
    void draft;
    void tick().then(resizePromptTextarea);
  });

  function closePickers() {
    modelPickerOpen = false;
    reasoningPickerOpen = false;
  }

  function activeModelName() {
    const catalog = props.catalogs[props.thread.profileId];
    const model = catalog.models.find((item) => item.id === provider.selectedModelId);
    if (model) {
      const separator = model.name.indexOf('/');
      return separator > 0 && separator < model.name.length - 1
        ? model.name.slice(separator + 1).trim()
        : model.name;
    }
    return catalog.status === 'loading' ? 'Loading models…' : 'Choose model';
  }

  function canEdit() {
    return status === 'disconnected' || status === 'ready';
  }

  function canSend() {
    return canEdit()
      && props.catalogs[props.thread.profileId]?.status === 'ready'
      && Boolean(provider.selectedModelId);
  }

  function resizePromptTextarea() {
    if (!promptTextarea) return;
    const nextExpanded = usesExpandedComposerLayout(measureCompactPromptHeight());
    const previousLayout = nextExpanded === expanded ? undefined : captureComposerLayout();
    expanded = nextExpanded;
    if (previousLayout) {
      void tick().then(() => {
        sizePromptTextarea();
        animateComposerLayout(previousLayout);
      });
    } else {
      sizePromptTextarea();
    }
  }

  function measureCompactPromptHeight() {
    if (!composerElement || !promptTextarea || !attachButton || !composerFooter) return 0;
    const composerStyle = getComputedStyle(composerElement);
    const horizontalPadding = Number.parseFloat(composerStyle.paddingLeft)
      + Number.parseFloat(composerStyle.paddingRight);
    const columnGap = Number.parseFloat(composerStyle.columnGap);
    const compactWidth = composerElement.clientWidth
      - horizontalPadding
      - attachButton.offsetWidth
      - composerFooter.offsetWidth
      - columnGap * 2;
    const previousWidth = promptTextarea.style.width;
    const previousHeight = promptTextarea.style.height;
    promptTextarea.style.width = `${Math.max(1, compactWidth)}px`;
    promptTextarea.style.height = 'auto';
    const height = promptTextarea.scrollHeight;
    promptTextarea.style.width = previousWidth;
    promptTextarea.style.height = previousHeight;
    return height;
  }

  function sizePromptTextarea() {
    if (!promptTextarea) return;
    promptTextarea.style.height = 'auto';
    const maxHeight = Number.parseFloat(getComputedStyle(promptTextarea).maxHeight);
    const height = Number.isFinite(maxHeight)
      ? Math.min(promptTextarea.scrollHeight, maxHeight)
      : promptTextarea.scrollHeight;
    promptTextarea.style.height = `${height}px`;
    promptTextarea.style.overflowY = promptTextarea.scrollHeight > height ? 'auto' : 'hidden';
  }

  function layoutBox(element: Element): LayoutBox {
    const { left, top, width, height } = element.getBoundingClientRect();
    return { left, top, width, height };
  }

  function captureComposerLayout() {
    if (!composerElement || !promptTextarea || !attachButton || !composerFooter) return;
    return {
      composerHeight: composerElement.getBoundingClientRect().height,
      elements: [promptTextarea, attachButton, composerFooter].map((element) => ({
        element,
        previous: layoutBox(element),
      })),
    };
  }

  function animateComposerLayout(layout: NonNullable<ReturnType<typeof captureComposerLayout>>) {
    if (!composerElement || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const timing: KeyframeAnimationOptions = {
      duration: 240,
      easing: 'cubic-bezier(0.22, 1, 0.36, 1)',
    };
    const currentHeight = composerElement.getBoundingClientRect().height;
    composerElement.animate(
      [{ height: `${layout.composerHeight}px` }, { height: `${currentHeight}px` }],
      timing,
    );
    for (const { element, previous } of layout.elements) {
      const current = layoutBox(element);
      element.animate(composerLayoutKeyframes(previous, current), timing);
    }
  }

  function openImageMenu(event: MouseEvent, image: ComposerImage) {
    contextMenu = menuFromEvent(event, [
      { label: 'Open preview', action: () => { imagePreview = { src: image.previewUrl, name: image.name }; } },
      { label: 'Copy image', action: () => copyImage(image.previewUrl) },
      { label: 'Save image', action: () => saveImage(image.previewUrl, image.name) },
      { label: 'Remove attachment', action: () => props.removeImage(image.id), danger: true, separatorBefore: true },
    ]);
  }

  function handleImageSelection(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const files = input.files ? Array.from(input.files) : [];
    input.value = '';
    props.attachImages(files);
  }

  function handlePaste(event: ClipboardEvent) {
    const files = Array.from(event.clipboardData?.items ?? [])
      .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
      .flatMap((item) => {
        const file = item.getAsFile();
        return file ? [file] : [];
      });
    if (files.length === 0) return;
    if (!event.clipboardData?.getData('text/plain')) event.preventDefault();
    props.attachImages(files);
  }

  function chooseModel(profileId: string, model: ModelOption) {
    props.activateProvider(profileId);
    modelPickerOpen = false;
    props.selectModel(profileId, model);
  }
</script>

{#if modelPickerOpen || reasoningPickerOpen}
  <button class="model-picker-dismiss" tabindex="-1" aria-label="Close picker" onclick={closePickers}></button>
{/if}

<section class:picker-open={modelPickerOpen || reasoningPickerOpen} class="composer-wrap">
  <div bind:this={composerElement} class="composer" class:expanded={expanded} class:working={status === 'running'}>
    {#if props.images.length > 0 || props.attachmentError}
      <div class="attachment-strip" aria-label="Attached images">
        {#each props.images as image (image.id)}
          <div class="image-attachment" role="group" title={image.name} oncontextmenu={(event) => openImageMenu(event, image)}>
            <img src={image.previewUrl} alt={image.name} />
            <button type="button" aria-label={`Remove ${image.name}`} title={`Remove ${image.name}`} onclick={() => props.removeImage(image.id)}>
              <IconX size={10} stroke={2} />
            </button>
          </div>
        {/each}
        {#if props.attachmentError}<span class="attachment-error">{props.attachmentError}</span>{/if}
      </div>
    {/if}
    <input bind:this={imageInput} class="image-input" type="file" accept="image/*" multiple disabled={!canEdit()} onchange={handleImageSelection} />
    <button bind:this={attachButton} class="attach-button" type="button" aria-label="Attach images" title="Attach images (you can also paste them)" disabled={!canEdit()} onclick={() => imageInput?.click()}>
      <IconPaperclip size={16} stroke={1.7} />
    </button>
    <textarea
      bind:this={promptTextarea}
      bind:value={props.thread.draft}
      aria-label="Prompt"
      placeholder={status === 'running'
        ? 'Agent is working...'
        : status === 'connecting'
          ? `Starting ${threadHarness(props.thread)}...`
          : status === 'error'
            ? provider.turnStatus === 'blocked'
              ? 'Turn state is inconsistent — reconnect provider'
              : `${threadHarness(props.thread)} unavailable — switch provider or retry`
            : status === 'stopped'
              ? 'Provider stopped — switch provider or reconnect'
              : 'Do anything...'}
      disabled={!canEdit()}
      rows="1"
      oninput={resizePromptTextarea}
      onpaste={handlePaste}
      onkeydown={(event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
          event.preventDefault();
          props.send();
        }
      }}
    ></textarea>
    <div bind:this={composerFooter} class="composer-footer">
      <div class="composer-context">
        <div class="model-picker-wrap">
          <button
            class="model-picker-trigger"
            aria-expanded={modelPickerOpen}
            aria-haspopup="dialog"
            title="Choose provider and model"
            onclick={() => { reasoningPickerOpen = false; modelPickerOpen = !modelPickerOpen; }}
          >
            <img src={profileById(props.thread.profileId).icon} alt="" />
            <span>{activeModelName()}</span>
            <IconChevronDown size={11} stroke={1.7} />
          </button>
          {#if modelPickerOpen}
            <ModelPicker thread={props.thread} catalogs={props.catalogs} choose={chooseModel} retryDiscovery={props.retryDiscovery} />
          {/if}
        </div>
        {#if provider.reasoningOptions.length > 1 || fastModeAvailable(provider)}
          <ReasoningPicker
            {provider}
            open={reasoningPickerOpen}
            setOpen={(open) => { modelPickerOpen = false; reasoningPickerOpen = open; }}
            select={props.selectReasoning}
            selectFastMode={props.selectFastMode}
          />
        {/if}
      </div>
      {#if status === 'running'}
        <button class="cancel-button" aria-label="Cancel turn" title="Cancel turn" onclick={props.cancel}><IconPlayerStop size={13} fill="currentColor" stroke={1.5} /></button>
      {:else if status === 'error' || status === 'stopped'}
        <button class="reconnect-button" aria-label="Reconnect provider" title="Reconnect provider" onclick={props.reconnect}><IconPlugConnected size={15} stroke={1.7} /></button>
      {:else}
        <button class="send-button" aria-label="Send prompt" title="Send prompt" disabled={(!props.thread.draft.trim() && props.images.length === 0) || !canSend()} onclick={props.send}>
          <IconArrowUp size={17} stroke={2} />
        </button>
      {/if}
    </div>
  </div>
  <div class="composer-meta">
    <span class="composer-meta-project" title={props.thread.cwd}><IconFolder size={12} stroke={1.55} /><span>{props.projectName}</span></span>
    <span class="composer-meta-actions">
      <span title={props.currentBranch ?? 'No Git branch'} class="worktree-path">
        <IconGitBranch size={12} stroke={1.55} />
        {props.currentBranch === undefined ? 'Resolving branch…' : props.currentBranch ?? 'No Git branch'}
      </span>
    </span>
  </div>
</section>

{#if contextMenu}
  <ContextMenu menu={contextMenu} close={() => { contextMenu = undefined; }} />
{/if}

{#if imagePreview}
  <ImagePreview src={imagePreview.src} name={imagePreview.name} close={() => { imagePreview = undefined; }} />
{/if}

<svelte:window onkeydown={(event) => {
  if (event.key === 'Escape' && (modelPickerOpen || reasoningPickerOpen)) closePickers();
}} />
