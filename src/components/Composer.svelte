<script lang="ts">
  import { tick } from 'svelte';
  import {
    IconArrowUp,
    IconPaperclip,
    IconPlayerStop,
    IconPlugConnected,
    IconSlash,
    IconSparkles,
    IconX,
  } from '@tabler/icons-svelte';

  import ContextMenu from './ContextMenu.svelte';
  import GitControls from './GitControls.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import ContextMeter from './ContextMeter.svelte';
  import ModelPicker from './ModelPicker.svelte';
  import MotionFly from './motion/MotionFly.svelte';
  import { profileById as officialProfileById, profiles as officialProfiles } from '../config/providers';
  import SessionPicker from './SessionPicker.svelte';
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
    SessionSelectId,
    SlashCommand,
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
    selectSessionOption: (option: SessionSelectId, valueId: string) => void;
    activateProvider: (profileId: string) => void;
    retryDiscovery: (profileId: string) => void;
    switchGitBranch: (branch: string) => Promise<void>;
    createGitWorktree: (baseBranch: string, branch: string) => Promise<void>;
  }

  const props: Props = $props();
  let modelPickerOpen = $state(false);
  let sessionPickerOpen = $state(false);
  let expanded = $state(false);
  let composerElement = $state<HTMLElement>();
  let attachButton = $state<HTMLButtonElement>();
  let composerFooter = $state<HTMLElement>();
  let promptEditor = $state<HTMLElement>();
  let imageInput = $state<HTMLInputElement>();
  let imagePreview = $state<{ src: string; name: string }>();
  let completionEntries = $state<ComposerCompletionEntry[]>([]);
  let completionStatus = $state<'loading' | 'ready' | 'error'>('loading');
  let completionPrefix = $state<'$' | '@' | '/'>();
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
  const sessionOptionsAvailable = $derived(
    provider.reasoningOptions.length > 1
    || fastModeAvailable(provider)
    || Object.values(provider.selects ?? {}).some((select) => select.options.length > 1),
  );
  const commandResults = $derived.by(() => {
    if (completionPrefix !== '/') return [];
    const query = completionQuery;
    const skills = new Set(
      completionEntries
        .filter((entry) => entry.kind === 'skill')
        .map((entry) => entry.name.toLowerCase()),
    );
    return (provider.commands ?? [])
      .filter((command) =>
        // Providers publish internal commands under a `$` prefix; they are not meant to be
        // invoked from a composer, so they never reach the completion list.
        !command.name.startsWith('$')
        && !(profile.id === 'claude' && skills.has(command.name.toLowerCase())),
      )
      .flatMap((command) => {
        const score = query ? fuzzyScore(`${command.name} ${command.description}`, query) : 0;
        return score === undefined ? [] : [{ command, score }];
      })
      .sort((left, right) => right.score - left.score || left.command.name.localeCompare(right.command.name))
      .slice(0, 40)
      .map(({ command }) => command);
  });
  const completionResults = $derived.by(() => {
    const prefix = completionPrefix;
    if (prefix === '/') return commandResults;
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
    return status === 'disconnected' || status === 'ready' || (status === 'running' && provider.supportsFollowups === true);
  }

  function canSend() {
    return canEdit()
      && props.catalogs[props.thread.profileId]?.status === 'ready'
      && props.selectableProfiles.some((candidate) => candidate.id === props.thread.profileId)
      && Boolean(provider.selectedModelId);
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
    const command = commandCompletion(node, text);
    const match = command ?? /(?:^|\s)([$@])([^\s$@]*)$/.exec(text);
    if (!match) {
      closeCompletion();
      return;
    }
    if (!command && !completionPrefix && completionStatus !== 'ready') {
      void loadCompletionEntries(props.thread.cwd);
    }
    completionPrefix = match[1] === '$' ? '$' : match[1] === '/' ? '/' : '@';
    completionQuery = match[2];
    completionIndex = 0;
    const end = selection?.anchorOffset ?? 0;
    completionRange = document.createRange();
    completionRange.setStart(node, end - completionQuery.length - 1);
    completionRange.setEnd(node, end);
  }

  /** Codex only reads a leading `/name`, so the command menu never opens mid-prompt. */
  function commandCompletion(node: Node, text: string) {
    if ((provider.commands ?? []).length === 0) return undefined;
    if (promptEditor?.firstChild !== node) return undefined;
    return /^(\/)([^\s/]*)$/.exec(text);
  }

  function chooseCommand(command: SlashCommand) {
    const range = completionRange?.cloneRange();
    if (!promptEditor || !range || !promptEditor.contains(range.commonAncestorContainer)) return;
    range.deleteContents();
    const text = document.createTextNode(`/${command.name} `);
    range.insertNode(text);
    range.setStart(text, text.length);
    range.collapse(true);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    closeCompletion();
    syncDraftFromEditor();
    resizePromptEditor();
    promptEditor.focus();
  }

  function isCommand(entry: ComposerCompletionEntry | SlashCommand): entry is SlashCommand {
    return !('kind' in entry);
  }

  function applyCompletion(entry: ComposerCompletionEntry | SlashCommand) {
    if (isCommand(entry)) chooseCommand(entry);
    else chooseCompletion(entry);
  }

  function referenceAvailable(reference: ComposerReference) {
    return completionEntries.some((entry) => entry.kind === reference.kind && entry.path === reference.path);
  }

  function hasMissingReferences() {
    return completionStatus === 'ready'
      && props.thread.draftReferences.some((reference) => !referenceAvailable(reference));
  }

  const referencePillClass = (reference: ComposerReference) =>
    `composer-reference inline-flex max-w-[180px] h-[1.45em] mx-px px-[5px] items-center gap-1 rounded-[5px] bg-panel-active text-text-soft text-[13px] leading-none whitespace-nowrap ${
      reference.kind === 'skill' ? 'align-baseline' : 'align-[-0.18em]'
    }`;

  function createReferenceElement(reference: ComposerReference) {
    const pill = document.createElement('span');
    pill.className = referencePillClass(reference);
    pill.contentEditable = 'false';
    pill.dataset.referenceId = reference.id;
    pill.title = reference.relativePath;
    pill.setAttribute('aria-label', `${reference.kind} ${reference.name}`);

    if (reference.kind === 'skill') {
      const mark = document.createElement('span');
      mark.className = 'font-semibold text-muted';
      mark.textContent = '$';
      pill.append(mark);
    } else {
      const icon = reference.kind === 'folder'
        ? materialFolderIcon(reference.name, false)
        : materialFileIcon(reference.name);
      if (icon) {
        const image = document.createElement('img');
        image.className = 'size-3 shrink-0 opacity-[0.64] [filter:var(--provider-filter)]';
        image.src = icon;
        image.alt = '';
        pill.append(image);
      }
    }
    const label = document.createElement('span');
    label.className = 'min-w-0 overflow-hidden text-ellipsis';
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
      pill.classList.toggle('border', Boolean(missing));
      pill.classList.toggle('border-[color-mix(in_srgb,var(--danger)_50%,transparent)]', Boolean(missing));
      pill.classList.toggle('text-danger', Boolean(missing));
      pill.classList.toggle('line-through', Boolean(missing));
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
    if (event.isComposing) return;
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
        if (event.key === 'Enter' && completionPrefix !== '/' && completionStatus === 'loading') {
          event.preventDefault();
          return;
        }
        const entry = completionResults[completionIndex];
        if (entry) {
          event.preventDefault();
          applyCompletion(entry);
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
    if (files.length > 0) props.attachImages(files);
  }

  function chooseModel(profileId: string, model: ModelOption) {
    props.activateProvider(profileId);
    modelPickerOpen = false;
    props.selectModel(profileId, model);
  }
</script>

<MotionFly y={props.reducedMotion ? 0 : 4} duration={props.reducedMotion ? 0 : 180}>
  <section
    class="composer-wrap relative z-[3] shrink-0 bg-transparent px-4 pb-3 pt-2 [.thread-view:not(.empty)_&]:pt-[22px] [.thread-view:not(.empty)_&]:before:pointer-events-none [.thread-view:not(.empty)_&]:before:absolute [.thread-view:not(.empty)_&]:before:inset-x-0 [.thread-view:not(.empty)_&]:before:top-0 [.thread-view:not(.empty)_&]:before:h-[22px] [.thread-view:not(.empty)_&]:before:bg-gradient-to-b [.thread-view:not(.empty)_&]:before:from-transparent [.thread-view:not(.empty)_&]:before:to-shell [.thread-view:not(.empty)_&]:before:content-['']"
    class:z-[15]={modelPickerOpen || sessionPickerOpen}
  >
  <div
    bind:this={composerElement}
    class="relative mx-auto grid min-h-[49px] w-[min(var(--content-width,720px),100%)] grid-cols-[auto_minmax(0,1fr)_auto] items-end gap-[7px] rounded-[18px] border border-line-strong bg-raised p-[7px] shadow-overlay backdrop-blur-overlay focus-within:border-focus-ring"
    class:expanded={expanded}
  >
    {#if props.images.length > 0 || props.attachmentError}
      <div class="col-span-full flex min-w-0 items-center gap-[7px] overflow-x-auto px-px pb-[3px] pt-px" aria-label="Attached images">
        {#each props.images as image (image.id)}
          <ContextMenu items={imageMenuItems(image)}>
            {#snippet children({ props: imageProps })}
              <div {...imageProps} class="relative size-11 shrink-0 overflow-visible rounded-[9px] border border-line-strong bg-panel-hover" role="group" title={image.name}>
                <button
                  type="button"
                  class="block size-full overflow-hidden rounded-lg border-0 bg-transparent p-0"
                  aria-label={`Preview ${image.name}`}
                  onclick={() => { imagePreview = { src: image.previewUrl, name: image.name }; }}
                ><img class="size-full object-cover" src={image.previewUrl} alt="" /></button>
                <button class="absolute -right-[5px] -top-[5px] grid size-[17px] place-items-center rounded-full border border-line-strong bg-floating p-0 text-text-soft hover:bg-raised-hover hover:text-text" type="button" aria-label={`Remove ${image.name}`} title={`Remove ${image.name}`} onclick={() => props.removeImage(image.id)}>
                  <IconX size={10} stroke={1.55} />
                </button>
              </div>
            {/snippet}
          </ContextMenu>
        {/each}
        {#if props.attachmentError}<span class="text-[11px] text-danger">{props.attachmentError}</span>{/if}
      </div>
    {/if}
    <input bind:this={imageInput} class="hidden" type="file" accept="image/*" multiple disabled={!canEdit()} onchange={handleImageSelection} />
    <button
      bind:this={attachButton}
      class="grid size-[30px] shrink-0 place-items-center rounded-full border-0 bg-transparent p-0 text-muted hover:bg-panel-hover hover:text-text-soft disabled:opacity-40"
      class:order-2={expanded}
      class:col-start-1={expanded}
      type="button"
      aria-label="Attach images"
      title="Attach images (you can also paste them)"
      disabled={!canEdit()}
      onclick={() => imageInput?.click()}
    >
      <IconPaperclip size={16} stroke={1.55} />
    </button>
    <div
      bind:this={promptEditor}
      class="prompt-editor h-auto w-full min-h-8 max-h-[161px] overflow-y-hidden border-0 bg-transparent py-[5px] pl-0 pr-1.5 text-sm leading-normal text-text outline-0 whitespace-pre-wrap break-anywhere aria-disabled:cursor-default aria-disabled:opacity-[0.62]"
      class:order-1={expanded}
      class:col-span-full={expanded}
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
        ? provider.supportsFollowups
          ? 'Send a follow-up…'
          : 'Agent is working…'
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
      <div id="composer-autocomplete" class="absolute bottom-[calc(100%+6px)] left-[38px] z-20 max-h-[min(280px,42vh)] w-[min(380px,calc(100%-52px))] overflow-y-auto rounded-overlay border border-line-strong bg-floating p-1 shadow-overlay backdrop-blur-overlay" role="listbox" aria-label={completionPrefix === '$' ? 'Skills' : completionPrefix === '/' ? 'Commands' : 'Workspace files'}>
        {#if completionPrefix !== '/' && completionStatus === 'loading'}
          <p class="m-0 p-[9px] text-[11px] text-muted">Loading…</p>
        {:else if completionPrefix !== '/' && completionStatus === 'error'}
          <p class="m-0 p-[9px] text-[11px] text-muted">Autocomplete unavailable.</p>
        {:else if completionResults.length === 0}
          <p class="m-0 p-[9px] text-[11px] text-muted">No matches.</p>
        {:else}
          {#each completionResults as entry, index (isCommand(entry) ? `/${entry.name}` : `${entry.kind}-${entry.path}`)}
            <button
              id={`composer-completion-${index}`}
              type="button"
              role="option"
              aria-selected={index === completionIndex}
              class="flex w-full min-w-0 items-center gap-2 rounded-md border-0 bg-transparent px-2 py-[7px] text-left font-[inherit] text-text-soft hover:bg-panel-active"
              class:bg-panel-active={index === completionIndex}
              onpointerenter={() => { completionIndex = index; }}
              onpointerdown={(event) => {
                event.preventDefault();
                applyCompletion(entry);
              }}
            >
              {#if isCommand(entry)}
                <span class="grid size-4 shrink-0 place-items-center text-muted"><IconSlash size={14} stroke={1.55} /></span>
                <span class="flex min-w-0 items-baseline gap-2">
                  <strong class="min-w-0 max-w-[55%] shrink grow basis-auto overflow-hidden text-ellipsis whitespace-nowrap text-xs font-semibold">/{entry.name}{#if entry.hint}<span class="ml-1 font-normal text-faint">{entry.hint}</span>{/if}</strong>
                  <small class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11px] leading-snug text-muted">{entry.description}</small>
                </span>
              {:else}
                {#if entry.kind === 'skill'}
                  <span class="grid size-4 shrink-0 place-items-center text-muted"><IconSparkles size={14} stroke={1.55} /></span>
                {:else}
                  {@const icon = entry.kind === 'folder' ? materialFolderIcon(entry.name, false) : materialFileIcon(entry.name)}
                  {#if icon}<img class="size-4 shrink-0 opacity-[0.64] [filter:var(--provider-filter)]" src={icon} alt="" />{/if}
                {/if}
                <span class="flex min-w-0 items-baseline gap-2">
                  <strong class="min-w-0 max-w-[55%] shrink grow basis-auto overflow-hidden text-ellipsis whitespace-nowrap text-xs font-semibold">{entry.name}</strong>
                  <small class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-[11px] leading-snug text-muted">{entry.description ?? entry.relativePath}</small>
                </span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    {/if}
    <div
      bind:this={composerFooter}
      class="flex items-center justify-end gap-[9px]"
      class:order-3={expanded}
      class:w-max={expanded}
      class:col-start-2={expanded}
      class:col-end-[-1]={expanded}
      class:justify-self-end={expanded}
    >
      <div class="flex max-w-[390px] items-center gap-[3px] overflow-visible whitespace-nowrap text-[11px] text-muted">
          <div class="relative min-w-0">
            {#if props.selectableProfiles.length > 0}
              <ModelPicker
                thread={props.thread}
                catalogs={props.catalogs}
                profiles={props.selectableProfiles}
                label={activeModelName()}
                open={modelPickerOpen}
                setOpen={(open) => { sessionPickerOpen = false; modelPickerOpen = open; }}
                choose={chooseModel}
                retryDiscovery={props.retryDiscovery}
              />
            {:else}
              <button class="flex h-7 max-w-[210px] items-center gap-1.5 rounded-[7px] border border-transparent bg-transparent px-[7px] text-[11px] font-medium text-muted hover:border-line hover:bg-panel hover:text-text-soft disabled:opacity-[0.62]" title="No authenticated providers" disabled>
                <img class:brand-color-icon={profile.iconMode === 'brand'} class="size-3.5 shrink-0 opacity-[0.62] [filter:var(--provider-filter)]" src={profile.icon} alt="" />
                <span class="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">No providers</span>
              </button>
            {/if}
          </div>
        {#if sessionOptionsAvailable}
          <SessionPicker
            {provider}
            open={sessionPickerOpen}
            setOpen={(open) => { modelPickerOpen = false; sessionPickerOpen = open; }}
            selectReasoning={props.selectReasoning}
            selectFastMode={props.selectFastMode}
            selectSessionOption={props.selectSessionOption}
          />
        {/if}
      </div>
      {#if status === 'running'}
        {#if provider.supportsFollowups}
          <button class="grid size-[30px] shrink-0 place-items-center rounded-full border border-line bg-transparent p-0 text-muted hover:border-line-strong hover:bg-panel-hover hover:text-text-soft" aria-label="Cancel turn" title="Cancel turn" onclick={props.cancel}><IconPlayerStop size={12} fill="currentColor" stroke={1.55} /></button>
          <button class="grid size-[30px] shrink-0 place-items-center rounded-full border-0 bg-accent p-0 text-accent-contrast hover:bg-accent-hover active:translate-y-px disabled:bg-panel-active disabled:text-faint" aria-label="Send follow-up" title="Send follow-up" disabled={props.gitBusy || (!hasPromptContent(props.thread.draft, props.thread.draftReferences) && props.images.length === 0) || !canSend() || hasMissingReferences()} onclick={props.send}>
            <IconArrowUp size={17} stroke={1.55} />
          </button>
        {:else}
          <button class="grid size-[30px] shrink-0 place-items-center rounded-full border-0 bg-accent p-0 text-accent-contrast hover:bg-accent-hover active:translate-y-px" aria-label="Cancel turn" title="Cancel turn" onclick={props.cancel}><IconPlayerStop size={13} fill="currentColor" stroke={1.55} /></button>
        {/if}
      {:else if status === 'error' || status === 'stopped'}
        <button class="grid size-[30px] shrink-0 place-items-center rounded-full border border-line-strong bg-panel-hover p-0 text-text-soft hover:bg-panel-active hover:text-text active:translate-y-px" aria-label="Reconnect provider" title="Reconnect provider" onclick={props.reconnect}><IconPlugConnected size={15} stroke={1.55} /></button>
      {:else}
        <button class="grid size-[30px] shrink-0 place-items-center rounded-full border-0 bg-accent p-0 text-accent-contrast hover:bg-accent-hover active:translate-y-px disabled:bg-panel-active disabled:text-faint" aria-label="Send prompt" title={hasMissingReferences() ? 'Remove missing references before sending' : props.gitBusy ? 'Wait for the Git operation to finish' : 'Send prompt'} disabled={props.gitBusy || (!hasPromptContent(props.thread.draft, props.thread.draftReferences) && props.images.length === 0) || !canSend() || hasMissingReferences()} onclick={props.send}>
          <IconArrowUp size={17} stroke={1.55} />
        </button>
      {/if}
    </div>
  </div>
  <div class="mx-auto flex min-h-7 w-[min(var(--content-width,720px),100%)] items-center justify-between gap-2.5 px-3.5 py-1 text-[11px] text-muted">
    <div class="flex min-w-0 flex-1 items-center justify-start gap-2">
      {#if props.currentBranch !== null}
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
      {/if}
    </div>
    <div class="flex items-center gap-2">
      {#if provider.quota?.totalTokens !== undefined}
        <span title="Tokens used by the last turn">{new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 }).format(provider.quota.totalTokens)} tokens</span>
      {/if}
      {#each provider.rateLimits ?? [] as limit (limit.id)}
        {#if limit.primary}
          <span title={limit.primary.resetsAt ? `${limit.name} resets ${new Date(limit.primary.resetsAt * 1000).toLocaleString()}` : limit.name}>{Math.max(0, Math.round(100 - limit.primary.usedPercent))}% {limit.name} left</span>
        {/if}
      {/each}
      <ContextMeter {provider} fresh={props.thread.messages.length === 0} />
    </div>
  </div>
  </section>
</MotionFly>

{#if imagePreview}
  <ImagePreview
    src={imagePreview.src}
    name={imagePreview.name}
    close={() => { imagePreview = undefined; }}
  />
{/if}

<svelte:window onblur={closeCompletion} />
