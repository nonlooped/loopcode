<script lang="ts">
  import { onDestroy, tick, type Snippet } from 'svelte';

  import appIcon from '../../assets/loopcode-mark.png';

  import Composer from './Composer.svelte';
  import FileViewer from './FileViewer.svelte';
  import ProjectExplorer from './ProjectExplorer.svelte';
  import QuestionComposer from './QuestionComposer.svelte';
  import TerminalDrawer from './TerminalDrawer.svelte';
  import Transcript from './Transcript.svelte';
  import { createGitWorktree, getGitBranch, listGitBranches, switchGitBranch } from '../services/native';
  import type { ProviderRuntime } from '../services/provider-runtime';
  import type { Workspace } from '../services/workspace';
  import type {
    ComposerImage,
    HarnessProfile,
    MessageImage,
    ModelOption,
    PermissionRequest,
    ProviderModelCatalog,
    QuestionAnswer,
    ThreadState,
  } from '../types';
  import type { AppPreferences } from '../utils/app-settings';
  import { promptParts, promptText } from '../utils/prompt-content';
  import { timelineEntries } from '../utils/timeline';
  import { threadReservesCheckout, threadStatus } from '../utils/threads';

  interface Props {
    settingsOpen: boolean;
    settings: Snippet;
    selectedThread?: ThreadState;
    selectedThreadId: string;
    threads: ThreadState[];
    workspace: Workspace;
    providers: ProviderRuntime;
    profiles: HarnessProfile[];
    selectableProfiles: HarnessProfile[];
    catalogs: Record<string, ProviderModelCatalog>;
    selectedInteraction?: { request: PermissionRequest };
    composerImages: ComposerImage[];
    attachmentError?: string;
    terminalThreads: ThreadState[];
    terminalThreadIds: string[];
    terminalVisible: boolean;
    terminalHeight: number;
    preferences: AppPreferences;
    reducedMotion: boolean;
    compactLayout: boolean;
    projectExplorerOpen: boolean;
    setProjectExplorerOpen: (open: boolean) => void;
    projectExplorerCollapsed: boolean;
    defaultWorkingFolder: string;
    attachImages: (files: File[]) => void;
    removeImage: (imageId: string) => void;
    clearAttachments: () => void;
    answerQuestion: (answer: QuestionAnswer) => void;
    dismissQuestion: () => void;
    cancelPrompt: () => Promise<void>;
    closeTerminal: () => void;
    terminalExited: (threadId: string) => void;
    startTerminalResize: (event: PointerEvent) => void;
    resizeTerminalBy: (delta: number) => void;
    startSidebarResize: (event: PointerEvent) => void;
    setGitBusy: (busy: boolean) => void;
  }

  let {
    settingsOpen,
    settings,
    selectedThread,
    selectedThreadId,
    threads,
    workspace,
    providers,
    profiles,
    selectableProfiles,
    catalogs,
    selectedInteraction,
    composerImages,
    attachmentError,
    terminalThreads,
    terminalThreadIds,
    terminalVisible,
    terminalHeight,
    preferences,
    reducedMotion,
    compactLayout,
    projectExplorerOpen,
    setProjectExplorerOpen,
    projectExplorerCollapsed = $bindable(),
    defaultWorkingFolder,
    attachImages,
    removeImage,
    clearAttachments,
    answerQuestion,
    dismissQuestion,
    cancelPrompt,
    closeTerminal,
    terminalExited,
    startTerminalResize,
    resizeTerminalBy,
    startSidebarResize,
    setGitBusy,
  }: Props = $props();

  let currentBranch = $state<string | null | undefined>(undefined);
  let gitBranches = $state<string[] | null | undefined>(undefined);
  let gitOperationThreadId = $state<string>();
  let fileHistory = $state<string[]>([]);
  let fileHistoryIndex = $state(-1);
  let fileRevision = $state(0);
  let composerCompletionRevision = $state(0);
  let fileViewerThreadId = $state('');
  let threadViewElement = $state<HTMLElement>();
  let completionRefreshTimer: number | undefined;
  let branchLookup = 0;

  const selectedTimelineEntries = $derived(selectedThread ? timelineEntries(selectedThread) : []);
  const selectedThreadEmpty = $derived(Boolean(
    selectedThread && selectedThread.messages.length === 0 && selectedThread.tools.length === 0,
  ));
  const selectedThreadIsWorktree = $derived(selectedThread?.managedWorktree ?? false);
  const selectedThreadHasTerminal = $derived(Boolean(
    selectedThread && terminalThreadIds.includes(selectedThread.id),
  ));
  const selectedThreadHasProvider = $derived(Boolean(
    selectedThread && Object.values(selectedThread.providers).some((provider) =>
      provider.connectionStatus === 'connecting' || provider.connectionStatus === 'ready'),
  ));
  const explorerRoot = $derived(selectedThread?.cwd ?? '');
  const explorerProjectName = $derived(
    selectedThread ? workspace.projectNameForThread(selectedThread) : 'Project',
  );
  const projectExplorerVisible = $derived(
    !settingsOpen && (compactLayout ? projectExplorerOpen : !projectExplorerCollapsed),
  );
  const activeFilePath = $derived(fileHistoryIndex >= 0 ? fileHistory[fileHistoryIndex] ?? null : null);

  $effect(() => {
    if (selectedThreadId === fileViewerThreadId) return;
    fileViewerThreadId = selectedThreadId;
    setProjectExplorerOpen(false);
    fileHistory = [];
    fileHistoryIndex = -1;
    fileRevision = 0;
  });

  $effect(() => {
    const cwd = selectedThread?.cwd;
    const status = selectedThread ? threadStatus(selectedThread) : undefined;
    void status;
    const lookup = ++branchLookup;
    currentBranch = undefined;
    gitBranches = undefined;
    if (!cwd) {
      currentBranch = null;
      gitBranches = null;
      return;
    }
    void Promise.all([getGitBranch(cwd), listGitBranches(cwd)])
      .then(([branch, branches]) => {
        if (lookup !== branchLookup) return;
        currentBranch = branch;
        gitBranches = branches;
      })
      .catch(() => {
        if (lookup !== branchLookup) return;
        currentBranch = null;
        gitBranches = null;
      });
  });

  $effect(() => {
    setGitBusy(gitOperationThreadId !== undefined);
  });

  onDestroy(() => {
    window.clearTimeout(completionRefreshTimer);
    setGitBusy(false);
  });

  function editableGitThread() {
    if (gitOperationThreadId) throw new Error('Wait for the current Git operation to finish');
    const thread = selectedThread;
    if (!thread || thread.messages.length > 0 || thread.tools.length > 0) {
      throw new Error('Git can only be changed before the first prompt');
    }
    if (terminalThreadIds.includes(thread.id)) {
      throw new Error('Exit the thread terminal before changing Git');
    }
    if (Object.values(thread.providers).some((provider) =>
      provider.connectionStatus === 'connecting' || provider.connectionStatus === 'ready')) {
      throw new Error('Git cannot change after the provider connects');
    }
    return thread;
  }

  async function switchThreadGitBranch(branch: string) {
    const thread = editableGitThread();
    const cwd = thread.cwd;
    const terminalThread = threads.find((candidate) =>
      candidate.cwd === cwd && terminalThreadIds.includes(candidate.id));
    if (terminalThread) throw new Error(`Exit the terminal for ${terminalThread.title} before changing this checkout`);
    const establishedThread = threads.find((candidate) =>
      candidate.id !== thread.id
      && candidate.cwd === cwd
      && threadReservesCheckout(candidate));
    if (establishedThread) {
      throw new Error(`${establishedThread.title} already uses this checkout`);
    }
    gitOperationThreadId = thread.id;
    try {
      await switchGitBranch(cwd, branch);
      if (thread.cwd !== cwd) throw new Error('The thread working folder changed during the Git operation');
      if (selectedThread?.id === thread.id) currentBranch = branch;
      composerCompletionRevision += 1;
      fileRevision += 1;
    } finally {
      if (gitOperationThreadId === thread.id) gitOperationThreadId = undefined;
    }
  }

  async function createThreadGitWorktree(baseBranch: string, branch: string) {
    const thread = editableGitThread();
    const cwd = thread.cwd;
    gitOperationThreadId = thread.id;
    try {
      const worktree = await createGitWorktree(cwd, baseBranch, branch);
      if (thread.cwd !== cwd || thread.messages.length > 0 || thread.tools.length > 0) {
        throw new Error(`The thread changed while the worktree was created at ${worktree.path}`);
      }
      if (!workspace.setThreadWorktree(thread.id, worktree.path)) {
        throw new Error(`Could not move the thread to the worktree at ${worktree.path}`);
      }
      if (selectedThread?.id === thread.id) {
        currentBranch = worktree.branch;
        gitBranches = [...new Set([...(gitBranches ?? []), worktree.branch])].sort();
        fileHistory = [];
        fileHistoryIndex = -1;
        fileRevision = 0;
        composerCompletionRevision += 1;
      }
    } finally {
      if (gitOperationThreadId === thread.id) gitOperationThreadId = undefined;
    }
  }

  function openFile(path: string) {
    if (path === activeFilePath) return;
    fileHistory = [...fileHistory.slice(0, fileHistoryIndex + 1), path];
    fileHistoryIndex = fileHistory.length - 1;
    fileRevision += 1;
    if (compactLayout) setProjectExplorerOpen(false);
  }

  function projectFilesChanged(paths: string[]) {
    if (activeFilePath && paths.includes(activeFilePath)) fileRevision += 1;
    window.clearTimeout(completionRefreshTimer);
    completionRefreshTimer = window.setTimeout(() => { composerCompletionRevision += 1; }, 160);
  }

  async function sendPrompt() {
    const thread = selectedThread;
    if (!thread || gitOperationThreadId) return;
    const content = promptParts(thread.draft, thread.draftReferences);
    const text = promptText(content).trim();
    const referencedContent = thread.draftReferences.length > 0 ? content : undefined;
    const images: MessageImage[] = composerImages.map(({ data, mimeType, name }) => ({ data, mimeType, name }));
    const centeredComposerTop = selectedThreadEmpty && !reducedMotion
      ? threadViewElement?.querySelector<HTMLElement>('.composer-wrap')?.getBoundingClientRect().top
      : undefined;
    const turn = providers.runTurn(thread, text, images, referencedContent);
    if (!turn) return;

    thread.draft = '';
    thread.draftReferences = [];
    clearAttachments();
    if (centeredComposerTop !== undefined) animateComposerToTranscript(centeredComposerTop);
    await turn;
  }

  function resendPrompt(text: string) {
    const thread = selectedThread;
    if (!thread || gitOperationThreadId) return;
    void providers.runTurn(thread, text);
  }

  function animateComposerToTranscript(previousTop: number) {
    void tick().then(() => {
      const composer = threadViewElement?.querySelector<HTMLElement>('.composer-wrap');
      if (!composer) return;
      const offset = previousTop - composer.getBoundingClientRect().top;
      if (Math.abs(offset) < 1) return;
      composer.animate(
        [{ transform: `translateY(${offset}px)` }, { transform: 'translateY(0)' }],
        { duration: 240, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' },
      );
    });
  }

  function activateProvider(profileId: string) {
    if (selectedThread) providers.activate(selectedThread, profileId);
  }

  async function selectModel(profileId: string, model: ModelOption) {
    if (!selectedThread) return;
    const currentModelId = selectedThread.providers[profileId].selectedModelId;
    if (model.id !== currentModelId) await providers.selectModel(selectedThread, profileId, model.id);
  }

  function toggleProjectExplorer() {
    if (compactLayout) setProjectExplorerOpen(!projectExplorerOpen);
    else projectExplorerCollapsed = !projectExplorerCollapsed;
  }

</script>

{#if compactLayout && projectExplorerOpen}
  <button type="button" class="compact-drawer-backdrop" tabindex="-1" aria-label="Close project explorer" onclick={() => setProjectExplorerOpen(false)}></button>
{/if}

<div class="workspace-grid">
  <main class="conversation">
    <div class="conversation-primary">
      {#if settingsOpen}
        {@render settings()}
      {:else if selectedThread}
        {#if activeFilePath}
          <FileViewer
            path={activeFilePath}
            projectRoot={explorerRoot}
            revision={fileRevision}
            canGoForward={fileHistoryIndex < fileHistory.length - 1}
            back={() => { if (fileHistoryIndex >= 0) fileHistoryIndex -= 1; }}
            forward={() => { if (fileHistoryIndex < fileHistory.length - 1) fileHistoryIndex += 1; }}
            close={() => { fileHistory = []; fileHistoryIndex = -1; }}
          />
        {:else}
          {#if fileHistory.length > 0}
            <div class="file-viewer-resume">
              <button type="button" class="settings-action" onclick={() => { fileHistoryIndex += 1; }}>
                Return to file
              </button>
            </div>
          {/if}
          <div bind:this={threadViewElement} class:empty={selectedThreadEmpty} class="thread-view">
            {#if selectedThreadEmpty}
              <img class="empty-thread-logo" src={appIcon} alt="" aria-hidden="true" />
              <h1 class="empty-thread-heading">
                What should we build in {workspace.projectNameForThread(selectedThread)}?
              </h1>
            {:else}
              {#key selectedThread.id}
                <Transcript
                  thread={selectedThread}
                  entries={selectedTimelineEntries}
                  {profiles}
                  {reducedMotion}
                  sendShortcut={preferences.sendShortcut}
                  {openFile}
                  {resendPrompt}
                />
              {/key}
            {/if}
            {#if selectedInteraction?.request.type === 'question'}
              {#key selectedInteraction.request}
                <QuestionComposer
                  request={selectedInteraction.request}
                  {reducedMotion}
                  answer={answerQuestion}
                  dismiss={dismissQuestion}
                />
              {/key}
            {:else}
              <Composer
                thread={selectedThread}
                {catalogs}
                {profiles}
                {selectableProfiles}
                images={composerImages}
                {attachmentError}
                completionRevision={composerCompletionRevision}
                {currentBranch}
                {gitBranches}
                gitWorktree={selectedThreadIsWorktree}
                gitEditable={selectedThreadEmpty && !selectedThreadHasTerminal && !selectedThreadHasProvider}
                gitLockReason={!selectedThreadEmpty
                  ? 'Git is fixed after the first prompt'
                  : selectedThreadHasTerminal
                    ? 'Exit the terminal before changing Git'
                    : selectedThreadHasProvider
                      ? 'Git is fixed after the provider connects'
                      : undefined}
                gitBusy={gitOperationThreadId !== undefined}
                {reducedMotion}
                sendShortcut={preferences.sendShortcut}
                spellcheck={preferences.composerSpellcheck}
                {attachImages}
                {removeImage}
                send={() => { void sendPrompt(); }}
                cancel={() => { void cancelPrompt(); }}
                reconnect={() => { void providers.connect(selectedThread, selectedThread.profileId); }}
                selectModel={(profileId, model) => { void selectModel(profileId, model); }}
                selectReasoning={(reasoningId) => { void providers.selectReasoning(selectedThread, reasoningId); }}
                selectFastMode={(enabled) => { void providers.selectFastMode(selectedThread, enabled); }}
                {activateProvider}
                retryDiscovery={(profileId) => {
                  const profile = profiles.find((candidate) => candidate.id === profileId);
                  if (profile) void providers.discover(profile, defaultWorkingFolder, threads);
                }}
                switchGitBranch={switchThreadGitBranch}
                createGitWorktree={createThreadGitWorktree}
              />
            {/if}
          </div>
        {/if}
      {/if}
    </div>
    {#if terminalThreads.length > 0}
      <TerminalDrawer
        threads={terminalThreads}
        {selectedThreadId}
        open={terminalVisible}
        height={terminalHeight}
        fontSize={preferences.terminalFontSize}
        scrollback={preferences.terminalScrollback}
        {reducedMotion}
        close={closeTerminal}
        {terminalExited}
        startResize={startTerminalResize}
        resizeBy={resizeTerminalBy}
      />
    {/if}
  </main>
  {#if explorerRoot}
    {#key explorerRoot}
      <ProjectExplorer
        open={projectExplorerOpen && !settingsOpen}
        visible={projectExplorerVisible}
        projectRoot={explorerRoot}
        projectName={explorerProjectName}
        {activeFilePath}
        {currentBranch}
        branches={gitBranches}
        toggle={toggleProjectExplorer}
        {openFile}
        filesChanged={projectFilesChanged}
        startResize={startSidebarResize}
      />
    {/key}
  {/if}
</div>
