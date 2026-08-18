<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import Composer from './components/Composer.svelte';
  import DeleteThreadModal from './components/DeleteThreadModal.svelte';
  import FileViewer from './components/FileViewer.svelte';
  import PermissionModal from './components/PermissionModal.svelte';
  import ProjectExplorer from './components/ProjectExplorer.svelte';
  import QuestionComposer from './components/QuestionComposer.svelte';
  import SettingsPage from './components/SettingsPage.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Titlebar from './components/Titlebar.svelte';
  import Transcript from './components/Transcript.svelte';
  import { profileById, profiles } from './config/providers';
  import { preferredAllowOptionId } from './services/acp';
  import {
    getGitBranch,
    getInitialWorkingDirectory,
    loadWorkspace,
    openProjectPath,
    pickFolder,
    registerFrontend,
    revealProjectPath,
    stopAllHarnesses,
  } from './services/native';
  import { ProviderRuntime } from './services/provider-runtime';
  import { WorkspacePersistence } from './services/workspace-persistence';
  import type {
    ComposerImage,
    MessageImage,
    ModelOption,
    PermissionMode,
    PermissionRequest,
    ProjectState,
    ProviderModelCatalog,
    ThreadState,
  } from './types';
  import { loadComposerImages, MAX_COMPOSER_IMAGES } from './utils/attachments';
  import {
    LEFT_SIDEBAR_WIDTH_RANGE,
    RIGHT_SIDEBAR_WIDTH_RANGE,
    loadPermissionMode,
    loadSidebarWidths,
    savePermissionMode,
    saveSidebarWidth,
  } from './utils/app-settings';
  import { addMessage, nextTimestamp, titleFromPrompt } from './utils/messages';
  import { findReusableEmptyThread } from './utils/thread-state';
  import { buildThreadTitlePrompt, normalizeThreadTitle } from './utils/thread-title';
  import { timelineEntries } from './utils/timeline';
  import {
    activeProvider,
    compareSidebarThreads,
    createThread,
    folderName,
    threadStatus,
  } from './utils/threads';
  import { restoreWorkspace, workspaceSnapshot } from './utils/workspace';

  const appWindow = getCurrentWindow();
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const initialCatalogs = Object.fromEntries(
    profiles.map((profile) => [
      profile.id,
      { status: 'loading', models: [], reasoningOptions: [] } satisfies ProviderModelCatalog,
    ]),
  );

  let defaultWorkingFolder = $state('');
  let providerCatalogs = $state<Record<string, ProviderModelCatalog>>(initialCatalogs);
  let projects = $state<ProjectState[]>([]);
  let selectedProjectId = $state<string | null>(null);
  const firstThread = createThread('', null, providerCatalogs);
  let threads = $state<ThreadState[]>([firstThread]);
  let selectedThreadId = $state(firstThread.id);
  let threadPendingRemoval = $state<ThreadState>();
  let interactions = $state<Record<string, {
    threadId: string;
    profileId: string;
    request: PermissionRequest;
  }>>({});
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let projectExplorerOpen = $state(false);
  let projectExplorerCollapsed = $state(false);
  const savedSidebarWidths = loadSidebarWidths();
  let leftSidebarWidth = $state<number | null>(savedSidebarWidths.left);
  let rightSidebarWidth = $state<number | null>(savedSidebarWidths.right);
  let compactLayout = $state(window.matchMedia('(max-width: 880px)').matches);
  let settingsOpen = $state(false);
  let compactSessionRows = $state(false);
  const initialPermissionMode = loadPermissionMode();
  let permissionMode = $state<PermissionMode>(initialPermissionMode);
  let showSettled = $state(false);
  let workspaceDropdownOpen = $state(false);
  let windowMaximized = $state(false);
  let composerImagesByThread = $state<Record<string, ComposerImage[]>>({});
  let attachmentErrorsByThread = $state<Record<string, string>>({});
  let currentBranch = $state<string | null | undefined>(undefined);
  let fileHistory = $state<string[]>([]);
  let fileHistoryIndex = $state(-1);
  let fileRevision = $state(0);
  let fileViewerThreadId = $state('');
  let threadViewElement = $state<HTMLElement>();
  let closing = false;
  let branchLookup = 0;
  let stopSidebarResize: (() => void) | undefined;

  const sidebarWidthStyle = $derived([
    leftSidebarWidth === null ? '' : `--sidebar-expanded-width: ${leftSidebarWidth}px`,
    rightSidebarWidth === null ? '' : `--project-explorer-expanded-width: ${rightSidebarWidth}px`,
  ].filter(Boolean).join('; '));

  const persistence = new WorkspacePersistence();
  const providers = new ProviderRuntime(providerCatalogs, {
    permission: (value) => { interactions[interactionKey(value.threadId, value.profileId)] = value; },
    clearPermission: (threadId, profileId) => {
      for (const [key, interaction] of Object.entries(interactions)) {
        if (interaction.threadId === threadId && (!profileId || interaction.profileId === profileId)) {
          delete interactions[key];
        }
      }
    },
  });

  providers.setPermissionMode(initialPermissionMode);

  const selectedThread = $derived(threads.find((thread) => thread.id === selectedThreadId));
  const selectedTimelineEntries = $derived(selectedThread ? timelineEntries(selectedThread) : []);
  const selectedThreadEmpty = $derived(Boolean(
    selectedThread && selectedThread.messages.length === 0 && selectedThread.tools.length === 0,
  ));
  const selectedInteraction = $derived(
    selectedThread ? interactions[interactionKey(selectedThread.id, selectedThread.profileId)] : undefined,
  );
  const inboxThreads = $derived(
    threads.filter((thread) => !thread.settled).sort(compareSidebarThreads),
  );
  const settledThreads = $derived(
    threads.filter((thread) => thread.settled).sort((left, right) => right.updatedAt - left.updatedAt),
  );
  const activeProject = $derived(
    selectedProjectId ? projects.find((project) => project.id === selectedProjectId) ?? null : null,
  );
  const explorerRoot = $derived(selectedThread?.cwd ?? '');
  const explorerProjectName = $derived(
    selectedThread ? projectNameForThread(selectedThread) : 'Project',
  );
  const projectExplorerVisible = $derived(
    compactLayout ? projectExplorerOpen : !projectExplorerCollapsed,
  );
  const activeFilePath = $derived(fileHistoryIndex >= 0 ? fileHistory[fileHistoryIndex] ?? null : null);

  $effect(() => {
    if (selectedThreadId === fileViewerThreadId) return;
    fileViewerThreadId = selectedThreadId;
    fileHistory = [];
    fileHistoryIndex = -1;
    fileRevision = 0;
  });

  $effect(() => {
    persistence.queue(workspaceSnapshot(threads, selectedThreadId, projects, selectedProjectId));
  });

  $effect(() => {
    const cwd = selectedThread?.cwd;
    const status = selectedThread ? threadStatus(selectedThread) : undefined;
    void status;
    const lookup = ++branchLookup;
    currentBranch = undefined;
    if (!cwd) {
      currentBranch = null;
      return;
    }
    void getGitBranch(cwd)
      .then((branch) => {
        if (lookup === branchLookup) currentBranch = branch;
      })
      .catch(() => {
        if (lookup === branchLookup) currentBranch = null;
      });
  });

  onMount(() => {
    void initializeWorkspace();
    let disposed = false;
    let unlistenResize: (() => void) | undefined;
    let unlistenClose: (() => void) | undefined;

    const syncMaximizedState = async () => {
      const maximized = await appWindow.isMaximized();
      if (!disposed) {
        windowMaximized = maximized;
        compactLayout = window.matchMedia('(max-width: 880px)').matches;
      }
    };

    void syncMaximizedState();
    void appWindow.onResized(() => { void syncMaximizedState(); }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenResize = unlisten;
    });
    void appWindow.onCloseRequested((event) => {
      event.preventDefault();
      void closeApp();
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenClose = unlisten;
    });

    return () => {
      disposed = true;
      stopSidebarResize?.();
      unlistenResize?.();
      unlistenClose?.();
    };
  });

  function targetWorkspace() {
    const project = selectedProjectId
      ? projects.find((item) => item.id === selectedProjectId)
      : null;
    return {
      cwd: project?.path ?? defaultWorkingFolder,
      projectId: project?.id ?? null,
    };
  }

  function addThread() {
    addThreadForTarget(targetWorkspace());
  }

  function addThreadToProject(projectId: string) {
    const project = projects.find((item) => item.id === projectId);
    if (!project) return;
    workspaceDropdownOpen = false;
    selectedProjectId = project.id;
    addThreadForTarget({ cwd: project.path, projectId: project.id });
  }

  function addThreadForTarget(target: { cwd: string; projectId: string | null }) {
    const reusable = findReusableEmptyThread(
      threads,
      target,
      (threadId) => composerImages(threadId).length > 0,
    );
    if (reusable) {
      selectThread(reusable.id);
      return;
    }
    const thread = createThread(target.cwd, target.projectId, providerCatalogs);
    threads.unshift(thread);
    selectThread(thread.id);
  }

  function ensureProjectForPath(path: string) {
    const trimmed = path.trim();
    const existing = projects.find((project) => project.path === trimmed);
    if (existing) return existing;
    const project: ProjectState = {
      id: crypto.randomUUID(),
      name: folderName(trimmed) || 'Untitled project',
      path: trimmed,
      createdAt: Date.now(),
    };
    projects = [...projects, project];
    return project;
  }

  async function openAddProject() {
    workspaceDropdownOpen = false;
    try {
      const picked = await pickFolder();
      if (!picked) return;
      selectedProjectId = ensureProjectForPath(picked).id;
    } catch (error) {
      const thread = selectedThread ?? threads[0];
      if (thread) addMessage(thread, 'error', `Could not add folder: ${errorMessage(error)}`);
    }
  }

  function selectProject(projectId: string | null) {
    selectedProjectId = projectId;
    workspaceDropdownOpen = false;
  }

  function renameThread(threadId: string) {
    const thread = threads.find((item) => item.id === threadId);
    if (!thread) return;
    const title = window.prompt('Rename thread', thread.title)?.trim();
    if (!title || title === thread.title) return;
    thread.title = title;
    thread.updatedAt = Date.now();
  }

  function openThreadFolder(thread: ThreadState) {
    if (thread.cwd) void runPathAction(openProjectPath(thread.cwd, thread.cwd));
  }

  function openProjectFolder(project: ProjectState) {
    void runPathAction(openProjectPath(project.path, project.path));
  }

  function revealProjectFolder(project: ProjectState) {
    void runPathAction(revealProjectPath(project.path, project.path));
  }

  function removeProject(projectId: string) {
    projects = projects.filter((project) => project.id !== projectId);
    for (const thread of threads) {
      if (thread.projectId === projectId) thread.projectId = null;
    }
    if (selectedProjectId === projectId) selectedProjectId = null;
    workspaceDropdownOpen = false;
  }

  async function runPathAction(action: Promise<void>) {
    try {
      await action;
    } catch (error) {
      const thread = selectedThread ?? threads[0];
      if (thread) addMessage(thread, 'error', errorMessage(error));
    }
  }

  function toggleSettled(threadId: string) {
    const thread = threads.find((item) => item.id === threadId);
    if (!thread) return;
    thread.settled = !thread.settled;
    thread.updatedAt = Date.now();
  }

  function requestThreadRemoval(threadId: string) {
    threadPendingRemoval = threads.find((thread) => thread.id === threadId);
  }

  async function removeThread(threadId: string) {
    threadPendingRemoval = undefined;
    await providers.removeThread(threadId);
    delete composerImagesByThread[threadId];
    delete attachmentErrorsByThread[threadId];
    threads = threads.filter((thread) => thread.id !== threadId);
    for (const [key, interaction] of Object.entries(interactions)) {
      if (interaction.threadId === threadId) delete interactions[key];
    }
    if (threads.length === 0) {
      const target = targetWorkspace();
      threads = [createThread(target.cwd, target.projectId, providerCatalogs)];
    }
    if (!threads.some((thread) => thread.id === selectedThreadId)) {
      selectedThreadId = threads[0].id;
    }
  }

  function selectThread(threadId: string) {
    selectedThreadId = threadId;
    sidebarOpen = false;
    projectExplorerOpen = false;
  }

  function selectThreadFromKeyboard(event: KeyboardEvent, threadId: string) {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    selectThread(threadId);
  }

  function toggleSidebar() {
    if (window.matchMedia('(max-width: 880px)').matches) sidebarOpen = !sidebarOpen;
    else sidebarCollapsed = !sidebarCollapsed;
  }

  function toggleProjectExplorer() {
    if (compactLayout) projectExplorerOpen = !projectExplorerOpen;
    else projectExplorerCollapsed = !projectExplorerCollapsed;
  }

  function openFile(path: string) {
    if (path === activeFilePath) return;
    fileHistory = [...fileHistory.slice(0, fileHistoryIndex + 1), path];
    fileHistoryIndex = fileHistory.length - 1;
    fileRevision += 1;
    if (compactLayout) projectExplorerOpen = false;
  }

  function goBackInFileHistory() {
    if (fileHistoryIndex >= 0) fileHistoryIndex -= 1;
  }

  function goForwardInFileHistory() {
    if (fileHistoryIndex < fileHistory.length - 1) fileHistoryIndex += 1;
  }

  function closeFileViewer() {
    fileHistory = [];
    fileHistoryIndex = -1;
  }

  function projectFilesChanged(paths: string[]) {
    if (activeFilePath && paths.includes(activeFilePath)) fileRevision += 1;
  }

  function startSidebarResize(event: PointerEvent, side: 'left' | 'right') {
    if (compactLayout || event.button !== 0) return;
    event.preventDefault();
    stopSidebarResize?.();

    const shell = document.querySelector<HTMLElement>('.app-shell');
    const panelSelector = side === 'left' ? '.thread-sidebar' : '.project-explorer';
    const panel = shell?.querySelector<HTMLElement>(panelSelector);
    if (!shell || !panel) return;

    const startX = event.clientX;
    const startWidth = panel.getBoundingClientRect().width;
    const range = side === 'left' ? LEFT_SIDEBAR_WIDTH_RANGE : RIGHT_SIDEBAR_WIDTH_RANGE;
    document.body.classList.add('resizing-sidebar');

    const onPointerMove = (moveEvent: PointerEvent) => {
      const delta = side === 'left' ? moveEvent.clientX - startX : startX - moveEvent.clientX;
      const width = Math.round(Math.min(range.max, Math.max(range.min, startWidth + delta)));
      if (side === 'left') leftSidebarWidth = width;
      else rightSidebarWidth = width;
    };
    const finish = () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', finish);
      window.removeEventListener('pointercancel', finish);
      document.body.classList.remove('resizing-sidebar');
      const width = side === 'left' ? leftSidebarWidth : rightSidebarWidth;
      if (width !== null) saveSidebarWidth(side, width);
      stopSidebarResize = undefined;
    };

    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', finish);
    window.addEventListener('pointercancel', finish);
    stopSidebarResize = finish;
  }

  function openSettings() {
    settingsOpen = true;
    sidebarCollapsed = false;
    sidebarOpen = false;
    workspaceDropdownOpen = false;
  }

  function closeSettings() {
    settingsOpen = false;
    sidebarOpen = false;
  }

  async function initializeWorkspace() {
    try {
      await registerFrontend();
      defaultWorkingFolder = await getInitialWorkingDirectory();
      const savedWorkspace = await loadWorkspace();
      if (savedWorkspace !== null) {
        const restored = restoreWorkspace(savedWorkspace, defaultWorkingFolder, providerCatalogs);
        if (!restored) throw new Error('The saved thread file has an unsupported or invalid format.');
        threads = restored.threads;
        selectedThreadId = restored.selectedThreadId;
        projects = restored.projects;
        selectedProjectId = restored.selectedProjectId;
      } else {
        for (const thread of threads) {
          if (!thread.cwd) thread.cwd = defaultWorkingFolder;
        }
      }
      persistence.setReady();
      await providers.discoverAll(defaultWorkingFolder, threads);
    } catch (error) {
      const thread = selectedThread ?? threads[0];
      if (!thread) return;
      const provider = activeProvider(thread);
      provider.connectionStatus = 'error';
      provider.error = errorMessage(error);
      provider.errorDetails = { scope: 'connection', message: provider.error };
      addMessage(thread, 'error', provider.error);
    }
  }

  function composerImages(threadId: string) {
    return composerImagesByThread[threadId] ?? [];
  }

  async function attachImages(files: File[], threadId: string) {
    try {
      const selection = await loadComposerImages(files, composerImages(threadId).length);
      attachmentErrorsByThread[threadId] = selection.error;
      if (selection.images.length === 0 || !threads.some((thread) => thread.id === threadId)) return;
      composerImagesByThread[threadId] = [...composerImages(threadId), ...selection.images]
        .slice(0, MAX_COMPOSER_IMAGES);
    } catch (error) {
      attachmentErrorsByThread[threadId] = errorMessage(error);
    }
  }

  function removeComposerImage(threadId: string, imageId: string) {
    composerImagesByThread[threadId] = composerImages(threadId).filter((image) => image.id !== imageId);
    attachmentErrorsByThread[threadId] = '';
  }

  async function sendPrompt() {
    const thread = selectedThread;
    if (!thread) return;
    const provider = activeProvider(thread);
    if (provider.turnStatus !== 'idle') return;
    if (provider.connectionStatus !== 'disconnected' && provider.connectionStatus !== 'ready') return;
    const text = thread.draft.trim();
    const images = composerImages(thread.id);
    if (!text && images.length === 0) return;
    if (providerCatalogs[thread.profileId]?.status !== 'ready' || !provider.selectedModelId) return;

    const profileId = thread.profileId;
    const isFirstPrompt = !thread.messages.some((message) => message.role === 'user');
    const centeredComposerTop = selectedThreadEmpty && !reducedMotion
      ? threadViewElement?.querySelector<HTMLElement>('.composer-wrap')?.getBoundingClientRect().top
      : undefined;
    const selectedModelId = provider.selectedModelId;
    const messageImages: MessageImage[] = images.map(({ data, mimeType, name }) => ({ data, mimeType, name }));
    const promptImages = messageImages.map(({ data, mimeType }) => ({ type: 'image' as const, data, mimeType }));
    thread.draft = '';
    composerImagesByThread[thread.id] = [];
    attachmentErrorsByThread[thread.id] = '';
    provider.error = undefined;
    provider.errorDetails = undefined;
    if (isFirstPrompt && !text) thread.title = 'Image prompt';
    addMessage(thread, 'user', text, messageImages);
    if (centeredComposerTop !== undefined) animateComposerToTranscript(centeredComposerTop);
    providers.startTurn(thread.id, profileId);

    let connection = providers.connection(thread.id, profileId);
    if (provider.connectionStatus === 'disconnected') connection = await providers.connect(thread, profileId);
    if (
      thread.profileId !== profileId ||
      thread.providers[profileId].connectionStatus !== 'ready' ||
      thread.providers[profileId].turnStatus !== 'idle' ||
      !connection
    ) return;

    const turnCompletion = connection.prompt(text, promptImages);
    if (isFirstPrompt && text) {
      void generateThreadTitle(thread, connection, text, selectedModelId);
    }
    try {
      await turnCompletion;
    } catch {
      // The provider callback already added a contextual error to the timeline.
    }
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

  async function generateThreadTitle(
    thread: ThreadState,
    connection: import('./services/acp').AcpConnection,
    request: string,
    selectedModelId: string,
  ) {
    let title: string | undefined;
    try {
      title = normalizeThreadTitle(
        await connection.generateTitle(thread.cwd, buildThreadTitlePrompt(request), selectedModelId),
      );
    } catch {
      // Use the local fallback when a provider cannot create a quiet title session.
    }
    if (!threads.some((item) => item.id === thread.id)) return;
    thread.title = title ?? titleFromPrompt(request);
    thread.updatedAt = nextTimestamp(thread);
  }

  async function cancelPrompt() {
    if (!selectedThread) return;
    await providers.cancel(selectedThread);
    addMessage(selectedThread, 'notice', 'The active turn was cancelled.');
  }

  function activateProvider(profileId: string) {
    if (selectedThread) providers.activate(selectedThread, profileId);
  }

  async function selectModel(profileId: string, model: ModelOption) {
    if (!selectedThread) return;
    const currentModelId = selectedThread.providers[profileId].selectedModelId;
    if (model.id === currentModelId) return;
    await providers.selectModel(selectedThread, profileId, model.id);
  }

  async function selectReasoning(reasoningId: string) {
    if (selectedThread) await providers.selectReasoning(selectedThread, reasoningId);
  }

  async function selectFastMode(enabled: boolean) {
    if (selectedThread) await providers.selectFastMode(selectedThread, enabled);
  }

  function reconnect() {
    if (selectedThread) void providers.connect(selectedThread, selectedThread.profileId);
  }

  function setPermissionMode(mode: PermissionMode) {
    permissionMode = mode;
    providers.setPermissionMode(mode);
    savePermissionMode(mode);
    if (mode !== 'full') return;
    for (const [key, interaction] of Object.entries(interactions)) {
      if (interaction.request.type !== 'permission') continue;
      const optionId = preferredAllowOptionId(interaction.request);
      if (!optionId) continue;
      delete interactions[key];
      providers.answerPermission(
        interaction.threadId,
        interaction.profileId,
        interaction.request.requestId,
        optionId,
      );
    }
  }

  function answerPermission(optionId?: string) {
    const active = selectedInteraction;
    if (!active) return;
    delete interactions[interactionKey(active.threadId, active.profileId)];
    providers.answerPermission(
      active.threadId,
      active.profileId,
      active.request.requestId,
      optionId,
    );
  }

  async function closeApp() {
    if (closing) return;
    closing = true;
    try {
      await persistence.flush();
      await stopAllHarnesses();
      await appWindow.destroy();
    } catch (error) {
      closing = false;
      const thread = selectedThread ?? threads[0];
      if (thread) addMessage(thread, 'error', `Could not save threads before closing: ${errorMessage(error)}`);
    }
  }

  function projectNameForThread(thread: ThreadState) {
    const project = thread.projectId ? projects.find((item) => item.id === thread.projectId) : null;
    return project?.name ?? folderName(thread.cwd) ?? 'No project';
  }

  function interactionKey(threadId: string, profileId: string) {
    return `${threadId}:${profileId}`;
  }

  function errorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<svelte:head><title>{settingsOpen ? 'Settings' : (selectedThread?.title ?? 'LoopCode')} | LoopCode</title></svelte:head>

<div
  class:maximized={windowMaximized}
  class:sidebar-collapsed={sidebarCollapsed}
  class:project-explorer-collapsed={!explorerRoot || (!compactLayout && projectExplorerCollapsed)}
  class:compact-session-rows={compactSessionRows}
  class="app-shell"
  style={sidebarWidthStyle}
>
  <Titlebar
    {settingsOpen}
    {selectedThread}
    {windowMaximized}
    {reducedMotion}
    {toggleSidebar}
    {addThread}
    {closeApp}
    minimize={() => { void appWindow.minimize(); }}
    toggleMaximize={() => { void appWindow.toggleMaximize(); }}
  />
  <Sidebar
      open={sidebarOpen}
      {settingsOpen}
      compactMotion={reducedMotion}
      {defaultWorkingFolder}
      {projects}
      {selectedProjectId}
      {activeProject}
      {workspaceDropdownOpen}
      {inboxThreads}
      {settledThreads}
      {selectedThreadId}
      {showSettled}
      setWorkspaceDropdownOpen={(open) => { workspaceDropdownOpen = open; }}
      {selectProject}
      addProject={() => { void openAddProject(); }}
      {addThread}
      {addThreadToProject}
      {selectThread}
      {selectThreadFromKeyboard}
      {toggleSettled}
      {renameThread}
      {openThreadFolder}
      removeThread={requestThreadRemoval}
      {openProjectFolder}
      {revealProjectFolder}
      {removeProject}
      setShowSettled={(show) => { showSettled = show; }}
      {openSettings}
      {closeSettings}
    startResize={(event) => startSidebarResize(event, 'left')}
  />
  <div class="workspace-grid">
    <main class="conversation">
      <div class="conversation-primary">
        {#if settingsOpen}
          <SettingsPage
            {compactSessionRows}
            {permissionMode}
            {reducedMotion}
            setCompactSessionRows={(value) => { compactSessionRows = value; }}
            {setPermissionMode}
          />
        {:else if selectedThread}
          {#if activeFilePath}
            <FileViewer
              path={activeFilePath}
              projectRoot={explorerRoot}
              revision={fileRevision}
              canGoForward={fileHistoryIndex < fileHistory.length - 1}
              back={goBackInFileHistory}
              forward={goForwardInFileHistory}
              close={closeFileViewer}
            />
          {:else}
            {#if fileHistory.length > 0}
              <div class="file-viewer-resume">
                <button type="button" class="settings-action" onclick={goForwardInFileHistory}>
                  Forward to file
                </button>
              </div>
            {/if}
            <div bind:this={threadViewElement} class:empty={selectedThreadEmpty} class="thread-view">
              {#if selectedThreadEmpty}
                <h1 class="empty-thread-heading">
                  What should we build in {projectNameForThread(selectedThread)}?
                </h1>
              {:else}
                {#key selectedThread.id}
                  <Transcript thread={selectedThread} entries={selectedTimelineEntries} {reducedMotion} />
                {/key}
              {/if}
              {#if selectedInteraction?.request.type === 'question'}
                <QuestionComposer
                  request={selectedInteraction.request}
                  {reducedMotion}
                  answer={(optionId) => answerPermission(optionId)}
                  dismiss={() => answerPermission()}
                />
              {:else}
                <Composer
                  thread={selectedThread}
                  catalogs={providerCatalogs}
                  images={composerImages(selectedThread.id)}
                  attachmentError={attachmentErrorsByThread[selectedThread.id]}
                  projectName={projectNameForThread(selectedThread)}
                  {currentBranch}
                  {reducedMotion}
                  attachImages={(files) => { void attachImages(files, selectedThread.id); }}
                  removeImage={(imageId) => removeComposerImage(selectedThread.id, imageId)}
                  send={() => { void sendPrompt(); }}
                  cancel={() => { void cancelPrompt(); }}
                  {reconnect}
                  selectModel={(profileId, model) => { void selectModel(profileId, model); }}
                  selectReasoning={(reasoningId) => { void selectReasoning(reasoningId); }}
                  selectFastMode={(enabled) => { void selectFastMode(enabled); }}
                  {activateProvider}
                  retryDiscovery={(profileId) => {
                    void providers.discover(profileById(profileId), defaultWorkingFolder, threads);
                  }}
                />
              {/if}
            </div>
          {/if}
        {/if}
      </div>
    </main>
    {#if explorerRoot}
      {#key explorerRoot}
        <ProjectExplorer
          open={projectExplorerOpen}
          visible={projectExplorerVisible}
          projectRoot={explorerRoot}
          projectName={explorerProjectName}
          {activeFilePath}
          toggle={toggleProjectExplorer}
          {openFile}
          filesChanged={projectFilesChanged}
          startResize={(event) => startSidebarResize(event, 'right')}
        />
      {/key}
    {/if}
  </div>
</div>

<svelte:window onkeydown={(event) => {
  if (event.key === 'Escape' && workspaceDropdownOpen) workspaceDropdownOpen = false;
}} />

{#if threadPendingRemoval}
  <DeleteThreadModal
    title={threadPendingRemoval.title}
    {reducedMotion}
    cancel={() => { threadPendingRemoval = undefined; }}
    confirm={() => { void removeThread(threadPendingRemoval!.id); }}
  />
{/if}

{#if selectedInteraction?.request.type === 'permission'}
  <PermissionModal
    request={selectedInteraction.request}
    {reducedMotion}
    answer={(optionId) => answerPermission(optionId)}
    decline={() => answerPermission()}
  />
{/if}
