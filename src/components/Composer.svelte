<script lang="ts">
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import {
    IconArrowUp,
    IconPaperclip,
    IconPlayerStop,
    IconPlugConnected,
    IconSparkles,
    IconX,
  } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import GitControls from './GitControls.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import ModelPicker from './ModelPicker.svelte';
  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import ReasoningPicker from './ReasoningPicker.svelte';
  import {
    listComposerCompletions,
    type ComposerCompletionEntry,
  } from '../services/native';
  import type {
    ComposerImage,
    ComposerReference,
    HarnessProfile,
    ModelOption,
    ProviderModelCatalog,
    ThreadState,
  } from '../types';
  import type { SendShortcut } from '../utils/app-settings';
  import { copyImage, saveImage } from '../utils/clipboard';
  import { composerLayoutKeyframes, usesExpandedComposerLayout, type LayoutBox } from '../utils/composer-layout';
  import { fastModeAvailable } from '../utils/fast-mode';
  import { materialFileIcon, materialFolderIcon } from '../utils/material-file-icons';
  import {
    composerEnterAction,
    fuzzyScore,
    hasPromptContent,
    REFERENCE_PLACEHOLDER,
  } from '../utils/prompt-content';
  import { activeProvider, threadHarness, threadStatus } from '../utils/threads';

  interface EditorDraft {
    draft: string;
    references: ComposerReference[];
  }

  interface Props {
    thread: ThreadState;
    catalogs: Record<string, ProviderModelCatalog>;
    profiles: HarnessProfile[];
    selectableProfiles: HarnessProfile[];
    images: ComposerImage[];
    attachmentError?: string;
    completionRevision: number;
    currentBranch: string | null | undefined;
    gitBranches: string[] | null | undefined;
    gitWorktree: boolean;
    gitEditable: boolean;
    gitLockReason?: string;
    gitBusy: boolean;
    reducedMotion: boolean;
    sendShortcut: SendShortcut;
    spellcheck: boolean;
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
    switchGitBranch: (branch: string) => Promise<void>;
    createGitWorktree: (baseBranch: string, branch: string) => Promise<void>;
  }

  const props: Props = $props();
  let modelPickerOpen = $state(false);
  let reasoningPickerOpen = $state(false);
  let expanded = $state(false);
  let composerElement = $state<HTMLElement>();
  let attachButton = $state<HTMLButtonElement>();
  let composerFooter = $state<HTMLElement>();
  let promptEditor = $state<HTMLElement>();
  let imageInput = $state<HTMLInputElement>();
  let imagePreview = $state<{ src: string; name: string }>();
  let completionEntries = $state<ComposerCompletionEntry[]>([]);
  let completionStatus = $state<'loading' | 'ready' | 'error'>('loading');
  let completionPrefix = $state<'$' | '@'>();
  let completionQuery = $state('');
  let completionIndex = $state(0);
  let completionRange: Range | undefined;
  let completionLoadToken = 0;
  let localDraft = '';
  let localThreadId = '';
  const provider = $derived(activeProvider(props.thread));
  const profile = $derived(
    props.profiles.find((candidate) => candidate.id === props.thread.profileId)
      ?? officialProfileById(props.thread.profileId)
      ?? officialProfiles[0],
  );
  const status = $derived(threadStatus(props.thread));
  const imageSupportError = $derived(
    props.images.length > 0 && !profile.supportsImages
      ? `${profile.label} does not support image prompts.`
      : '',
  );
  const completionResults = $derived.by(() => {
    const prefix = completionPrefix;
    if (!prefix || completionStatus !== 'ready') return [];
    const query = completionQuery;
    return completionEntries
      .filter((entry) => prefix === '$' ? entry.kind === 'skill' : entry.kind !== 'skill')
      .flatMap((entry) => {
        const score = query ? fuzzyScore(`${entry.name} ${entry.relativePath}`, query) : 0;
        return score === undefined ? [] : [{ entry, score }];
      })
      .sort((left, right) =>
        right.score - left.score
        || Number(right.entry.kind === 'folder') - Number(left.entry.kind === 'folder')
        || left.entry.relativePath.localeCompare(right.entry.relativePath),
      )
      .slice(0, 40)
      .map(({ entry }) => entry);
  });

  $effect(() => {
    const { id, draft } = props.thread;
    const references = props.thread.draftReferences;
    if (!promptEditor || (id === localThreadId && draft === localDraft)) return;
    if (id !== localThreadId) closeCompletion();
    localThreadId = id;
    localDraft = draft;
    void tick().then(() => {
      renderDraft(draft, references);
      resizePromptEditor();
    });
  });

  $effect(() => {
    void props.completionRevision;
    void loadCompletionEntries(props.thread.cwd);
  });

  async function loadCompletionEntries(cwd: string) {
    const token = ++completionLoadToken;
    completionStatus = 'loading';
    void tick().then(updateMissingPills);
    try {
      const entries = await listComposerCompletions(cwd);
      if (token !== completionLoadToken) return;
      completionEntries = entries;
      completionStatus = 'ready';
      updateMissingPills();
    } catch {
      if (token === completionLoadToken) {
        completionStatus = 'error';
        updateMissingPills();
      }
    }
  }

  function activeModelName() {
    const catalog = props.catalogs[props.thread.profileId];
    const model = catalog?.models.find((item) => item.id === provider.selectedModelId);
    if (model) {
      const separator = model.name.indexOf('/');
      return separator > 0 && separator < model.name.length - 1
        ? model.name.slice(separator + 1).trim()
        : model.name;
    }
    return catalog?.status === 'loading' ? 'Loading models…' : 'Choose model';
  }

  function canEdit() {
    return status === 'disconnected' || status === 'ready';
  }

  function canSend() {
    return canEdit()
      && props.catalogs[props.thread.profileId]?.status === 'ready'
      && props.selectableProfiles.some((candidate) => candidate.id === props.thread.profileId)
      && Boolean(provider.selectedModelId)
      && !imageSupportError;
  }

  function resizePromptEditor() {
    if (!promptEditor) return;
    const nextExpanded = usesExpandedComposerLayout(measureCompactPromptHeight());
    const previousLayout = nextExpanded === expanded ? undefined : captureComposerLayout();
    expanded = nextExpanded;
    if (previousLayout) {
      void tick().then(() => {
        sizePromptEditor();
        animateComposerLayout(previousLayout);
      });
    } else {
      sizePromptEditor();
    }
  }

  function measureCompactPromptHeight() {
    if (!composerElement || !promptEditor || !attachButton || !composerFooter) return 0;
    const composerStyle = getComputedStyle(composerElement);
    const horizontalPadding = Number.parseFloat(composerStyle.paddingLeft)
      + Number.parseFloat(composerStyle.paddingRight);
    const columnGap = Number.parseFloat(composerStyle.columnGap);
    const compactWidth = composerElement.clientWidth
      - horizontalPadding
      - attachButton.offsetWidth
      - composerFooter.offsetWidth
      - columnGap * 2;
    const previousWidth = promptEditor.style.width;
    const previousHeight = promptEditor.style.height;
    promptEditor.style.width = `${Math.max(1, compactWidth)}px`;
    promptEditor.style.height = 'auto';
    const height = promptEditor.scrollHeight;
    promptEditor.style.width = previousWidth;
    promptEditor.style.height = previousHeight;
    return height;
  }

  function sizePromptEditor() {
    if (!promptEditor) return;
    promptEditor.style.height = 'auto';
    const maxHeight = Number.parseFloat(getComputedStyle(promptEditor).maxHeight);
    const height = Number.isFinite(maxHeight)
      ? Math.min(promptEditor.scrollHeight, maxHeight)
      : promptEditor.scrollHeight;
    promptEditor.style.height = `${height}px`;
    promptEditor.style.overflowY = promptEditor.scrollHeight > height ? 'auto' : 'hidden';
  }

  function layoutBox(element: Element): LayoutBox {
    const { left, top, width, height } = element.getBoundingClientRect();
    return { left, top, width, height };
  }

  function captureComposerLayout() {
    if (!composerElement || !promptEditor || !attachButton || !composerFooter) return;
    return {
      composerHeight: composerElement.getBoundingClientRect().height,
      elements: [promptEditor, attachButton, composerFooter].map((element) => ({
        element,
        previous: layoutBox(element),
      })),
    };
  }

  function animateComposerLayout(layout: NonNullable<ReturnType<typeof captureComposerLayout>>) {
    if (!composerElement || props.reducedMotion) return;
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

  function closeCompletion() {
    completionPrefix = undefined;
    completionQuery = '';
    completionIndex = 0;
    completionRange = undefined;
  }

  function detectCompletion() {
    const selection = window.getSelection();
    const node = selection?.anchorNode;
    if (!promptEditor || !node || node.nodeType !== Node.TEXT_NODE || !promptEditor.contains(node)) {
      closeCompletion();
      return;
    }
    const text = node.textContent?.slice(0, selection?.anchorOffset ?? 0) ?? '';
    const match = /(?:^|\s)([$@])([^\s$@]*)$/.exec(text);
    if (!match) {
      closeCompletion();
      return;
    }
    if (!completionPrefix && completionStatus !== 'ready') {
      void loadCompletionEntries(props.thread.cwd);
    }
    completionPrefix = match[1] === '$' ? '$' : '@';
    completionQuery = match[2];
    completionIndex = 0;
    const end = selection?.anchorOffset ?? 0;
    completionRange = document.createRange();
    completionRange.setStart(node, end - completionQuery.length - 1);
    completionRange.setEnd(node, end);
  }

  function referenceAvailable(reference: ComposerReference) {
    return completionEntries.some((entry) => entry.kind === reference.kind && entry.path === reference.path);
  }

  function hasMissingReferences() {
    return completionStatus === 'ready'
      && props.thread.draftReferences.some((reference) => !referenceAvailable(reference));
  }

  function createReferenceElement(reference: ComposerReference) {
    const pill = document.createElement('span');
    pill.className = `composer-reference ${reference.kind === 'skill' ? 'skill' : ''}`;
    pill.contentEditable = 'false';
    pill.dataset.referenceId = reference.id;
    pill.title = reference.relativePath;
    pill.setAttribute('aria-label', `${reference.kind} ${reference.name}`);

    if (reference.kind === 'skill') {
      const mark = document.createElement('span');
      mark.className = 'composer-reference-mark';
      mark.textContent = '$';
      pill.append(mark);
    } else {
      const icon = reference.kind === 'folder'
        ? materialFolderIcon(reference.name, false)
        : materialFileIcon(reference.name);
      if (icon) {
        const image = document.createElement('img');
        image.src = icon;
        image.alt = '';
        pill.append(image);
      }
    }
    const label = document.createElement('span');
    label.textContent = reference.name;
    pill.append(label);
    return pill;
  }

  function renderDraft(draft: string, references: ComposerReference[]) {
    if (!promptEditor) return;
    promptEditor.replaceChildren();
    const texts = draft.split(REFERENCE_PLACEHOLDER);
    for (const [index, text] of texts.entries()) {
      if (text) promptEditor.append(document.createTextNode(text));
      const reference = references[index];
      if (reference) promptEditor.append(createReferenceElement(reference));
    }
    updateMissingPills();
  }

  function updateMissingPills() {
    if (!promptEditor) return;
    const references = new Map(props.thread.draftReferences.map((reference) => [reference.id, reference]));
    for (const pill of promptEditor.querySelectorAll<HTMLElement>('[data-reference-id]')) {
      const reference = references.get(pill.dataset.referenceId ?? '');
      const missing = completionStatus === 'ready' && reference && !referenceAvailable(reference);
      pill.classList.toggle('missing', Boolean(missing));
      if (reference) pill.title = missing ? `${reference.relativePath} is missing` : reference.relativePath;
    }
  }

  function readEditorDraft(): EditorDraft {
    if (!promptEditor) return { draft: '', references: [] };
    const known = new Map(props.thread.draftReferences.map((reference) => [reference.id, reference]));
    const references: ComposerReference[] = [];
    let draft = '';
    const readNode = (node: Node) => {
      if (node.nodeType === Node.TEXT_NODE) {
        draft += node.textContent ?? '';
        return;
      }
      if (!(node instanceof HTMLElement)) return;
      const reference = known.get(node.dataset.referenceId ?? '');
      if (reference) {
        draft += REFERENCE_PLACEHOLDER;
        references.push(reference);
        return;
      }
      if (node.tagName === 'BR') {
        draft += '\n';
        return;
      }
      for (const child of node.childNodes) readNode(child);
    };
    for (const child of promptEditor.childNodes) readNode(child);
    return { draft, references };
  }

  function syncDraftFromEditor() {
    const { draft, references } = readEditorDraft();
    props.thread.draft = draft;
    props.thread.draftReferences = references;
    localDraft = draft;
    updateMissingPills();
  }

  function insertText(text: string) {
    const selection = window.getSelection();
    if (!selection?.rangeCount) return;
    const range = selection.getRangeAt(0);
    range.deleteContents();
    const node = document.createTextNode(text);
    range.insertNode(node);
    range.setStartAfter(node);
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
    syncDraftFromEditor();
    detectCompletion();
    resizePromptEditor();
  }

  function fileUri(path: string) {
    const normalized = path.replace(/^\\\\\?\\/, '').replaceAll('\\', '/');
    return new URL(normalized.startsWith('//') ? `file:${normalized}` : `file://${normalized.startsWith('/') ? '' : '/'}${normalized}`).href;
  }

  function chooseCompletion(entry: ComposerCompletionEntry) {
    const range = completionRange?.cloneRange();
    if (!promptEditor || !range || !promptEditor.contains(range.commonAncestorContainer)) return;
    const reference: ComposerReference = {
      id: crypto.randomUUID(),
      kind: entry.kind,
      name: entry.name,
      path: entry.path,
      relativePath: entry.relativePath,
      uri: fileUri(entry.path),
    };
    props.thread.draftReferences = [...props.thread.draftReferences, reference];
    range.deleteContents();
    const pill = createReferenceElement(reference);
    const trailingSpace = document.createTextNode(' ');
    range.insertNode(trailingSpace);
    range.insertNode(pill);
    range.setStart(trailingSpace, 1);
    range.collapse(true);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    closeCompletion();
    syncDraftFromEditor();
    resizePromptEditor();
    promptEditor.focus();
  }

  function handleEditorInput() {
    if (promptEditor?.childNodes.length === 1 && promptEditor.firstChild?.nodeName === 'BR') {
      promptEditor.replaceChildren();
    }
    syncDraftFromEditor();
    detectCompletion();
    resizePromptEditor();
  }

  function handleEditorKeydown(event: KeyboardEvent) {
    if (completionPrefix) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        if (completionResults.length > 0) {
          completionIndex = (completionIndex + (event.key === 'ArrowDown' ? 1 : -1) + completionResults.length) % completionResults.length;
          void tick().then(() =>
            document.getElementById(`composer-completion-${completionIndex}`)?.scrollIntoView({ block: 'nearest' }),
          );
        }
        return;
      }
      if (event.key === 'Enter' || event.key === 'Tab') {
        if (event.key === 'Enter' && completionStatus === 'loading') {
          event.preventDefault();
          return;
        }
        const entry = completionResults[completionIndex];
        if (entry) {
          event.preventDefault();
          chooseCompletion(entry);
          return;
        }
        closeCompletion();
      }
      if (event.key === 'Escape') {
        event.preventDefault();
        closeCompletion();
        return;
      }
    }
    const enterAction = composerEnterAction(props.sendShortcut, event);
    if (enterAction) {
      event.preventDefault();
      if (enterAction === 'newline') insertText('\n');
      else if (!hasMissingReferences()) props.send();
    }
  }

  function imageMenuItems(image: ComposerImage) {
    return [
      { label: 'Open preview', action: () => { imagePreview = { src: image.previewUrl, name: image.name }; } },
      { label: 'Copy image', action: () => copyImage(image.previewUrl) },
      { label: 'Save image', action: () => saveImage(image.previewUrl, image.name) },
      { label: 'Remove attachment', action: () => props.removeImage(image.id), danger: true, separatorBefore: true },
    ];
  }

  function handleImageSelection(event: Event) {
    if (!(event.currentTarget instanceof HTMLInputElement)) return;
    const input = event.currentTarget;
    const files = input.files ? Array.from(input.files) : [];
    input.value = '';
    props.attachImages(files);
  }

  function handlePaste(event: ClipboardEvent) {
    event.preventDefault();
    const files = Array.from(event.clipboardData?.items ?? [])
      .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
      .flatMap((item) => {
        const file = item.getAsFile();
        return file ? [file] : [];
      });
    const text = event.clipboardData?.getData('text/plain') ?? '';
    if (text) insertText(text);
    if (files.length > 0 && profile.supportsImages) props.attachImages(files);
  }

  function chooseModel(profileId: string, model: ModelOption) {
    props.activateProvider(profileId);
    modelPickerOpen = false;
    props.selectModel(profileId, model);
  }
