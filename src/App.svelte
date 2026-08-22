<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import Composer from './components/Composer.svelte';
  import DeleteThreadModal from './components/DeleteThreadModal.svelte';
  import FileViewer from './components/FileViewer.svelte';
  import PermissionModal from './components/PermissionModal.svelte';
  import ProjectExplorer from './components/ProjectExplorer.svelte';
  import QuestionComposer from './components/QuestionComposer.svelte';
  import RenameThreadModal from './components/RenameThreadModal.svelte';
  import SettingsPage from './components/SettingsPage.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TerminalDrawer from './components/TerminalDrawer.svelte';
  import Titlebar from './components/Titlebar.svelte';
  import Transcript from './components/Transcript.svelte';
  import { profileById, profiles as officialProfiles } from './config/providers';
  import { preferredAllowOptionId } from './services/acp';
  import {
    getGitBranch,
    getInitialWorkingDirectory,
    getProviderAuthStatus,
    getProviderVersion,
    loadWorkspace,
    openProjectPath,
    pickFolder,
    registerFrontend,
    revealProjectPath,
    stopAllHarnesses,
    stopAllTerminals,
    stopTerminalForThread,
  } from './services/native';
  import { ProviderRuntime } from './services/provider-runtime';
  import { createWorkspaceState, Workspace } from './services/workspace';
  import type {
    ComposerImage,
    HarnessProfile,
    MessageImage,
    ModelOption,
    PermissionMode,
    PermissionRequest,
    ProjectState,
    QuestionAnswer,
    ProviderModelCatalog,
    ThreadState,
  } from './types';
  import { loadComposerImages, MAX_COMPOSER_IMAGES } from './utils/attachments';
  import {
    DEFAULT_APP_PREFERENCES,
    DEFAULT_TERMINAL_HEIGHT,
    INTERFACE_ZOOM_RANGE,
    LEFT_SIDEBAR_WIDTH_RANGE,
    RIGHT_SIDEBAR_WIDTH_RANGE,
    TERMINAL_HEIGHT_RANGE,
    LINUX_SHELL_TRANSPARENCY_RANGE,
    MAX_LINUX_SHELL_TRANSPARENCY,
    configuredProviderProfiles,
    loadAppPreferences,
    loadLinuxShellTransparency,
    loadPermissionMode,
    loadSidebarWidths,
    loadTerminalHeight,
    providerVersionFromOutput,
    resetAppSettings as resetStoredAppSettings,
    saveAppPreference,
    saveLinuxShellTransparency,
    savePermissionMode,
    saveSidebarWidth,
    saveTerminalHeight,
    type AppPreferences,
    type ProviderPreference,
    type SettingsCategory,
  } from './utils/app-settings';
  import { addMessage } from './utils/messages';
  import { hasPromptContent, promptParts, promptText } from './utils/prompt-content';
  import {
    initialProviderCatalog,
    previewProviderCatalog,
    readyProviderId,
  } from './utils/provider-availability';
  import { timelineEntries } from './utils/timeline';
  import { activeProvider, compareSidebarThreads, threadStatus } from './utils/threads';

  const appWindow = getCurrentWindow();
  const appWebview = getCurrentWebview();
  const webPreview = !import.meta.env.TAURI_ENV_PLATFORM;
  const isLinux = navigator.userAgent.includes('Linux');
  const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
  const loadedPreferences = loadAppPreferences();
  loadedPreferences.defaultProviderId = profileById(loadedPreferences.defaultProviderId).id;
  loadedPreferences.providerModelDefaults = Object.fromEntries(
    officialProfiles.flatMap((profile) => {
      const modelId = loadedPreferences.providerModelDefaults[profile.id];
      return modelId ? [[profile.id, modelId]] : [];
    }),
  );
  loadedPreferences.providerSettings = Object.fromEntries(
    officialProfiles.flatMap((profile) => {
      const setting = loadedPreferences.providerSettings[profile.id];
      return setting ? [[profile.id, setting]] : [];
    }),
  );
  loadedPreferences.titleProviderId = profileById(loadedPreferences.titleProviderId).id;
  const initialProfiles = configuredProviderProfiles(officialProfiles, loadedPreferences.providerSettings);
  const initialCatalogs = Object.fromEntries(
    initialProfiles.map((profile) => [
      profile.id,
      webPreview ? previewProviderCatalog(profile) : initialProviderCatalog(profile),
    ]),
  );

  let defaultWorkingFolder = $state('');
  let providerCatalogs = $state<Record<string, ProviderModelCatalog>>(initialCatalogs);
  let providerVersions = $state<Record<string, string>>({});
  let providerAuthStatuses = $state<Record<string, boolean>>({});
  const workspaceState = $state(createWorkspaceState('', providerCatalogs));
  const projects = $derived(workspaceState.projects);
  const selectedProjectId = $derived(workspaceState.selectedProjectId);
  const threads = $derived(workspaceState.threads);
  const selectedThreadId = $derived(workspaceState.selectedThreadId);
  let threadPendingRemoval = $state<ThreadState>();
  let threadPendingRename = $state<ThreadState>();
  let interactions = $state<Record<string, {
    threadId: string;
    profileId: string;
    request: PermissionRequest;
  }>>({});
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let projectExplorerOpen = $state(false);
  let projectExplorerCollapsed = $state(loadedPreferences.explorerStartup === 'collapsed');
  let terminalOpen = $state(false);
  let terminalThreadIds = $state<string[]>([]);
  let terminalHeight = $state(loadTerminalHeight());
  const savedSidebarWidths = loadSidebarWidths();
  let leftSidebarWidth = $state<number | null>(savedSidebarWidths.left);
  let rightSidebarWidth = $state<number | null>(savedSidebarWidths.right);
  let compactLayout = $state(window.matchMedia('(max-width: 880px)').matches);
  let settingsOpen = $state(false);
  let settingsCategory = $state<SettingsCategory>('general');
  let preferences = $state<AppPreferences>(loadedPreferences);
  let systemReducedMotion = $state(motionPreference.matches);
  let linuxShellTransparency = $state(isLinux ? loadLinuxShellTransparency() : 0);
  const initialPermissionMode = loadPermissionMode();
  let permissionMode = $state<PermissionMode>(initialPermissionMode);
  let windowMaximized = $state(false);
  let composerImagesByThread = $state<Record<string, ComposerImage[]>>({});
  let attachmentErrorsByThread = $state<Record<string, string>>({});
  let currentBranch = $state<string | null | undefined>(undefined);
  let fileHistory = $state<string[]>([]);
  let fileHistoryIndex = $state(-1);
  let fileRevision = $state(0);
  let composerCompletionRevision = $state(0);
  let fileViewerThreadId = $state('');
  let threadViewElement = $state<HTMLElement>();
  let zoomPercent = $state<number>();
  let zoomNoticeTimer: number | undefined;
  let completionRefreshTimer: number | undefined;
  let closing = false;
  let branchLookup = 0;
  const providerVersionGenerations = new Map<string, number>();
  const providerAuthGenerations = new Map<string, number>();
  let stopSidebarResize: (() => void) | undefined;
  let stopTerminalResize: (() => void) | undefined;

  const reducedMotion = $derived(
    systemReducedMotion || preferences.motionMode === 'reduced',
  );
  const profiles = $derived(configuredProviderProfiles(officialProfiles, preferences.providerSettings));
  const enabledProfiles = $derived(
    profiles.filter((profile) => preferences.providerSettings[profile.id]?.enabled !== false),
  );
  const selectableProfiles = $derived(enabledProfiles);
  const layoutStyle = $derived([
    leftSidebarWidth === null ? '' : `--sidebar-expanded-width: ${leftSidebarWidth}px`,
    rightSidebarWidth === null ? '' : `--project-explorer-expanded-width: ${rightSidebarWidth}px`,
    `--terminal-height: ${terminalHeight}px`,
    `--content-width: ${preferences.contentWidth}px`,
    isLinux
      ? `--linux-shell-opacity: ${1 - (linuxShellTransparency / 100) * MAX_LINUX_SHELL_TRANSPARENCY / 100}`
      : '',
  ].filter(Boolean).join('; '));

  const workspace = new Workspace(workspaceState, providerCatalogs);
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
  const threadsWithDrafts = $derived(
    threads.filter((thread) =>
      hasPromptContent(thread.draft, thread.draftReferences)
      || composerImages(thread.id).length > 0
    ).length,
  );
  const activeProject = $derived(workspace.activeProject);
  const explorerRoot = $derived(selectedThread?.cwd ?? '');
  const explorerProjectName = $derived(
    selectedThread ? workspace.projectNameForThread(selectedThread) : 'Project',
  );
  const projectExplorerVisible = $derived(
    compactLayout ? projectExplorerOpen : !projectExplorerCollapsed,
  );
  const activeFilePath = $derived(fileHistoryIndex >= 0 ? fileHistory[fileHistoryIndex] ?? null : null);
  const terminalVisible = $derived(terminalOpen && !settingsOpen && Boolean(selectedThread));
  const terminalThreads = $derived(
    terminalThreadIds.flatMap((threadId) => {
      const thread = threads.find((item) => item.id === threadId);
      return thread ? [thread] : [];
    }),
  );

  $effect(() => {
    void appWebview.setZoom(preferences.interfaceZoom / 100);
  });

  $effect(() => {
    providers.setTitlePreference({
      profileId: preferences.titleProviderId,
      modelId: preferences.titleModelId,
    });
  });

  $effect(() => {
    document.body.classList.toggle('reduced-motion', reducedMotion);
    return () => document.body.classList.remove('reduced-motion');
  });

  $effect(() => {
    const threadId = selectedThread?.id;
    if (terminalVisible && threadId && !terminalThreadIds.includes(threadId)) {
      terminalThreadIds = [...terminalThreadIds, threadId];
    }
  });

  $effect(() => {
    if (selectedThreadId === fileViewerThreadId) return;
    fileViewerThreadId = selectedThreadId;
    fileHistory = [];
    fileHistoryIndex = -1;
    fileRevision = 0;
  });

  $effect(() => {
    workspace.queuePersistence();
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
    const syncMotionPreference = (event: MediaQueryListEvent) => {
      systemReducedMotion = event.matches;
    };
    motionPreference.addEventListener('change', syncMotionPreference);

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
      stopTerminalResize?.();
      window.clearTimeout(zoomNoticeTimer);
      window.clearTimeout(completionRefreshTimer);
      motionPreference.removeEventListener('change', syncMotionPreference);
      unlistenResize?.();
      unlistenClose?.();
    };
  });

  function handleAppKeydown(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
    if (event.code === 'Backquote' && selectedThread && !settingsOpen) {
      event.preventDefault();
      toggleTerminal();
      return;
    }
    const direction = event.key === '-' ? -1 : event.key === '+' || event.key === '=' ? 1 : 0;
    if (!direction) return;

    event.preventDefault();
    const nextZoom = Math.min(
      INTERFACE_ZOOM_RANGE.max,
      Math.max(INTERFACE_ZOOM_RANGE.min, preferences.interfaceZoom + direction * 10),
    );
    setPreference('interfaceZoom', nextZoom);
    zoomPercent = nextZoom;
    window.clearTimeout(zoomNoticeTimer);
    zoomNoticeTimer = window.setTimeout(() => { zoomPercent = undefined; }, 1200);
  }

  function closeThreadSurfaces() {
    sidebarOpen = false;
    projectExplorerOpen = false;
  }

  function applyNewThreadDefaults(thread: ThreadState) {
    const defaultProviderId = readyProviderId(
      preferences.defaultProviderId,
      enabledProfiles,
      providerCatalogs,
    ) ?? (providerCatalogs[preferences.defaultProviderId]?.status === 'loading'
      ? preferences.defaultProviderId
      : thread.profileId);
    providers.activate(thread, defaultProviderId, false);
    for (const [profileId, modelId] of Object.entries(preferences.providerModelDefaults)) {
      if (thread.providers[profileId]) void providers.selectModel(thread, profileId, modelId);
    }
  }

  function addThread() {
    const thread = workspace.addThread(
      defaultWorkingFolder,
      (threadId) => composerImages(threadId).length > 0,
      preferences.newThreadProject === 'selected' ? selectedProjectId : null,
      preferences.reuseEmptyThreads,
    );
    if (thread) applyNewThreadDefaults(thread);
    closeThreadSurfaces();
  }

  function addThreadToProject(projectId: string) {
    const thread = workspace.addThread(
      defaultWorkingFolder,
      (threadId) => composerImages(threadId).length > 0,
      projectId,
      preferences.reuseEmptyThreads,
    );
    if (thread) {
      applyNewThreadDefaults(thread);
      closeThreadSurfaces();
    }
  }

  async function openAddProject() {
    try {
      const picked = await pickFolder();
      if (!picked) return;
      workspace.selectProject(workspace.ensureProject(picked).id);
    } catch (error) {
      const thread = selectedThread ?? threads[0];
      if (thread) addMessage(thread, 'error', `Could not add folder: ${errorMessage(error)}`);
    }
  }

  function selectProject(projectId: string | null) {
    workspace.selectProject(projectId);
  }

  function renameThread(threadId: string) {
    threadPendingRename = threads.find((thread) => thread.id === threadId);
  }

  function saveThreadRename(title: string) {
    const thread = threadPendingRename;
    threadPendingRename = undefined;
    if (thread) workspace.renameThread(thread.id, title);
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
    workspace.removeProject(projectId);
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
    workspace.toggleSettled(threadId);
  }

  function requestThreadRemoval(threadId: string) {
    threadPendingRemoval = threads.find((thread) => thread.id === threadId);
  }

  async function removeThread(threadId: string) {
    threadPendingRemoval = undefined;
    const thread = threads.find((item) => item.id === threadId);
    try {
      await Promise.all([providers.removeThread(threadId), stopTerminalForThread(threadId)]);
    } catch (error) {
      if (thread) addMessage(thread, 'error', `Could not stop the thread: ${errorMessage(error)}`);
      return;
    }
    terminalThreadIds = terminalThreadIds.filter((id) => id !== threadId);
    delete composerImagesByThread[threadId];
    delete attachmentErrorsByThread[threadId];
    for (const [key, interaction] of Object.entries(interactions)) {
      if (interaction.threadId === threadId) delete interactions[key];
    }
    const replacement = workspace.removeThread(threadId, defaultWorkingFolder);
    if (replacement) applyNewThreadDefaults(replacement);
  }

  async function clearArchivedThreads() {
    for (const threadId of settledThreads.map((thread) => thread.id)) {
      await removeThread(threadId);
    }
  }

  function clearComposerDrafts() {
    for (const thread of threads) {
      thread.draft = '';
      thread.draftReferences = [];
    }
    composerImagesByThread = {};
    attachmentErrorsByThread = {};
  }

  function selectThread(threadId: string) {
    if (workspace.selectThread(threadId)) closeThreadSurfaces();
  }

  function toggleSidebar() {
    if (window.matchMedia('(max-width: 880px)').matches) sidebarOpen = !sidebarOpen;
    else sidebarCollapsed = !sidebarCollapsed;
  }

  function toggleProjectExplorer() {
    if (compactLayout) projectExplorerOpen = !projectExplorerOpen;
    else projectExplorerCollapsed = !projectExplorerCollapsed;
  }

  function toggleTerminal() {
    if (!selectedThread || settingsOpen) return;
    if (terminalOpen) {
      closeTerminal();
      return;
    }
    terminalOpen = true;
  }

  function closeTerminal() {
    terminalOpen = false;
    void tick().then(() => {
      document.querySelector<HTMLButtonElement>('.title-terminal-toggle')?.focus();
    });
  }

  function terminalExited(threadId: string) {
    terminalThreadIds = terminalThreadIds.filter((id) => id !== threadId);
    if (threadId === selectedThreadId && terminalOpen) closeTerminal();
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
    window.clearTimeout(completionRefreshTimer);
    completionRefreshTimer = window.setTimeout(() => { composerCompletionRevision += 1; }, 160);
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

  function startTerminalResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    stopTerminalResize?.();
    const conversation = document.querySelector<HTMLElement>('.conversation');
    if (!conversation) return;
    const startY = event.clientY;
    const startHeight = terminalHeight;
    const maxHeight = Math.max(
      TERMINAL_HEIGHT_RANGE.min,
      Math.min(TERMINAL_HEIGHT_RANGE.max, conversation.getBoundingClientRect().height - 120),
    );
    document.body.classList.add('resizing-terminal');

    const onPointerMove = (moveEvent: PointerEvent) => {
      terminalHeight = Math.round(Math.min(
        maxHeight,
        Math.max(TERMINAL_HEIGHT_RANGE.min, startHeight + startY - moveEvent.clientY),
      ));
    };
    const finish = () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', finish);
      window.removeEventListener('pointercancel', finish);
      document.body.classList.remove('resizing-terminal');
      saveTerminalHeight(terminalHeight);
      stopTerminalResize = undefined;
    };

    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', finish);
    window.addEventListener('pointercancel', finish);
    stopTerminalResize = finish;
  }

  function resizeTerminalDrawerBy(delta: number) {
    const conversationHeight = document.querySelector<HTMLElement>('.conversation')
      ?.getBoundingClientRect().height ?? window.innerHeight - 38;
    terminalHeight = Math.round(Math.min(
      TERMINAL_HEIGHT_RANGE.max,
      Math.max(TERMINAL_HEIGHT_RANGE.min, Math.min(conversationHeight - 120, terminalHeight + delta)),
    ));
    saveTerminalHeight(terminalHeight);
  }

  function openSettings() {
    settingsOpen = true;
    sidebarCollapsed = false;
    sidebarOpen = false;
    void Promise.all(profiles.map(loadProviderMetadata));
  }

  function closeSettings() {
    settingsOpen = false;
    sidebarOpen = false;
  }

  function setSettingsCategory(category: SettingsCategory) {
    settingsCategory = category;
    if (category === 'providers') void Promise.all(profiles.map(loadProviderMetadata));
    if (!compactLayout) return;
    sidebarOpen = false;
    void tick().then(() => document.getElementById('settings-title')?.focus());
  }

  async function initializeWorkspace() {
    try {
      await registerFrontend();
      defaultWorkingFolder = await getInitialWorkingDirectory();
      const savedWorkspace = await loadWorkspace();
      if (!workspace.initialize(savedWorkspace, defaultWorkingFolder)) {
        throw new Error('The saved thread file has an unsupported or invalid format.');
      }
      providers.setProfiles(initialProfiles, threads);
      for (const profile of initialProfiles) {
        providers.setCustomModels(profile.id, loadedPreferences.providerSettings[profile.id]?.models ?? [], threads);
      }
      providers.setDisabledProfiles(
        initialProfiles.filter((profile) => loadedPreferences.providerSettings[profile.id]?.enabled === false).map((profile) => profile.id),
        threads,
      );
      if (preferences.startupBehavior === 'new-thread') {
        const startupThread = workspace.addThread(
          defaultWorkingFolder,
          () => false,
          preferences.newThreadProject === 'selected' ? selectedProjectId : null,
          preferences.reuseEmptyThreads,
        );
        if (startupThread) applyNewThreadDefaults(startupThread);
      } else if (savedWorkspace === null && selectedThread) {
        applyNewThreadDefaults(selectedThread);
      }
      if (!webPreview) await providers.discoverAll(defaultWorkingFolder, threads);
      void Promise.all(initialProfiles.map(loadProviderMetadata));
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
    const content = promptParts(thread.draft, thread.draftReferences);
    const text = promptText(content).trim();
    const referencedContent = thread.draftReferences.length > 0 ? content : undefined;
    const images: MessageImage[] = composerImages(thread.id)
      .map(({ data, mimeType, name }) => ({ data, mimeType, name }));
    const centeredComposerTop = selectedThreadEmpty && !reducedMotion
      ? threadViewElement?.querySelector<HTMLElement>('.composer-wrap')?.getBoundingClientRect().top
      : undefined;
    const turn = providers.runTurn(thread, text, images, referencedContent);
    if (!turn) return;

    thread.draft = '';
    thread.draftReferences = [];
    composerImagesByThread[thread.id] = [];
    attachmentErrorsByThread[thread.id] = '';
    if (centeredComposerTop !== undefined) animateComposerToTranscript(centeredComposerTop);
    await turn;
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

  function setPreference<K extends keyof AppPreferences>(key: K, value: AppPreferences[K]) {
    preferences[key] = value;
    saveAppPreference(key, value);
  }

  function setProviderPreference(profileId: string, preference: ProviderPreference) {
    const official = officialProfiles.find((profile) => profile.id === profileId);
    if (!official) return;
    const models = [...new Map((preference.models ?? []).flatMap((model) => {
      const id = model.id.trim().slice(0, 256);
      const name = model.name.trim().slice(0, 256);
      const option: ModelOption = { id, name };
      return id && name ? [[id, option] as const] : [];
    })).values()].slice(0, 100);
    const setting: ProviderPreference = {
      ...(preference.enabled === false ? { enabled: false } : {}),
      ...(preference.name?.trim() && preference.name.trim() !== official.label
        ? { name: preference.name.trim().slice(0, 100) }
        : {}),
      ...(preference.command?.trim() && preference.command.trim() !== official.command
        ? { command: preference.command.trim().slice(0, 4096) }
        : {}),
      ...(models.length > 0 ? { models } : {}),
    };
    const previous = preferences.providerSettings[profileId];
    const providerSettings = { ...preferences.providerSettings };
    if (Object.keys(setting).length > 0) providerSettings[profileId] = setting;
    else delete providerSettings[profileId];
    setPreference('providerSettings', providerSettings);

    const nextProfiles = configuredProviderProfiles(officialProfiles, providerSettings);
    providers.setProfiles(nextProfiles, threads);
    providers.setCustomModels(profileId, models, threads);
    providers.setDisabledProfiles(
      nextProfiles.filter((profile) => providerSettings[profile.id]?.enabled === false).map((profile) => profile.id),
      threads,
    );
    const enabled = setting.enabled !== false;
    const commandChanged = (previous?.command || official.command) !== (setting.command || official.command);
    if (!webPreview && enabled && (previous?.enabled === false || commandChanged)) {
      const profile = nextProfiles.find((candidate) => candidate.id === profileId);
      if (profile) void rediscoverProvider(profile);
    } else if (commandChanged) {
      const profile = nextProfiles.find((candidate) => candidate.id === profileId);
      if (profile) void loadProviderMetadata(profile);
    }
  }

  async function rediscoverProvider(profile: HarnessProfile) {
    await providers.discover(profile, defaultWorkingFolder, threads);
    await loadProviderMetadata(profile);
  }

  async function loadProviderMetadata(profile: HarnessProfile) {
    const catalog = providerCatalogs[profile.id];
    await Promise.all([
      catalog?.status === 'unavailable' && catalog.unavailableReason === 'missing-executable'
        ? Promise.resolve()
        : loadProviderVersion(profile),
      loadProviderAuthStatus(profile),
    ]);
  }

  async function loadProviderVersion(profile: HarnessProfile) {
    const generation = (providerVersionGenerations.get(profile.id) ?? 0) + 1;
    providerVersionGenerations.set(profile.id, generation);
    try {
      const output = await getProviderVersion(profile.versionCommand, profile.versionArgs);
      if (providerVersionGenerations.get(profile.id) !== generation) return;
      const version = output ? providerVersionFromOutput(output) : undefined;
      if (version) providerVersions[profile.id] = version;
      else delete providerVersions[profile.id];
    } catch {
      if (providerVersionGenerations.get(profile.id) === generation) {
        delete providerVersions[profile.id];
      }
    }
  }

  async function loadProviderAuthStatus(profile: HarnessProfile) {
    if (!profile.authCommand || !profile.authArgs) return;
    const generation = (providerAuthGenerations.get(profile.id) ?? 0) + 1;
    providerAuthGenerations.set(profile.id, generation);
    try {
      const authenticated = await getProviderAuthStatus(profile.authCommand, profile.authArgs);
      if (providerAuthGenerations.get(profile.id) !== generation) return;
      if (authenticated === null) delete providerAuthStatuses[profile.id];
      else providerAuthStatuses[profile.id] = authenticated;
    } catch {
      if (providerAuthGenerations.get(profile.id) === generation) {
        delete providerAuthStatuses[profile.id];
      }
    }
  }

  function setLinuxShellTransparency(transparency: number) {
    linuxShellTransparency = saveLinuxShellTransparency(transparency);
  }

  function setTerminalHeight(height: number) {
    terminalHeight = Math.min(
      TERMINAL_HEIGHT_RANGE.max,
      Math.max(TERMINAL_HEIGHT_RANGE.min, Math.round(height)),
    );
    saveTerminalHeight(terminalHeight);
  }

  function resetSettings() {
    resetStoredAppSettings();
    preferences = {
      ...DEFAULT_APP_PREFERENCES,
      defaultProviderId: profileById(DEFAULT_APP_PREFERENCES.defaultProviderId).id,
      providerModelDefaults: {},
      providerSettings: {},
      titleProviderId: profileById(DEFAULT_APP_PREFERENCES.titleProviderId).id,
      titleModelId: '',
    };
    projectExplorerCollapsed = false;
    terminalHeight = DEFAULT_TERMINAL_HEIGHT;
    leftSidebarWidth = null;
    rightSidebarWidth = null;
    linuxShellTransparency = 0;
    permissionMode = 'restricted';
    const resetProfiles = configuredProviderProfiles(officialProfiles, {});
    providers.setProfiles(resetProfiles, threads);
    for (const profile of resetProfiles) providers.setCustomModels(profile.id, [], threads);
    providers.setDisabledProfiles([], threads);
    void (async () => {
      if (!webPreview) await providers.discoverAll(defaultWorkingFolder, threads);
      await Promise.all(resetProfiles.map(loadProviderMetadata));
    })();
    providers.setPermissionMode('restricted');
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

  function answerQuestion(answer: QuestionAnswer) {
    const active = selectedInteraction;
    if (!active || active.request.type !== 'question') return;
    delete interactions[interactionKey(active.threadId, active.profileId)];
    providers.answerQuestion(
      active.threadId,
      active.profileId,
      active.request.requestId,
      answer,
    );
  }

  async function closeApp() {
    if (closing) return;
    closing = true;
    try {
      await workspace.flush();
      await stopAllHarnesses();
      await stopAllTerminals();
      await appWindow.destroy();
    } catch (error) {
      closing = false;
      const thread = selectedThread ?? threads[0];
      if (thread) addMessage(thread, 'error', `Could not save threads before closing: ${errorMessage(error)}`);
    }
  }

  function interactionKey(threadId: string, profileId: string) {
    return `${threadId}:${profileId}`;
  }

  function errorMessage(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }
</script>

<svelte:head><title>{settingsOpen ? 'Settings' : (selectedThread?.title ?? 'LoopCode')} | LoopCode</title></svelte:head>

<div
  class:maximized={windowMaximized}
  class:linux-shell={isLinux}
  class:sidebar-collapsed={sidebarCollapsed}
  class:project-explorer-collapsed={!explorerRoot || (!compactLayout && projectExplorerCollapsed)}
  class:compact-session-rows={preferences.compactSessionRows}
  class:compact-transcript={preferences.transcriptDensity === 'compact'}
  class:wrap-message-code={preferences.wrapCode}
  class:show-message-timestamps={preferences.showMessageTimestamps}
  class="app-shell"
  style={layoutStyle}
>
  <Titlebar
    {settingsOpen}
    {selectedThread}
    {windowMaximized}
    {reducedMotion}
    {profiles}
    terminalOpen={terminalVisible}
    {toggleSidebar}
    {toggleTerminal}
    {addThread}
    {closeApp}
    minimize={() => { void appWindow.minimize(); }}
    toggleMaximize={() => { void appWindow.toggleMaximize(); }}
  />
  {#if zoomPercent}
    <div class="zoom-indicator" role="status" transition:fade={{ duration: reducedMotion ? 0 : 120 }}>
      {zoomPercent}%
    </div>
  {/if}
  <Sidebar
      open={sidebarOpen}
      {settingsOpen}
      {settingsCategory}
      {profiles}
      compactMotion={reducedMotion}
      {defaultWorkingFolder}
      {projects}
      {selectedProjectId}
      {activeProject}
      {inboxThreads}
      {settledThreads}
      {selectedThreadId}
      showSettled={preferences.showSettled}
      {selectProject}
      addProject={() => { void openAddProject(); }}
      {addThread}
      {addThreadToProject}
      {selectThread}
      {toggleSettled}
      {renameThread}
      {openThreadFolder}
      removeThread={requestThreadRemoval}
      {openProjectFolder}
      {revealProjectFolder}
      {removeProject}
      setShowSettled={(show) => setPreference('showSettled', show)}
      {openSettings}
      {closeSettings}
      {setSettingsCategory}
    startResize={(event) => startSidebarResize(event, 'left')}
  />
  <div class="workspace-grid">
    <main class="conversation">
      <div class="conversation-primary">
        {#if settingsOpen}
          <SettingsPage
            category={settingsCategory}
            {preferences}
            {profiles}
            baseProfiles={officialProfiles}
            catalogs={providerCatalogs}
            {providerVersions}
            {providerAuthStatuses}
            {isLinux}
            {linuxShellTransparency}
            {permissionMode}
            {terminalHeight}
            {reducedMotion}
            {setPreference}
            {setProviderPreference}
            setLinuxShellTransparency={setLinuxShellTransparency}
            linuxShellTransparencyRange={LINUX_SHELL_TRANSPARENCY_RANGE}
            {setPermissionMode}
            {setTerminalHeight}
            archivedThreadCount={settledThreads.length}
            draftThreadCount={threadsWithDrafts}
            clearArchivedThreads={() => { void clearArchivedThreads(); }}
            {clearComposerDrafts}
            {resetSettings}
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
                  Return to file
                </button>
              </div>
            {/if}
            <div bind:this={threadViewElement} class:empty={selectedThreadEmpty} class="thread-view">
              {#if selectedThreadEmpty}
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
                    autoFollowOutput={preferences.autoFollowOutput}
                  />
                {/key}
              {/if}
              {#if selectedInteraction?.request.type === 'question'}
                {#key selectedInteraction.request}
                  <QuestionComposer
                    request={selectedInteraction.request}
                    {reducedMotion}
                    answer={answerQuestion}
                    dismiss={() => answerPermission()}
                  />
                {/key}
              {:else}
                <Composer
                  thread={selectedThread}
                  catalogs={providerCatalogs}
                  {profiles}
                  {selectableProfiles}
                  images={composerImages(selectedThread.id)}
                  attachmentError={attachmentErrorsByThread[selectedThread.id]}
                  projectName={workspace.projectNameForThread(selectedThread)}
                  completionRevision={composerCompletionRevision}
                  {currentBranch}
                  {reducedMotion}
                  sendShortcut={preferences.sendShortcut}
                  spellcheck={preferences.composerSpellcheck}
                  autocomplete={preferences.composerAutocomplete}
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
                    const profile = profiles.find((candidate) => candidate.id === profileId);
                    if (profile) void providers.discover(profile, defaultWorkingFolder, threads);
                  }}
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
          resizeBy={resizeTerminalDrawerBy}
        />
      {/if}
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
  handleAppKeydown(event);
}} />

{#if threadPendingRename}
  <RenameThreadModal
    title={threadPendingRename.title}
    cancel={() => { threadPendingRename = undefined; }}
    save={saveThreadRename}
  />
{/if}

{#if threadPendingRemoval}
  <DeleteThreadModal
    title={threadPendingRemoval.title}
    cancel={() => { threadPendingRemoval = undefined; }}
    confirm={() => { void removeThread(threadPendingRemoval!.id); }}
  />
{/if}

{#if selectedInteraction?.request.type === 'permission'}
  <PermissionModal
    request={selectedInteraction.request}
    answer={(optionId) => answerPermission(optionId)}
    decline={() => answerPermission()}
  />
{/if}
