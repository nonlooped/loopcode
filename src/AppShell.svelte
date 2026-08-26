<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import DeleteThreadModal from './components/DeleteThreadModal.svelte';
  import PermissionModal from './components/PermissionModal.svelte';
  import RenameThreadModal from './components/RenameThreadModal.svelte';
  import SelectedThreadWorkspace from './components/SelectedThreadWorkspace.svelte';
  import SettingsPage from './components/SettingsPage.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Titlebar from './components/Titlebar.svelte';
  import { profileById, profiles as officialProfiles } from './config/providers';
  import { preferredAllowOptionId } from './services/acp';
  import {
    getInitialWorkingDirectory,
    getProviderAuthStatus,
    getProviderVersion,
    loadWorkspace,
    openProjectPath,
    pickFolder,
    registerFrontend,
    revealProjectPath,
    saveWorkspace,
    stopAllHarnesses,
    stopAllTerminals,
    stopTerminalForThread,
  } from './services/native';
  import { ProviderRuntime } from './services/provider-runtime';
  import { createWorkspaceState, Workspace } from './services/workspace';
  import { WorkspacePersistence } from './services/workspace-persistence';
  import type {
    ComposerImage,
    HarnessProfile,
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
    configuredProviderProfiles,
    loadAppPreferences,
    loadPermissionMode,
    loadSidebarWidths,
    loadTerminalHeight,
    providerVersionFromOutput,
    resetInterfaceSettings as resetStoredInterfaceSettings,
    saveAppPreference,
    savePermissionMode,
    saveSidebarWidth,
    saveTerminalHeight,
    type AppPreferences,
    type ProviderPreference,
    type SettingsCategory,
  } from './utils/app-settings';
  import { addMessage } from './utils/messages';
  import {
    initialProviderCatalog,
    previewProviderCatalog,
    readyProviderId,
  } from './utils/provider-availability';
  import { activeProvider, compareSidebarThreads } from './utils/threads';

  const appWindow = getCurrentWindow();
  const appWebview = getCurrentWebview();
  const webPreview = !import.meta.env.TAURI_ENV_PLATFORM;
  const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
  const colorPreference = window.matchMedia('(prefers-color-scheme: dark)');
  const loadedPreferences = loadAppPreferences();
  document.documentElement.dataset.theme = loadedPreferences.theme;
  document.documentElement.dataset.colorMode = loadedPreferences.colorMode === 'system'
    ? colorPreference.matches ? 'dark' : 'light'
    : loadedPreferences.colorMode;
  loadedPreferences.defaultProviderId = profileById(loadedPreferences.defaultProviderId)?.id ?? officialProfiles[0].id;
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
  loadedPreferences.titleProviderId = profileById(loadedPreferences.titleProviderId)?.id ?? officialProfiles[0].id;
  const initialProfiles = configuredProviderProfiles(officialProfiles, loadedPreferences.providerSettings);
  const initialCatalogs = Object.fromEntries(
    initialProfiles.map((profile) => [
      profile.id,
      webPreview ? previewProviderCatalog(profile) : initialProviderCatalog(profile),
    ]),
  );

  let defaultWorkingFolder = $state(loadedPreferences.defaultWorkingFolder);
  let initialWorkingFolder = $state('');
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
  let showSettled = $state(false);
  let projectExplorerCollapsed = $state(true);
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
  let systemDarkMode = $state(colorPreference.matches);
  const initialPermissionMode = loadPermissionMode();
  let permissionMode = $state<PermissionMode>(initialPermissionMode);
  let windowMaximized = $state(false);
  let composerImagesByThread = $state<Record<string, ComposerImage[]>>({});
  let attachmentErrorsByThread = $state<Record<string, string>>({});
  let gitOperationBusy = $state(false);
  let zoomPercent = $state<number>();
  let workspaceSaveError = $state('');
  let zoomNoticeTimer: number | undefined;
  let closing = false;
  const providerVersionGenerations = new Map<string, number>();
  const providerAuthGenerations = new Map<string, number>();
  let stopSidebarResize: (() => void) | undefined;
  let stopTerminalResize: (() => void) | undefined;

  const reducedMotion = $derived(
    systemReducedMotion || preferences.motionMode === 'reduced',
  );
  const resolvedColorMode = $derived(
    preferences.colorMode === 'system'
      ? systemDarkMode ? 'dark' : 'light'
      : preferences.colorMode,
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
  ].filter(Boolean).join('; '));

  const workspace = new Workspace(
    workspaceState,
    providerCatalogs,
    new WorkspacePersistence(saveWorkspace, reportWorkspaceSaveFailure),
  );
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

  $effect(() => {
    workspace.queuePersistence();
  });

  const selectedThread = $derived(threads.find((thread) => thread.id === selectedThreadId));
  const selectedInteraction = $derived(
    selectedThread ? interactions[interactionKey(selectedThread.id, selectedThread.profileId)] : undefined,
  );
  const interactionRequestsByThread = $derived.by(() => {
    const requests: Record<string, PermissionRequest> = {};
    for (const thread of threads) {
      const interaction = interactions[interactionKey(thread.id, thread.profileId)];
      if (interaction) requests[thread.id] = interaction.request;
    }
    return requests;
  });
  const inboxThreads = $derived(
    threads.filter((thread) => !thread.settled).sort((left, right) =>
      compareSidebarThreads(
        left,
        right,
        interactionRequestsByThread[left.id],
        interactionRequestsByThread[right.id],
      )),
  );
  const settledThreads = $derived(
    threads.filter((thread) => thread.settled).sort((left, right) => right.updatedAt - left.updatedAt),
  );
  const activeProject = $derived(workspace.activeProject);
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
    const root = document.documentElement;
    root.dataset.theme = preferences.theme;
    root.dataset.colorMode = resolvedColorMode;
    document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')
      ?.setAttribute('content', getComputedStyle(root).getPropertyValue('--shell-solid').trim());
  });

  $effect(() => {
    providers.setTitlePreference(preferences.automaticTitleGeneration
      ? { profileId: preferences.titleProviderId, modelId: preferences.titleModelId }
      : undefined);
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

  onMount(() => {
    void initializeWorkspace();
    let disposed = false;
    let unlistenResize: (() => void) | undefined;
    let unlistenClose: (() => void) | undefined;
    const syncMotionPreference = (event: MediaQueryListEvent) => {
      systemReducedMotion = event.matches;
    };
    const syncColorPreference = (event: MediaQueryListEvent) => {
      systemDarkMode = event.matches;
    };
    motionPreference.addEventListener('change', syncMotionPreference);
    colorPreference.addEventListener('change', syncColorPreference);

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
      motionPreference.removeEventListener('change', syncMotionPreference);
      colorPreference.removeEventListener('change', syncColorPreference);
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
  }

  function focusComposer() {
    void tick().then(() => document.querySelector<HTMLElement>('.prompt-editor')?.focus());
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
      preferences.newThreadProject === 'selected' ? selectedProjectId : null,
      hasComposerImages,
    );
    if (!thread) return;
    applyNewThreadDefaults(thread);
    closeThreadSurfaces();
    focusComposer();
  }

  function addThreadToProject(projectId: string) {
    const thread = workspace.addThread(
      defaultWorkingFolder,
      projectId,
      hasComposerImages,
    );
    if (!thread) return;
    applyNewThreadDefaults(thread);
    closeThreadSurfaces();
    focusComposer();
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

  function selectThread(threadId: string) {
    if (workspace.selectThread(threadId)) closeThreadSurfaces();
  }

  function toggleSidebar() {
    if (window.matchMedia('(max-width: 880px)').matches) sidebarOpen = !sidebarOpen;
    else sidebarCollapsed = !sidebarCollapsed;
  }

  function toggleTerminal() {
    if (!selectedThread || settingsOpen || gitOperationBusy) return;
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
      initialWorkingFolder = await getInitialWorkingDirectory();
      if (!defaultWorkingFolder) defaultWorkingFolder = initialWorkingFolder;
      const savedWorkspace = await loadWorkspace();
      if (!workspace.initialize(savedWorkspace, defaultWorkingFolder)) {
        throw new Error('The saved thread file has an unsupported or invalid format.');
      }
      configureProviderRuntime(initialProfiles, loadedPreferences.providerSettings);
      if (preferences.startupBehavior === 'new-thread') {
        const startupThread = workspace.addThread(
          defaultWorkingFolder,
          preferences.newThreadProject === 'selected' ? selectedProjectId : null,
          hasComposerImages,
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

  function hasComposerImages(threadId: string) {
    return composerImages(threadId).length > 0;
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

  async function cancelPrompt() {
    if (!selectedThread) return;
    await providers.cancel(selectedThread);
    addMessage(selectedThread, 'notice', 'The active turn was cancelled.');
  }

  function setPreference<K extends keyof AppPreferences>(key: K, value: AppPreferences[K]) {
    preferences[key] = value;
    saveAppPreference(key, value);
  }

  async function chooseDefaultWorkingFolder() {
    const folder = await pickFolder();
    if (!folder) return;
    defaultWorkingFolder = folder;
    setPreference('defaultWorkingFolder', folder);
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
    configureProviderRuntime(nextProfiles, providerSettings);
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

  function configureProviderRuntime(
    nextProfiles: HarnessProfile[],
    settings: Record<string, ProviderPreference | undefined>,
  ) {
    providers.configure({
      profiles: nextProfiles,
      customModels: Object.fromEntries(
        nextProfiles.map((profile) => [profile.id, settings[profile.id]?.models ?? []]),
      ),
      disabledProfileIds: nextProfiles.flatMap((profile) =>
        settings[profile.id]?.enabled === false ? [profile.id] : [],
      ),
    }, threads);
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

  function resetSettings() {
    resetStoredInterfaceSettings();
    preferences = {
      ...preferences,
      colorMode: DEFAULT_APP_PREFERENCES.colorMode,
      theme: DEFAULT_APP_PREFERENCES.theme,
      compactSessionRows: DEFAULT_APP_PREFERENCES.compactSessionRows,
      motionMode: DEFAULT_APP_PREFERENCES.motionMode,
      interfaceZoom: DEFAULT_APP_PREFERENCES.interfaceZoom,
      transcriptDensity: DEFAULT_APP_PREFERENCES.transcriptDensity,
      contentWidth: DEFAULT_APP_PREFERENCES.contentWidth,
      wrapCode: DEFAULT_APP_PREFERENCES.wrapCode,
      showMessageTimestamps: DEFAULT_APP_PREFERENCES.showMessageTimestamps,
    };
    projectExplorerCollapsed = true;
    terminalHeight = DEFAULT_TERMINAL_HEIGHT;
    leftSidebarWidth = null;
    rightSidebarWidth = null;
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
      workspaceSaveError = `Could not close LoopCode: ${errorMessage(error)}`;
    }
  }

  function reportWorkspaceSaveFailure(error: unknown) {
    workspaceSaveError = `Could not save workspace: ${errorMessage(error)}`;
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
  class:sidebar-collapsed={sidebarCollapsed}
  class:project-explorer-collapsed={!selectedThread?.cwd || settingsOpen || (!compactLayout && projectExplorerCollapsed)}
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
  {#if workspaceSaveError}
    <div class="workspace-save-error" role="alert">
      <span>{workspaceSaveError}</span>
      <button type="button" onclick={() => { workspaceSaveError = ''; }}>Dismiss</button>
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
      {interactionRequestsByThread}
      {settledThreads}
      {selectedThreadId}
      showSettled={showSettled}
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
      setShowSettled={(show) => { showSettled = show; }}
      clearArchivedThreads={() => { void clearArchivedThreads(); }}
      {openSettings}
      {closeSettings}
      {setSettingsCategory}
    startResize={(event) => startSidebarResize(event, 'left')}
  />
  <SelectedThreadWorkspace
    {settingsOpen}
    {selectedThread}
    {selectedThreadId}
    {threads}
    {workspace}
    {providers}
    {profiles}
    {selectableProfiles}
    catalogs={providerCatalogs}
    {selectedInteraction}
    composerImages={selectedThread ? composerImages(selectedThread.id) : []}
    attachmentError={selectedThread ? attachmentErrorsByThread[selectedThread.id] : undefined}
    {terminalThreads}
    {terminalThreadIds}
    {terminalVisible}
    {terminalHeight}
    {preferences}
    {reducedMotion}
    {compactLayout}
    bind:projectExplorerCollapsed
    {defaultWorkingFolder}
    attachImages={(files) => { if (selectedThread) void attachImages(files, selectedThread.id); }}
    removeImage={(imageId) => { if (selectedThread) removeComposerImage(selectedThread.id, imageId); }}
    clearAttachments={() => {
      if (!selectedThread) return;
      composerImagesByThread[selectedThread.id] = [];
      attachmentErrorsByThread[selectedThread.id] = '';
    }}
    {answerQuestion}
    dismissQuestion={() => answerPermission()}
    {cancelPrompt}
    {closeTerminal}
    {terminalExited}
    {startTerminalResize}
    resizeTerminalBy={resizeTerminalDrawerBy}
    startSidebarResize={(event) => startSidebarResize(event, 'right')}
    setGitBusy={(busy) => { gitOperationBusy = busy; }}
  >
    {#snippet settings()}
      <SettingsPage
        category={settingsCategory}
        {preferences}
        {profiles}
        baseProfiles={officialProfiles}
        catalogs={providerCatalogs}
        {providerVersions}
        {providerAuthStatuses}
        {permissionMode}
        {reducedMotion}
        {setPreference}
        {setProviderPreference}
        {setPermissionMode}
        {defaultWorkingFolder}
        chooseDefaultWorkingFolder={() => { void chooseDefaultWorkingFolder(); }}
        {resetSettings}
      />
    {/snippet}
  </SelectedThreadWorkspace>
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
