import { providerDefinitions, type ProviderDefinition } from "../config/provider-definitions.ts";
import type {
  AcpErrorDetails,
  ConnectionStatus,
  MessageImage,
  PermissionMode,
  PermissionRequest,
  PromptPart,
  QuestionAnswer,
  ProviderModelCatalog,
  ProviderSessionState,
  SessionSelectId,
  SlashCommand,
  ThreadState,
} from "../types/index.ts";
import { applyFastModeForSelectedModel } from "../utils/fast-mode.ts";
import { jsonValueSchema } from "../utils/json.ts";
import { loadWorkspaceMcpServers } from "../utils/mcp-servers.ts";
import { addMessage, nextTimestamp, titleFromPrompt } from "../utils/messages.ts";
import { discoverModelOptions } from "../utils/model-options.ts";
import { sessionSelectLabels } from "../utils/model-state.ts";
import {
  firstReadyProviderId,
  titleGenerationSelection,
  unavailableReason,
  type TitleGenerationPreference,
} from "../utils/provider-availability.ts";
import { applyReasoningForSelectedModel } from "../utils/reasoning-options.ts";
import { buildThreadTitlePrompt, normalizeThreadTitle } from "../utils/thread-title.ts";
import {
  AcpConnection,
  readModelState,
  type AcpCallbacks,
  type AcpModelState,
  type PromptContent,
} from "./acp.ts";
import { recordDiagnostic } from "./native.ts";
import { SessionUpdateHandler, slashCommands } from "./session-updates.ts";

interface RuntimeHooks {
  permission: (value: { threadId: string; profileId: string; request: PermissionRequest }) => void;
  clearPermission: (threadId: string, profileId?: string) => void;
}

export interface ProviderRuntimeConfiguration {
  profiles: ProviderDefinition[];
  customModels: Record<string, ProviderModelCatalog["models"] | undefined>;
  disabledProfileIds: string[];
}

export class ProviderRuntime {
  #catalogs: Record<string, ProviderModelCatalog>;
  #hooks: RuntimeHooks;
  #profiles: ProviderDefinition[] = providerDefinitions;
  #threads: ThreadState[] = [];
  #disabledProfiles = new Set<string>();
  #baseModels = new Map<string, ProviderModelCatalog["models"]>();
  #customModels = new Map<string, ProviderModelCatalog["models"]>();
  #permissionMode: PermissionMode = "restricted";
  #connections = new Map<string, AcpConnection>();
  #createConnection: (callbacks: AcpCallbacks) => AcpConnection;
  #loadMcpServers: typeof loadWorkspaceMcpServers;
  #tokens = new Map<string, string>();
  #stoppingProfiles = new Map<string, Promise<void>>();
  #turnTokens = new Map<string, string>();
  #titleTokens = new Map<string, string>();
  #titleConnections = new Map<string, AcpConnection>();
  #titlePreference?: TitleGenerationPreference;
  #updates = new SessionUpdateHandler(applyProviderConfigState);

  constructor(
    catalogs: Record<string, ProviderModelCatalog>,
    hooks: RuntimeHooks,
    createConnection = (callbacks: AcpCallbacks) => new AcpConnection(callbacks),
    loadMcpServers = loadWorkspaceMcpServers,
  ) {
    this.#catalogs = catalogs;
    this.#hooks = hooks;
    this.#createConnection = createConnection;
    this.#loadMcpServers = loadMcpServers;
    for (const [profileId, catalog] of Object.entries(catalogs)) {
      this.#baseModels.set(profileId, catalog.models);
    }
  }

  connection(threadId: string, profileId: string) {
    return this.#connections.get(connectionKey(threadId, profileId));
  }

  setPermissionMode(mode: PermissionMode) {
    this.#permissionMode = mode;
  }

  setTitlePreference(preference?: TitleGenerationPreference) {
    this.#titlePreference = preference;
  }

