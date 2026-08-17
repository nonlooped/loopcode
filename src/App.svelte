<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import Composer from './components/Composer.svelte';
  import PermissionModal from './components/PermissionModal.svelte';
  import SettingsPage from './components/SettingsPage.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Titlebar from './components/Titlebar.svelte';
  import Transcript from './components/Transcript.svelte';
  import { profileById, profiles } from './config/providers';
  import {
    getGitBranch,
    getInitialWorkingDirectory,
    loadWorkspace,
    pickFolder,
    stopAllHarnesses,
  } from './services/native';
  import { ProviderRuntime } from './services/provider-runtime';
  import { WorkspacePersistence } from './services/workspace-persistence';
  import type {
    ComposerImage,
    MessageImage,
    ModelOption,
    PermissionRequest,
    ProjectState,
    ProviderModelCatalog,
    ThreadState,
  } from './types';
  import { loadComposerImages, MAX_COMPOSER_IMAGES } from './utils/attachments';
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
  let permission = $state<{
    threadId: string;
    profileId: string;
    request: PermissionRequest;
  } | null>(null);
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let settingsOpen = $state(false);
  let compactSessionRows = $state(false);
  let showSettled = $state(false);
  let workspaceDropdownOpen = $state(false);
  let windowMaximized = $state(false);
  let composerImagesByThread = $state<Record<string, ComposerImage[]>>({});
  let attachmentErrorsByThread = $state<Record<string, string>>({});
  let currentBranch = $state<string | null | undefined>(undefined);
  let closing = false;
  let allowWindowClose = false;
  let branchLookup = 0;

  const persistence = new WorkspacePersistence();
  const providers = new ProviderRuntime(providerCatalogs, {
    permission: (value) => { permission = value; },
    clearPermission: (threadId, profileId) => {
      if (
        permission?.threadId === threadId &&
        (!profileId || permission.profileId === profileId)
      ) {
        permission = null;
      }
    },
  });

  const selectedThread = $derived(threads.find((thread) => thread.id === selectedThreadId));
  const selectedTimelineEntries = $derived(selectedThread ? timelineEntries(selectedThread) : []);
  const inboxThreads = $derived(
    threads.filter((thread) => !thread.settled).sort(compareSidebarThreads),
  );
  const settledThreads = $derived(
    threads.filter((thread) => thread.settled).sort((left, right) => right.updatedAt - left.updatedAt),
  );
  const activeProject = $derived(
    selectedProjectId ? projects.find((project) => project.id === selectedProjectId) ?? null : null,
  );

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
      if (!disposed) windowMaximized = maximized;
    };

    void syncMaximizedState();
    void appWindow.onResized(() => { void syncMaximizedState(); }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenResize = unlisten;
    });
    void appWindow.onCloseRequested((event) => {
      if (allowWindowClose) return;
      event.preventDefault();
      void closeApp();
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenClose = unlisten;
    });

    const stopChildren = () => { void stopAllHarnesses(); };
    window.addEventListener('beforeunload', stopChildren);
    return () => {
      disposed = true;
      unlistenResize?.();
      unlistenClose?.();
      window.removeEventListener('beforeunload', stopChildren);
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
    const target = targetWorkspace();
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

  function toggleSettled(threadId: string) {
    const thread = threads.find((item) => item.id === threadId);
    if (!thread) return;
    thread.settled = !thread.settled;
    thread.updatedAt = Date.now();
  }

  async function removeThread(threadId: string) {
    await providers.removeThread(threadId);
    delete composerImagesByThread[threadId];
    delete attachmentErrorsByThread[threadId];
    threads = threads.filter((thread) => thread.id !== threadId);
    if (permission?.threadId === threadId) permission = null;
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
      provider.status = 'error';
      provider.error = errorMessage(error);
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
    if (provider.status !== 'disconnected' && provider.status !== 'ready') return;
    const text = thread.draft.trim();
    const images = composerImages(thread.id);
    if (!text && images.length === 0) return;
    if (providerCatalogs[thread.profileId]?.status !== 'ready' || !provider.selectedModelId) return;

    const profileId = thread.profileId;
    const isFirstPrompt = !thread.messages.some((message) => message.role === 'user');
    const selectedModelId = provider.selectedModelId;
    const messageImages: MessageImage[] = images.map(({ data, mimeType, name }) => ({ data, mimeType, name }));
    const promptImages = messageImages.map(({ data, mimeType }) => ({ type: 'image' as const, data, mimeType }));
    thread.draft = '';
    composerImagesByThread[thread.id] = [];
    attachmentErrorsByThread[thread.id] = '';
    provider.error = undefined;
    if (isFirstPrompt && !text) thread.title = 'Image prompt';
    addMessage(thread, 'user', text, messageImages);
    providers.startTurn(thread.id, profileId);

    let connection = providers.connection(thread.id, profileId);
    if (provider.status === 'disconnected') connection = await providers.connect(thread, profileId);
    if (thread.profileId !== profileId || thread.providers[profileId].status !== 'ready' || !connection) return;

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

  function reconnect() {
    if (selectedThread) void providers.connect(selectedThread, selectedThread.profileId);
  }

  function answerPermission(optionId?: string) {
    const active = permission;
    if (!active) return;
    permission = null;
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
      allowWindowClose = true;
      await appWindow.close();
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

  function errorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<svelte:head><title>{settingsOpen ? 'Settings' : (selectedThread?.title ?? 'LoopCode')} | LoopCode</title></svelte:head>

<div class:maximized={windowMaximized} class:sidebar-collapsed={sidebarCollapsed} class:compact-session-rows={compactSessionRows} class="app-shell">
  <Titlebar
    {settingsOpen}
    {selectedThread}
    {toggleSidebar}
    {addThread}
    {closeApp}
    minimize={() => { void appWindow.minimize(); }}
    toggleMaximize={() => { void appWindow.toggleMaximize(); }}
  />
  <div class="workspace-grid">
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
      {selectThread}
      {selectThreadFromKeyboard}
      {toggleSettled}
      removeThread={(threadId) => { void removeThread(threadId); }}
      setShowSettled={(show) => { showSettled = show; }}
      {openSettings}
      {closeSettings}
    />
    <main class="conversation">
      <div class="conversation-primary">
        {#if settingsOpen}
          <SettingsPage {compactSessionRows} setCompactSessionRows={(value) => { compactSessionRows = value; }} />
        {:else if selectedThread}
          <Transcript thread={selectedThread} entries={selectedTimelineEntries} reducedMotion={reducedMotion} />
          <Composer
            thread={selectedThread}
            catalogs={providerCatalogs}
            images={composerImages(selectedThread.id)}
            attachmentError={attachmentErrorsByThread[selectedThread.id]}
            projectName={projectNameForThread(selectedThread)}
            {currentBranch}
            attachImages={(files) => { void attachImages(files, selectedThread.id); }}
            removeImage={(imageId) => removeComposerImage(selectedThread.id, imageId)}
            send={() => { void sendPrompt(); }}
            cancel={() => { void cancelPrompt(); }}
            {reconnect}
            selectModel={(profileId, model) => { void selectModel(profileId, model); }}
            selectReasoning={(reasoningId) => { void selectReasoning(reasoningId); }}
            {activateProvider}
            retryDiscovery={(profileId) => {
              void providers.discover(profileById(profileId), defaultWorkingFolder, threads);
            }}
          />
        {/if}
      </div>
    </main>
  </div>
</div>

<svelte:window onkeydown={(event) => {
  if (event.key === 'Escape' && workspaceDropdownOpen) workspaceDropdownOpen = false;
}} />

{#if permission}
  <PermissionModal
    request={permission.request}
    answer={(optionId) => answerPermission(optionId)}
    decline={() => answerPermission()}
  />
{/if}