</script>

<section
  class:picker-open={modelPickerOpen || reasoningPickerOpen}
  class="composer-wrap"
  in:fly|global={{ y: props.reducedMotion ? 0 : 4, duration: props.reducedMotion ? 0 : 180 }}
>
  <div bind:this={composerElement} class="composer" class:expanded={expanded} class:working={status === 'running'}>
    {#if props.images.length > 0 || props.attachmentError || imageSupportError}
      <div class="attachment-strip" aria-label="Attached images">
        {#each props.images as image (image.id)}
          <ContextMenu items={imageMenuItems(image)}>
            {#snippet children({ props: imageProps })}
              <div {...imageProps} class="image-attachment" role="group" title={image.name}>
                <button
                  type="button"
                  class="image-attachment-preview"
                  aria-label={`Preview ${image.name}`}
                  onclick={() => { imagePreview = { src: image.previewUrl, name: image.name }; }}
                ><img src={image.previewUrl} alt="" /></button>
                <button class="image-attachment-remove" type="button" aria-label={`Remove ${image.name}`} title={`Remove ${image.name}`} onclick={() => props.removeImage(image.id)}>
                  <IconX size={10} stroke={1.55} />
                </button>
              </div>
            {/snippet}
          </ContextMenu>
        {/each}
        {#if imageSupportError}<span class="attachment-error">{imageSupportError}</span>{/if}
        {#if props.attachmentError}<span class="attachment-error">{props.attachmentError}</span>{/if}
      </div>
    {/if}
    <input bind:this={imageInput} class="image-input" type="file" accept="image/*" multiple disabled={!canEdit() || !profile.supportsImages} onchange={handleImageSelection} />
    <button
      bind:this={attachButton}
      class="attach-button"
      type="button"
      aria-label={profile.supportsImages ? 'Attach images' : `${profile.label} does not support images`}
      title={profile.supportsImages ? 'Attach images (you can also paste them)' : `${profile.label} does not support image prompts`}
      disabled={!canEdit() || !profile.supportsImages}
      onclick={() => imageInput?.click()}
    >
      <IconPaperclip size={16} stroke={1.55} />
    </button>
    <div
      bind:this={promptEditor}
      class="prompt-editor"
      role="textbox"
      aria-label="Prompt"
      aria-autocomplete="list"
      aria-haspopup="listbox"
      aria-controls={completionPrefix ? 'composer-autocomplete' : undefined}
      aria-activedescendant={completionPrefix && completionResults[completionIndex] ? `composer-completion-${completionIndex}` : undefined}
      aria-disabled={!canEdit()}
      aria-multiline="true"
      tabindex={canEdit() ? 0 : -1}
      contenteditable={canEdit()}
      spellcheck={props.spellcheck}
      data-placeholder={status === 'running'
        ? 'Agent is working…'
        : status === 'connecting'
          ? `Starting ${threadHarness(props.thread, props.profiles)}…`
          : status === 'error'
            ? provider.turnStatus === 'blocked'
              ? 'Turn state is inconsistent — reconnect provider'
              : `${threadHarness(props.thread, props.profiles)} unavailable — switch provider or retry`
            : status === 'stopped'
              ? 'Provider stopped — switch provider or reconnect'
              : 'Ask anything…'}
      oninput={handleEditorInput}
      onpaste={handlePaste}
      onkeydown={handleEditorKeydown}
      onblur={() => { window.setTimeout(closeCompletion, 100); }}
    ></div>
    {#if completionPrefix}
      <div id="composer-autocomplete" class="composer-autocomplete" role="listbox" aria-label={completionPrefix === '$' ? 'Skills' : 'Workspace files'}>
        {#if completionStatus === 'loading'}
          <p>Loading…</p>
        {:else if completionStatus === 'error'}
          <p>Autocomplete unavailable.</p>
        {:else if completionResults.length === 0}
          <p>No matches.</p>
        {:else}
          {#each completionResults as entry, index (`${entry.kind}-${entry.path}`)}
            <button
              id={`composer-completion-${index}`}
              type="button"
              role="option"
              aria-selected={index === completionIndex}
              class:active={index === completionIndex}
              onpointerenter={() => { completionIndex = index; }}
              onpointerdown={(event) => {
                event.preventDefault();
                chooseCompletion(entry);
              }}
            >
              {#if entry.kind === 'skill'}
                <span class="composer-completion-skill"><IconSparkles size={14} stroke={1.55} /></span>
              {:else}
                {@const icon = entry.kind === 'folder' ? materialFolderIcon(entry.name, false) : materialFileIcon(entry.name)}
                {#if icon}<img src={icon} alt="" />{/if}
              {/if}
              <span class="composer-completion-copy">
                <strong>{entry.name}</strong>
                <small>{entry.description ?? entry.relativePath}</small>
              </span>
            </button>
          {/each}
        {/if}
      </div>
    {/if}
    <div bind:this={composerFooter} class="composer-footer">
      <div class="composer-context">
          <div class="model-picker-wrap">
            {#if props.selectableProfiles.length > 0}
              <ModelPicker
                thread={props.thread}
                catalogs={props.catalogs}
                profiles={props.selectableProfiles}
                label={activeModelName()}
                open={modelPickerOpen}
                setOpen={(open) => { reasoningPickerOpen = false; modelPickerOpen = open; }}
                choose={chooseModel}
                retryDiscovery={props.retryDiscovery}
              />
            {:else}
              <button class="model-picker-trigger" title="No authenticated providers" disabled>
                <img class:brand-color-icon={profile.iconMode === 'brand'} src={profile.icon} alt="" />
                <span>No providers</span>
              </button>
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
        <button class="cancel-button" aria-label="Cancel turn" title="Cancel turn" onclick={props.cancel}><IconPlayerStop size={13} fill="currentColor" stroke={1.55} /></button>
      {:else if status === 'error' || status === 'stopped'}
        <button class="reconnect-button" aria-label="Reconnect provider" title="Reconnect provider" onclick={props.reconnect}><IconPlugConnected size={15} stroke={1.55} /></button>
      {:else}
        <button class="send-button" aria-label="Send prompt" title={hasMissingReferences() ? 'Remove missing references before sending' : props.gitBusy ? 'Wait for the Git operation to finish' : 'Send prompt'} disabled={props.gitBusy || (!hasPromptContent(props.thread.draft, props.thread.draftReferences) && props.images.length === 0) || !canSend() || hasMissingReferences()} onclick={props.send}>
          <IconArrowUp size={17} stroke={1.55} />
        </button>
      {/if}
    </div>
  </div>
  <div class="composer-meta">
    <div class="composer-meta-actions">
      <GitControls
        cwd={props.thread.cwd}
        currentBranch={props.currentBranch}
        branches={props.gitBranches}
        worktree={props.gitWorktree}
        editable={props.gitEditable}
        lockReason={props.gitLockReason}
        busy={props.gitBusy}
        switchBranch={props.switchGitBranch}
        createWorktree={props.createGitWorktree}
      />
    </div>
  </div>
</section>

{#if imagePreview}
  <ImagePreview
    src={imagePreview.src}
    name={imagePreview.name}
    close={() => { imagePreview = undefined; }}
  />
{/if}

<svelte:window onblur={closeCompletion} />