  configure(configuration: ProviderRuntimeConfiguration, threads = this.#threads) {
    this.#threads = [...threads];
    const disconnected = new Set<string>();
    const catalogsToApply = new Set<string>();
    const disabledProfiles = new Set(configuration.disabledProfileIds);

    for (const profile of configuration.profiles) {
      const current = this.#profiles.find((candidate) => candidate.id === profile.id);
      if (
        !current ||
        current.command !== profile.command ||
        current.args.length !== profile.args.length ||
        current.args.some((arg, index) => arg !== profile.args[index])
      ) {
        disconnected.add(profile.id);
      }

      const models = configuration.customModels[profile.id] ?? [];
      const previous = this.#customModels.get(profile.id) ?? [];
      if (!sameModels(models, previous)) {
        this.#customModels.set(profile.id, models);
        const catalog = this.#catalogs[profile.id];
        if (catalog?.status === "ready") {
          const mergedModels = mergeProviderModels(this.#baseModels.get(profile.id) ?? [], models);
          const selectedModelId = mergedModels.some((model) => model.id === catalog.selectedModelId)
            ? catalog.selectedModelId
            : mergedModels[0]?.id;
          this.#catalogs[profile.id] = { ...catalog, models: mergedModels, selectedModelId };
          catalogsToApply.add(profile.id);
        }
        disconnected.add(profile.id);
      }
    }
    for (const profileId of disabledProfiles) {
      if (!this.#disabledProfiles.has(profileId)) disconnected.add(profileId);
    }

    this.#profiles = configuration.profiles;
    this.#disabledProfiles = disabledProfiles;
    for (const profileId of disconnected) this.#disconnectProfile(profileId, threads);
    for (const profileId of catalogsToApply) this.#applyCatalogState(profileId, threads);
    this.#applyFallback(threads);
  }

  runTurn(thread: ThreadState, text: string, images: MessageImage[] = [], content?: PromptPart[]) {
    this.#rememberThread(thread);
    const profileId = thread.profileId;
    const provider = thread.providers[profileId];
    if (this.#disabledProfiles.has(profileId)) return;
    if (!text && images.length === 0) return;
    if (provider.turnStatus === "running") {
      if (!provider.supportsFollowups || provider.connectionStatus !== "ready") return;
      addMessage(thread, "user", text, images, content);
      this.#updates.startTurn(thread.id, profileId);
      return this.#completeFollowUp(thread, profileId, text, images, content);
    }
    if (provider.turnStatus !== "idle") return;
    if (provider.connectionStatus !== "disconnected" && provider.connectionStatus !== "ready")
      return;
    if (this.#catalogs[profileId]?.status !== "ready" || !provider.selectedModelId) return;

    const isFirstPrompt = !thread.messages.some((message) => message.role === "user");
    const turnToken = crypto.randomUUID();
    this.#turnTokens.set(thread.id, turnToken);
    provider.error = undefined;
    provider.errorDetails = undefined;
    if (isFirstPrompt && !text) thread.title = "Image prompt";
    addMessage(thread, "user", text, images, content);
    this.#updates.startTurn(thread.id, profileId);

    return this.#completeTurn(thread, profileId, text, images, content, isFirstPrompt, turnToken);
  }

  activate(thread: ThreadState, profileId: string, announceSwitch = true) {
    this.#rememberThread(thread);
    if (
      profileId === thread.profileId ||
      this.#disabledProfiles.has(profileId) ||
      this.#catalogs[profileId]?.status === "unavailable" ||
      Object.values(thread.providers).some(
        (provider) => provider.turnStatus === "running" || provider.turnStatus === "blocked",
      )
    )
      return;
    const profile = this.#profileById(profileId);
    this.#turnTokens.delete(thread.id);
    thread.profileId = profile.id;
    thread.updatedAt = Date.now();
    this.#updates.clear(thread.id, profile.id);
    if (announceSwitch && thread.messages.length > 0) {
      addMessage(thread, "notice", `${profile.label} is now active.`);
    }
  }

  async discoverAll(cwd: string, threads: ThreadState[]) {
    this.#threads = [...threads];
    this.#applyFallback(threads);
    await Promise.allSettled(
      this.#enabledProfiles().map((profile) => this.discover(profile, cwd, threads)),
    );
  }

  async discover(profile: ProviderDefinition, cwd: string, threads: ThreadState[]) {
    this.#threads = [...threads];
    if (this.#disabledProfiles.has(profile.id)) return;
    this.#catalogs[profile.id] = {
      status: "loading",
      models: [],
      reasoningOptions: [],
    };
    let discovered: AcpModelState = { models: [], reasoningOptions: [] };
    let agentVersion: string | undefined;
    let commands: SlashCommand[] | undefined;
    const connection = new AcpConnection({
      connectionStatus: () => {},
      turnStatus: () => {},
      initialized: (agentInfo) => {
        agentVersion = agentInfo?.version;
      },
      ready: (session) => {
        discovered = session;
      },
      update: (update) => {
        // Commands are published once per session, so discovery is the earliest they exist.
        if (update.sessionUpdate === "available_commands_update") {
          commands = slashCommands(update.availableCommands);
        }
      },
      permission: () => {},
      stderr: () => {},
      error: () => {},
      exited: () => {},
    });

    try {
      const mcpServers = await this.#loadMcpServers(cwd);
      await connection.connect({
        cwd,
        command: profile.command,
        args: profile.args,
        profileId: profile.id,
        mcpServers,
      });
      if (discovered.models.length === 0) {
        throw new Error(`${profile.label} did not advertise any models.`);
      }
      this.#baseModels.set(profile.id, discovered.models);
      const models = mergeProviderModels(
        discovered.models,
        this.#customModels.get(profile.id) ?? [],
      );
      const advertisedModelId = discovered.selectedModelId;
      const selectedModelId =
        advertisedModelId && models.some((model) => model.id === advertisedModelId)
          ? advertisedModelId
          : models[0].id;
      const { reasoningOptionsByModel, fastModeOptionsByModel } = await discoverModelOptions(
        connection,
        { ...discovered, selectedModelId },
      );
      const selectedReasoning = reasoningOptionsByModel[selectedModelId];
      const selectedFastMode = fastModeOptionsByModel[selectedModelId];
      this.#catalogs[profile.id] = {
        status: "ready",
        agentVersion,
        models,
        selectedModelId,
        reasoningOptions: selectedReasoning?.options ?? [],
        selectedReasoningId: selectedReasoning?.selectedId,
        reasoningOptionsByModel,
        fastModeConfigId: selectedFastMode?.configId,
        fastModeModelId: selectedFastMode ? selectedModelId : undefined,
        fastModeOptionsByModel,
        fastModeEnabled: selectedFastMode?.enabled,
        fastModeValueType: selectedFastMode?.valueType,
        fastModeDescription: selectedFastMode?.description,
        selects: discovered.selects,
        supportsFollowups: discovered.supportsFollowups,
        commands,
      };
    } catch (error) {
      this.#baseModels.set(profile.id, []);
      this.#catalogs[profile.id] = {
        status: "unavailable",
        agentVersion,
        models: [],
        reasoningOptions: [],
        unavailableReason: unavailableReason(error),
        error: error instanceof Error ? error.message : String(error),
      };
    } finally {
      try {
        await connection.stop();
      } catch {
        // Discovery sessions are disposable; app shutdown also stops every child.
      }
      this.applyCatalog(profile.id, threads);
    }
  }

  applyCatalog(profileId: string, threads: ThreadState[]) {
    this.#applyCatalogState(profileId, threads);
    this.#applyFallback(threads);
  }

  async connect(thread: ThreadState, profileId: string) {
    this.#rememberThread(thread);
    let stopping: Promise<void> | undefined;
    while ((stopping = this.#stoppingProfiles.get(profileId))) await stopping;
    if (
      !thread.cwd ||
      this.#disabledProfiles.has(profileId) ||
      this.#catalogs[profileId]?.status !== "ready"
    )
      return;
    const profile = this.#profileById(profileId);
    const provider = thread.providers[profile.id];
    const selectedModelId = provider.selectedModelId;
    const requestedReasoningId = provider.selectedReasoningId;
    const requestedFastModeEnabled = provider.fastModeEnabled;
    const requestedSelects = Object.entries(provider.selects ?? {}).map(
      ([select, value]) => [select as SessionSelectId, value.selectedId] as const,
    );
    const existingSessionId = provider.sessionId;
    if (!selectedModelId || !provider.models.some((model) => model.id === selectedModelId)) {
      this.#setError(thread, profile.id, {
        scope: "connection",
        message: `Choose an advertised ${profile.label} model before connecting.`,
      });
      return;
    }

    const key = connectionKey(thread.id, profile.id);
    const token = crypto.randomUUID();
    this.#tokens.set(key, token);
    provider.harnessId = undefined;
    provider.error = undefined;
    provider.errorDetails = undefined;
    this.#setConnectionStatus(thread, profile.id, "connecting");

    if (!(await this.#stopPreviousConnection(key, token, thread, profile.id))) return;

    const isCurrent = () => this.#tokens.get(key) === token;
    let startupComplete = false;
    let connectionReportedError = false;
    let sessionSelectedModelId: string | undefined;
    const connection = this.#createConnection({
      connectionStatus: (status) => {
        if (!isCurrent() || (status === "ready" && !startupComplete)) return;
        this.#setConnectionStatus(thread, profile.id, status);
      },
      turnStatus: (status) => {
        if (!isCurrent()) return;
        this.#setTurnStatus(thread, profile.id, status);
      },
      ready: (session) => {
        if (!isCurrent()) return;
        provider.harnessId = session.harnessId;
        provider.sessionId = session.sessionId;
        provider.goalActions = session.goalActions;
        provider.goalControlMethod = session.goalControlMethod;
        applyProviderConfigState(provider, session);
        if (
          requestedReasoningId &&
          session.reasoningOptions.some((option) => option.id === requestedReasoningId)
        ) {
          provider.selectedReasoningId = requestedReasoningId;
        }
        if (requestedFastModeEnabled !== undefined && session.fastModeConfigId) {
          provider.fastModeEnabled = requestedFastModeEnabled;
        }
        sessionSelectedModelId = session.selectedModelId;
        provider.error = undefined;
        provider.errorDetails = undefined;
      },
      update: (update) => {
        if (!isCurrent()) return;
        const modelState =
          update.sessionUpdate === "config_option_update" ? readModelState(update) : undefined;
        this.#updates.handle(thread, profile.id, update, modelState);
      },
      metadata: (metadata) => {
        if (isCurrent()) this.#updates.handleMetadata(thread, profile.id, metadata);
      },
      permission: (request) => {
        if (isCurrent()) {
          this.#hooks.permission({ threadId: thread.id, profileId: profile.id, request });
        }
      },
      permissionMode: () => this.#permissionMode,
      stderr: () => {},
      error: (error) => {
        if (!isCurrent()) return;
        const diagnosticData = jsonValueSchema.safeParse(error.data).data;
        void recordDiagnostic("error", "acp.client.error", {
          scope: error.scope,
          method: error.method,
          code: error.code,
          message: error.message,
          data: diagnosticData,
          profileId: profile.id,
          threadId: thread.id,
          harnessId: provider.harnessId,
        }).catch(() => {});
        if (error.scope !== "turn") connectionReportedError = true;
        this.#setError(thread, profile.id, error);
        const reason = unavailableReason(error.message);
        if (reason === "authentication" || reason === "missing-executable") {
          this.#markProviderUnavailable(profile, error.message, reason);
        }
        if (thread.profileId === profile.id) addMessage(thread, "error", error.message);
      },
      exited: (code) => {
        if (!isCurrent()) return;
        this.#hooks.clearPermission(thread.id, profile.id);
        const message =
          code === null ? "The harness exited." : `The harness exited with code ${code}.`;
        provider.error = message;
        provider.errorDetails = { scope: "transport", message };
        this.#setConnectionStatus(thread, profile.id, "stopped");
        if (thread.profileId === profile.id) addMessage(thread, "notice", message);
      },
    });

    this.#connections.set(key, connection);
    try {
      const mcpServers = await this.#loadMcpServers(thread.cwd);
      await connection.connect({
        cwd: thread.cwd,
        command: profile.command,
        args: profile.args,
        profileId: profile.id,
        threadId: thread.id,
        sessionId: existingSessionId,
        mcpServers,
      });
      if (!isCurrent()) {
        await connection.stop();
        return;
      }
      if (!provider.modelConfigId) {
        throw new Error(`${profile.label} did not expose a model configuration.`);
      }
      const updated = await connection.setConfigOption(provider.modelConfigId, selectedModelId);
      const appliedModelId = updated.selectedModelId ?? sessionSelectedModelId;
      if (appliedModelId !== selectedModelId) {
        throw new Error(`${profile.label} did not apply model ${selectedModelId}.`);
      }
      applyProviderConfigState(provider, updated);
      provider.selectedModelId = selectedModelId;
      await this.#restoreReasoning(
        connection,
        provider,
        profile.label,
        requestedReasoningId,
        updated.selectedReasoningId,
      );
      await this.#restoreFastMode(connection, provider, requestedFastModeEnabled);
      await this.#restoreSessionSelections(connection, provider, requestedSelects);
      startupComplete = true;
      provider.error = undefined;
      provider.errorDetails = undefined;
      this.#setConnectionStatus(thread, profile.id, "ready");
      return connection;
    } catch (error) {
      if (!isCurrent()) return;
      this.#connections.delete(key);
      if (!connectionReportedError) this.#reportError(thread, profile.id, error);
      try {
        await connection.stop();
      } catch {
        // Preserve the startup error; process cleanup is best-effort here.
      }
    }
  }

  async #stopPreviousConnection(
    key: string,
    token: string,
    thread: ThreadState,
    profileId: string,
  ) {
    const previousConnection = this.#connections.get(key);
    try {
      await previousConnection?.stop();
    } catch (error) {
      if (this.#tokens.get(key) === token) this.#reportError(thread, profileId, error);
      return false;
    } finally {
      if (this.#connections.get(key) === previousConnection) this.#connections.delete(key);
    }
    return this.#tokens.get(key) === token;
  }

  async #restoreReasoning(
    connection: AcpConnection,
    provider: ProviderSessionState,
    profileLabel: string,
    requestedReasoningId: string | undefined,
    appliedReasoningId: string | undefined,
  ) {
    const reasoningId =
      requestedReasoningId &&
      provider.reasoningOptions.some((option) => option.id === requestedReasoningId)
        ? requestedReasoningId
        : provider.selectedReasoningId;
    if (!reasoningId || !provider.reasoningConfigId || appliedReasoningId === reasoningId) return;
    const reasoningState = await connection.setConfigOption(
      provider.reasoningConfigId,
      reasoningId,
    );
    if (reasoningState.selectedReasoningId !== reasoningId) {
      throw new Error(`${profileLabel} did not apply reasoning variant ${reasoningId}.`);
    }
    applyProviderConfigState(provider, reasoningState);
  }

  async #restoreFastMode(
    connection: AcpConnection,
    provider: ProviderSessionState,
    requestedFastModeEnabled: boolean | undefined,
  ) {
    const enabled = requestedFastModeEnabled ?? provider.fastModeEnabled;
    if (enabled === undefined || !provider.fastModeConfigId || provider.fastModeEnabled === enabled)
      return;
    const fastModeState = await connection.setFastModeConfigOption(
      provider.fastModeConfigId,
      enabled,
      provider.fastModeValueType,
    );
    applyProviderConfigState(provider, fastModeState);
  }

  async #restoreSessionSelections(
    connection: AcpConnection,
    provider: ProviderSessionState,
    requested: readonly (readonly [SessionSelectId, string | undefined])[],
  ) {
    for (const [select, value] of requested) {
      const spec = provider.selects?.[select];
      if (!value || !spec || spec.selectedId === value) continue;
      if (!spec.options.some((option) => option.id === value)) continue;
      applyProviderConfigState(provider, await connection.setConfigOption(spec.configId, value));
      if (provider.selects?.[select]?.selectedId !== value) {
        throw new Error(`The agent did not apply ${sessionSelectLabels[select]} ${value}.`);
      }
    }
  }

  async #completeTurn(
    thread: ThreadState,
    profileId: string,
    text: string,
    images: MessageImage[],
    content: PromptPart[] | undefined,
    isFirstPrompt: boolean,
    turnToken: string,
  ) {
    let connection = this.connection(thread.id, profileId);
    if (thread.providers[profileId].connectionStatus === "disconnected") {
      connection = await this.connect(thread, profileId);
    }
    const provider = thread.providers[profileId];
    if (
      this.#turnTokens.get(thread.id) !== turnToken ||
      thread.profileId !== profileId ||
      provider.connectionStatus !== "ready" ||
      provider.turnStatus !== "idle" ||
      !connection
    ) {
      if (this.#turnTokens.get(thread.id) === turnToken) this.#turnTokens.delete(thread.id);
      return;
    }

    const turnCompletion = connection.prompt(acpPrompt(content, text, images));
    const titleCompletion =
      isFirstPrompt && text ? this.#generateThreadTitle(thread, text) : undefined;
    try {
      await turnCompletion;
    } catch {
      // The connection callback already added a contextual error to the timeline.
    } finally {
      if (this.#turnTokens.get(thread.id) === turnToken) this.#turnTokens.delete(thread.id);
    }
    await titleCompletion;
  }

  async #completeFollowUp(
    thread: ThreadState,
    profileId: string,
    text: string,
    images: MessageImage[],
    content?: PromptPart[],
  ) {
    try {
      const connection = this.connection(thread.id, profileId);
      if (!connection) throw new Error(`${this.#profileById(profileId).label} is not connected`);
      await connection.followUp(acpPrompt(content, text, images));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      addMessage(thread, "error", `Could not send follow-up: ${message}`);
    }
  }

  async #generateThreadTitle(thread: ThreadState, request: string) {
    const token = crypto.randomUUID();
    this.#titleTokens.set(thread.id, token);
    const selection = this.#titlePreference
      ? titleGenerationSelection(this.#titlePreference, this.#enabledProfiles(), this.#catalogs)
      : undefined;
    let title: string | undefined;
    let connection: AcpConnection | undefined;
    try {
      if (selection) {
        connection = this.#createConnection({
          connectionStatus: () => {},
          turnStatus: () => {},
          ready: () => {},
          update: () => {},
          permission: () => {},
          stderr: () => {},
          error: () => {},
          exited: () => {},
        });
        this.#titleConnections.set(thread.id, connection);
        await connection.connect({
          cwd: thread.cwd,
          command: selection.profile.command,
          args: selection.profile.args,
          profileId: selection.profile.id,
        });
        title = normalizeThreadTitle(
          await connection.generateTitle(
            thread.cwd,
            buildThreadTitlePrompt(request),
            selection.model.id,
          ),
        );
      }
    } catch {
      // Title generation is isolated from the active turn and falls back locally.
    } finally {
      if (connection && this.#titleConnections.get(thread.id) === connection) {
        this.#titleConnections.delete(thread.id);
      }
      try {
        await connection?.stop();
      } catch {
        // Preserve the generated title or local fallback when cleanup fails.
      }
    }
    if (this.#titleTokens.get(thread.id) !== token) return;
    this.#titleTokens.delete(thread.id);
    thread.title = title ?? titleFromPrompt(request);
    thread.updatedAt = nextTimestamp(thread);
  }

  async selectModel(thread: ThreadState, profileId: string, modelId: string) {
    const provider = thread.providers[profileId];
    const catalog = this.#catalogs[profileId];
    if (
      !catalog ||
      catalog.status === "unavailable" ||
      (catalog.status === "ready" && !catalog.models.some((model) => model.id === modelId))
    )
      return;
    const previousModelId = provider.selectedModelId;
    provider.selectedModelId = modelId;
    provider.error = undefined;
    if (
      provider.connectionStatus !== "ready" ||
      provider.turnStatus !== "idle" ||
      !provider.modelConfigId
    ) {
      applyReasoningForSelectedModel(provider);
      applyFastModeForSelectedModel(provider);
      return;
    }
    try {
      const connection = this.connection(thread.id, profileId);
      if (!connection) throw new Error(`${this.#profileById(profileId).label} is not connected`);
      const updated = await connection.setConfigOption(provider.modelConfigId, modelId);
      applyProviderConfigState(provider, updated);
      provider.selectedModelId = updated.selectedModelId ?? modelId;
    } catch (error) {
      provider.selectedModelId = previousModelId;
      const message = error instanceof Error ? error.message : String(error);
      provider.error = message;
      if (thread.profileId === profileId)
        addMessage(thread, "error", `Could not change model: ${message}`);
    }
  }

  async selectReasoning(thread: ThreadState, reasoningId: string) {
    const provider = thread.providers[thread.profileId];
    if (!provider.reasoningOptions.some((option) => option.id === reasoningId)) return;
    const previousReasoningId = provider.selectedReasoningId;
    provider.selectedReasoningId = reasoningId;
    provider.error = undefined;
    if (
      provider.connectionStatus !== "ready" ||
      provider.turnStatus !== "idle" ||
      !provider.reasoningConfigId
    )
      return;
    try {
      const connection = this.connection(thread.id, thread.profileId);
      if (!connection) {
        throw new Error(`${this.#profileById(thread.profileId).label} is not connected`);
      }
      const updated = await connection.setConfigOption(provider.reasoningConfigId, reasoningId);
      if (updated.selectedReasoningId !== reasoningId) {
        throw new Error(
          `${this.#profileById(thread.profileId).label} did not apply reasoning variant ${reasoningId}.`,
        );
      }
      applyProviderConfigState(provider, updated);
    } catch (error) {
      provider.selectedReasoningId = previousReasoningId;
      const message = error instanceof Error ? error.message : String(error);
      provider.error = message;
      addMessage(thread, "error", `Could not change reasoning variant: ${message}`);
    }
  }

  async selectFastMode(thread: ThreadState, enabled: boolean) {
    const provider = thread.providers[thread.profileId];
    if (!provider.fastModeConfigId) return;
    const previousFastModeEnabled = provider.fastModeEnabled;
    provider.fastModeEnabled = enabled;
    provider.error = undefined;
    if (provider.connectionStatus !== "ready" || provider.turnStatus !== "idle") return;
    try {
      const connection = this.connection(thread.id, thread.profileId);
      if (!connection) {
        throw new Error(`${this.#profileById(thread.profileId).label} is not connected`);
      }
      const updated = await connection.setFastModeConfigOption(
        provider.fastModeConfigId,
        enabled,
        provider.fastModeValueType,
      );
      if (updated.fastModeEnabled !== enabled) {
        throw new Error(`${this.#profileById(thread.profileId).label} did not apply Fast mode.`);
      }
      applyProviderConfigState(provider, updated);
    } catch (error) {
      provider.fastModeEnabled = previousFastModeEnabled;
      const message = error instanceof Error ? error.message : String(error);
      provider.error = message;
      addMessage(thread, "error", `Could not change Fast mode: ${message}`);
    }
  }

  async selectSessionOption(thread: ThreadState, select: SessionSelectId, valueId: string) {
    const provider = thread.providers[thread.profileId];
    const spec = provider.selects?.[select];
    if (!spec || spec.selectedId === valueId) return;
    if (!spec.options.some((option) => option.id === valueId)) return;
    const previous = spec.selectedId;
    setSelected(provider, select, valueId);
    provider.error = undefined;
    if (provider.connectionStatus !== "ready" || provider.turnStatus !== "idle") return;
    try {
      const connection = this.connection(thread.id, thread.profileId);
      if (!connection) {
        throw new Error(`${this.#profileById(thread.profileId).label} is not connected`);
      }
      applyProviderConfigState(provider, await connection.setConfigOption(spec.configId, valueId));
    } catch (error) {
      setSelected(provider, select, previous);
      const message = error instanceof Error ? error.message : String(error);
      provider.error = message;
      addMessage(thread, "error", `Could not change ${sessionSelectLabels[select]}: ${message}`);
    }
  }

  async removeThread(threadId: string) {
    this.#threads = this.#threads.filter((thread) => thread.id !== threadId);
    this.#turnTokens.delete(threadId);
    this.#titleTokens.delete(threadId);
    const titleConnection = this.#titleConnections.get(threadId);
    this.#titleConnections.delete(threadId);
    const threadConnections = this.#profiles.flatMap((profile) => {
      const key = connectionKey(threadId, profile.id);
      this.#tokens.delete(key);
      const connection = this.#connections.get(key);
      this.#connections.delete(key);
      this.#updates.clear(threadId, profile.id);
      return connection ? [connection] : [];
    });
    await Promise.allSettled(
      [...threadConnections, ...(titleConnection ? [titleConnection] : [])].map((connection) =>
        connection.stop(),
      ),
    );
  }

  cancel(thread: ThreadState) {
    return this.connection(thread.id, thread.profileId)?.cancel();
  }

  async controlGoal(
    thread: ThreadState,
    action: import("../types/index.ts").GoalAction,
    objective?: string,
  ) {
    const provider = thread.providers[thread.profileId];
    if (!provider.goalActions?.includes(action)) return;
    const connection = this.connection(thread.id, thread.profileId);
    if (!connection) throw new Error("Connect the provider before changing its goal");
    await connection.goal(action, objective?.trim(), provider.goalControlMethod);
  }

  async retryLastTurn(thread: ThreadState) {
    const prompt = [...thread.messages].reverse().find((message) => message.role === "user");
    if (!prompt) return;
    const provider = thread.providers[thread.profileId];
    if (provider.connectionStatus !== "ready") await this.connect(thread, thread.profileId);
    return this.runTurn(thread, prompt.text, prompt.images, prompt.content);
  }

  async newSession(thread: ThreadState) {
    const profileId = thread.profileId;
    const key = connectionKey(thread.id, profileId);
    const connection = this.#connections.get(key);
    this.#tokens.delete(key);
    this.#connections.delete(key);
    await connection?.stop();
    const provider = thread.providers[profileId];
    provider.sessionId = undefined;
    provider.harnessId = undefined;
    provider.connectionStatus = "disconnected";
    provider.turnStatus = "idle";
    provider.goal = null;
    return this.connect(thread, profileId);
  }

  answerPermission(
    threadId: string,
    profileId: string,
    requestId: string | number | null,
    optionId?: string,
  ) {
    const connection = this.connection(threadId, profileId);
    if (!connection) return;
    if (optionId) connection.answerPermission(requestId, optionId);
    else connection.cancelPermission(requestId);
  }

  answerQuestion(
    threadId: string,
    profileId: string,
    requestId: string | number | null,
    answer: QuestionAnswer,
  ) {
    this.connection(threadId, profileId)?.answerQuestion(requestId, answer);
  }

  #rememberThread(thread: ThreadState) {
    if (!this.#threads.some((item) => item.id === thread.id)) this.#threads.push(thread);
  }

  #profileById(profileId: string) {
    return this.#profiles.find((profile) => profile.id === profileId) ?? this.#profiles[0];
  }

  #enabledProfiles() {
    return this.#profiles.filter((profile) => !this.#disabledProfiles.has(profile.id));
  }

  #applyCatalogState(profileId: string, threads: ThreadState[]) {
    const catalog = this.#catalogs[profileId];
    if (!catalog) return;
    for (const thread of threads) {
      const provider = thread.providers[profileId];
      if (!provider) continue;
      if (catalog.status === "ready") {
        provider.models = catalog.models;
        if (
          !provider.selectedModelId ||
          !catalog.models.some((model) => model.id === provider.selectedModelId)
        ) {
          provider.selectedModelId = catalog.selectedModelId;
        }
        provider.reasoningOptionsByModel = catalog.reasoningOptionsByModel;
        applyReasoningForSelectedModel(provider);
        provider.fastModeOptionsByModel = catalog.fastModeOptionsByModel;
        applyFastModeForSelectedModel(provider);
        applySessionSelects(provider, catalog);
        provider.commands ??= catalog.commands;
      } else if (catalog.status === "unavailable") {
        provider.models = [];
        provider.selectedModelId = undefined;
      }
    }
  }

  #applyFallback(threads: ThreadState[]) {
    const fallbackProfileId = firstReadyProviderId(this.#enabledProfiles(), this.#catalogs);
    if (!fallbackProfileId) return;
    for (const thread of threads) {
      if (
        this.#disabledProfiles.has(thread.profileId) ||
        this.#catalogs[thread.profileId]?.status === "unavailable"
      ) {
        this.activate(thread, fallbackProfileId, false);
      }
    }
  }

  #disconnectProfile(profileId: string, threads: ThreadState[]) {
    const connections: AcpConnection[] = [];
    for (const thread of threads) {
      const key = connectionKey(thread.id, profileId);
      this.#tokens.delete(key);
      const connection = this.#connections.get(key);
      this.#connections.delete(key);
      if (connection) connections.push(connection);
      const provider = thread.providers[profileId];
      if (provider) {
        provider.connectionStatus = "disconnected";
        provider.turnStatus = "idle";
      }
      this.#hooks.clearPermission(thread.id, profileId);
    }
    this.#trackStops(profileId, connections);
  }

  #trackStops(profileId: string, connections: AcpConnection[]) {
    if (connections.length === 0) return;
    const previous = this.#stoppingProfiles.get(profileId);
    const stopping = Promise.allSettled([
      ...(previous ? [previous] : []),
      ...connections.map((connection) => connection.stop()),
    ]).then(() => {});
    this.#stoppingProfiles.set(profileId, stopping);
    void stopping.then(() => {
      if (this.#stoppingProfiles.get(profileId) === stopping) {
        this.#stoppingProfiles.delete(profileId);
      }
    });
  }

  #markProviderUnavailable(
    profile: ProviderDefinition,
    message: string,
    reason: "authentication" | "missing-executable",
  ) {
    this.#catalogs[profile.id] = {
      status: "unavailable",
      models: [],
      reasoningOptions: [],
      unavailableReason: reason,
      error: message,
    };
    const connections: AcpConnection[] = [];
    for (const thread of this.#threads) {
      const key = connectionKey(thread.id, profile.id);
      this.#tokens.delete(key);
      const connection = this.#connections.get(key);
      this.#connections.delete(key);
      if (connection) connections.push(connection);
    }
    this.#trackStops(profile.id, connections);
    this.applyCatalog(profile.id, this.#threads);
  }

  #setConnectionStatus(thread: ThreadState, profileId: string, status: ConnectionStatus) {
    const provider = thread.providers[profileId];
    const previousStatus = provider.connectionStatus;
    provider.connectionStatus = status;
    void recordDiagnostic("info", "acp.connection.status_changed", {
      threadId: thread.id,
      profileId,
      harnessId: provider.harnessId,
      previousStatus,
      status,
    }).catch(() => {});
    if (thread.profileId === profileId) thread.updatedAt = Date.now();
  }

  #setTurnStatus(
    thread: ThreadState,
    profileId: string,
    status: ProviderSessionState["turnStatus"],
  ) {
    const provider = thread.providers[profileId];
    const previousStatus = provider.turnStatus;
    provider.turnStatus = status;
    void recordDiagnostic("info", "acp.turn.status_changed", {
      threadId: thread.id,
      profileId,
      harnessId: provider.harnessId,
      previousStatus,
      status,
    }).catch(() => {});
    if (status === "idle") {
      provider.error = undefined;
      provider.errorDetails = undefined;
    }
    if (thread.profileId === profileId) thread.updatedAt = Date.now();
  }

  #setError(thread: ThreadState, profileId: string, error: AcpErrorDetails) {
    const provider = thread.providers[profileId];
    if (error.scope === "turn")
      provider.turnStatus = provider.turnStatus === "blocked" ? "blocked" : "failed";
    else provider.connectionStatus = "error";
    provider.error = error.message;
    provider.errorDetails = error;
    if (thread.profileId === profileId) thread.updatedAt = Date.now();
  }

  #reportError(thread: ThreadState, profileId: string, cause: unknown) {
    const details: AcpErrorDetails = {
      scope: "connection",
      message: cause instanceof Error ? cause.message : String(cause),
    };
    this.#setError(thread, profileId, details);
    if (thread.profileId === profileId) addMessage(thread, "error", details.message);
  }
}

export function mergeProviderModels(
  advertised: ProviderModelCatalog["models"],
  custom: ProviderModelCatalog["models"],
) {
  return [...new Map([...advertised, ...custom].map((model) => [model.id, model])).values()];
}

function sameModels(left: ProviderModelCatalog["models"], right: ProviderModelCatalog["models"]) {
  return (
    left.length === right.length &&
    left.every((model, index) => model.id === right[index]?.id && model.name === right[index]?.name)
  );
}

function acpPrompt(content: PromptPart[] | undefined, text: string, images: MessageImage[]) {
  const prompt: PromptContent[] = (content ?? (text ? [{ type: "text", text }] : [])).map((part) =>
    part.type === "text"
      ? { type: "text", text: part.text }
      : {
          type: "resource_link",
          uri: part.reference.uri,
          name: part.reference.name,
          title: part.reference.relativePath,
        },
  );
  prompt.push(...images.map(({ data, mimeType }) => ({ type: "image" as const, data, mimeType })));
  return prompt;
}

function connectionKey(threadId: string, profileId: string) {
  return `${threadId}:${profileId}`;
}

export function applyProviderConfigState(provider: ProviderSessionState, state: AcpModelState) {
  if (state.supportsFollowups !== undefined) provider.supportsFollowups = state.supportsFollowups;
  if (state.modelConfigId) provider.modelConfigId = state.modelConfigId;
  if (state.models.length > 0) provider.models = state.models;
  if (state.selectedModelId) provider.selectedModelId = state.selectedModelId;
  provider.reasoningConfigId = state.reasoningConfigId;
  provider.reasoningOptions = state.reasoningOptions;
  provider.selectedReasoningId = state.selectedReasoningId;
  const modelId = state.selectedModelId ?? provider.selectedModelId;
  if (modelId) {
    provider.reasoningOptionsByModel = {
      ...provider.reasoningOptionsByModel,
      [modelId]: {
        options: state.reasoningOptions,
        selectedId: state.selectedReasoningId,
      },
    };
  }
  if (modelId) {
    const fastModeOptionsByModel = { ...provider.fastModeOptionsByModel };
    if (state.fastModeConfigId && state.fastModeEnabled !== undefined) {
      fastModeOptionsByModel[modelId] = {
        configId: state.fastModeConfigId,
        enabled: state.fastModeEnabled,
        valueType: state.fastModeValueType,
        description: state.fastModeDescription,
      };
    } else {
      delete fastModeOptionsByModel[modelId];
    }
    provider.fastModeOptionsByModel = fastModeOptionsByModel;
  }
  provider.fastModeConfigId = state.fastModeConfigId;
  provider.fastModeModelId = state.fastModeConfigId ? modelId : undefined;
  provider.fastModeEnabled = state.fastModeEnabled;
  provider.fastModeValueType = state.fastModeValueType;
  provider.fastModeDescription = state.fastModeDescription;
  applySessionSelects(provider, state);
}

function applySessionSelects(provider: ProviderSessionState, state: AcpModelState) {
  if (state.selects) provider.selects = { ...provider.selects, ...state.selects };
}

/** Replaces the select rather than mutating it, so catalog-shared objects never alias. */
function setSelected(
  provider: ProviderSessionState,
  select: SessionSelectId,
  valueId: string | undefined,
) {
  const current = provider.selects?.[select];
  if (current)
    provider.selects = {
      ...provider.selects,
      [select]: { ...current, selectedId: valueId },
    };
}
