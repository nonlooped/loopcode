import type { SessionUpdate } from "@agentclientprotocol/sdk";

import { profileById, profiles } from "../config/providers.ts";
import type {
  HarnessProfile,
  PermissionRequest,
  ProviderModelCatalog,
  ProviderSessionState,
  ThreadState,
} from "../types/index.ts";
import { applyFastModeForSelectedModel } from "../utils/fast-mode.ts";
import { addMessage } from "../utils/messages.ts";
import { discoverModelOptions } from "../utils/model-options.ts";
import { applyReasoningForSelectedModel } from "../utils/reasoning-options.ts";
import { AcpConnection, readModelState, type AcpModelState } from "./acp.ts";
import { SessionUpdateHandler } from "./session-updates.ts";

interface RuntimeHooks {
  permission: (value: { threadId: string; profileId: string; request: PermissionRequest }) => void;
  clearPermission: (threadId: string, profileId?: string) => void;
}

export class ProviderRuntime {
  #catalogs: Record<string, ProviderModelCatalog>;
  #hooks: RuntimeHooks;
  #connections = new Map<string, AcpConnection>();
  #tokens = new Map<string, string>();
  #updates = new SessionUpdateHandler(applyProviderConfigState);

  constructor(catalogs: Record<string, ProviderModelCatalog>, hooks: RuntimeHooks) {
    this.#catalogs = catalogs;
    this.#hooks = hooks;
  }

  connection(threadId: string, profileId: string) {
    return this.#connections.get(connectionKey(threadId, profileId));
  }

  startTurn(threadId: string, profileId: string) {
    this.#updates.startTurn(threadId, profileId);
  }

  activate(thread: ThreadState, profileId: string, announceSwitch = true) {
    if (profileId === thread.profileId) return;
    const profile = profileById(profileId);
    const previousProfileId = thread.profileId;
    this.#hooks.clearPermission(thread.id, previousProfileId);
    thread.profileId = profile.id;
    thread.updatedAt = Date.now();
    this.#updates.clear(thread.id, profile.id);
    if (announceSwitch && thread.messages.length > 0) {
      addMessage(thread, "notice", `${profile.label} is now active.`);
    }
  }

  async discoverAll(cwd: string, threads: ThreadState[]) {
    await Promise.allSettled(profiles.map((profile) => this.discover(profile, cwd, threads)));
  }

  async discover(profile: HarnessProfile, cwd: string, threads: ThreadState[]) {
    this.#catalogs[profile.id] = { status: "loading", models: [], reasoningOptions: [] };
    let discovered: AcpModelState = { models: [], reasoningOptions: [] };
    const connection = new AcpConnection({
      status: () => {},
      ready: (session) => {
        discovered = session;
      },
      update: () => {},
      permission: () => {},
      stderr: () => {},
      error: () => {},
      exited: () => {},
    });

    try {
      await connection.connect({ cwd, command: profile.command, args: profile.args });
      if (discovered.models.length === 0) {
        throw new Error(`${profile.label} did not advertise any models.`);
      }
      const advertisedModelId = discovered.selectedModelId;
      const selectedModelId =
        advertisedModelId && discovered.models.some((model) => model.id === advertisedModelId)
          ? advertisedModelId
          : discovered.models[0].id;
      const { reasoningOptionsByModel, fastModeOptionsByModel } = await discoverModelOptions(
        connection,
        discovered,
      );
      const selectedReasoning = reasoningOptionsByModel[selectedModelId];
      const selectedFastMode = fastModeOptionsByModel[selectedModelId];
      this.#catalogs[profile.id] = {
        status: "ready",
        models: discovered.models,
        selectedModelId,
        reasoningOptions: selectedReasoning?.options ?? [],
        selectedReasoningId: selectedReasoning?.selectedId,
        reasoningOptionsByModel,
        fastModeConfigId: selectedFastMode?.configId,
        fastModeModelId: selectedFastMode ? selectedModelId : undefined,
        fastModeOptionsByModel,
        fastModeEnabled: selectedFastMode?.enabled,
        fastModeDescription: selectedFastMode?.description,
      };
    } catch (error) {
      this.#catalogs[profile.id] = {
        status: "error",
        models: [],
        reasoningOptions: [],
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
    const catalog = this.#catalogs[profileId];
    if (!catalog || catalog.status !== "ready") return;
    for (const thread of threads) {
      const provider = thread.providers[profileId];
      if (!provider) continue;
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
    }
  }

  async connect(thread: ThreadState, profileId: string) {
    if (!thread.cwd) return;
    const profile = profileById(profileId);
    const provider = thread.providers[profile.id];
    const selectedModelId = provider.selectedModelId;
    const requestedReasoningId = provider.selectedReasoningId;
    const requestedFastModeEnabled = provider.fastModeEnabled;
    const existingSessionId = provider.sessionId;
    if (!selectedModelId || !provider.models.some((model) => model.id === selectedModelId)) {
      this.#setError(
        thread,
        profile.id,
        `Choose an advertised ${profile.label} model before connecting.`,
      );
      return;
    }

    const key = connectionKey(thread.id, profile.id);
    const token = crypto.randomUUID();
    this.#tokens.set(key, token);
    provider.harnessId = undefined;
    provider.error = undefined;
    this.#setStatus(thread, profile.id, "connecting");

    const previousConnection = this.#connections.get(key);
    try {
      await previousConnection?.stop();
    } catch (error) {
      if (this.#tokens.get(key) !== token) return;
      this.#reportError(thread, profile.id, error);
      return;
    }
    if (this.#connections.get(key) === previousConnection) this.#connections.delete(key);
    if (this.#tokens.get(key) !== token) return;

    const isCurrent = () => this.#tokens.get(key) === token;
    let startupComplete = false;
    let connectionReportedError = false;
    let sessionSelectedModelId: string | undefined;
    const connection = new AcpConnection({
      status: (status) => {
        if (!isCurrent() || (status === "ready" && !startupComplete)) return;
        this.#setStatus(thread, profile.id, status);
      },
      ready: (session) => {
        if (!isCurrent()) return;
        provider.harnessId = session.harnessId;
        provider.sessionId = session.sessionId;
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
      },
      update: (update) => {
        if (!isCurrent()) return;
        const modelState =
          update.sessionUpdate === "config_option_update" ? readModelState(update) : undefined;
        this.#updates.handle(thread, profile.id, update, modelState);
      },
      permission: (request) => {
        if (isCurrent()) {
          this.#hooks.permission({ threadId: thread.id, profileId: profile.id, request });
        }
      },
      stderr: () => {},
      error: (message) => {
        if (!isCurrent()) return;
        connectionReportedError = true;
        this.#setError(thread, profile.id, message);
        if (thread.profileId === profile.id) addMessage(thread, "error", message);
      },
      exited: (code) => {
        if (!isCurrent()) return;
        const message =
          code === null ? "The harness exited." : `The harness exited with code ${code}.`;
        provider.error = message;
        this.#setStatus(thread, profile.id, "stopped");
        if (thread.profileId === profile.id) addMessage(thread, "notice", message);
      },
    });

    this.#connections.set(key, connection);
    try {
      await connection.connect({
        cwd: thread.cwd,
        command: profile.command,
        args: profile.args,
        sessionId: existingSessionId,
      });
      if (!isCurrent()) {
        await connection.stop();
        return;
      }
      if (!provider.modelConfigId) {
        throw new Error(`${profile.label} did not expose a model configuration.`);
      }
      const updated = await connection.setModel(provider.modelConfigId, selectedModelId);
      const appliedModelId = updated.selectedModelId ?? sessionSelectedModelId;
      if (appliedModelId !== selectedModelId) {
        throw new Error(`${profile.label} did not apply model ${selectedModelId}.`);
      }
      applyProviderConfigState(provider, updated);
      provider.selectedModelId = selectedModelId;
      const reasoningId =
        requestedReasoningId &&
        provider.reasoningOptions.some((option) => option.id === requestedReasoningId)
          ? requestedReasoningId
          : provider.selectedReasoningId;
      if (
        reasoningId &&
        provider.reasoningConfigId &&
        updated.selectedReasoningId !== reasoningId
      ) {
        const reasoningState = await connection.setConfigOption(
          provider.reasoningConfigId,
          reasoningId,
        );
        if (reasoningState.selectedReasoningId !== reasoningId) {
          throw new Error(`${profile.label} did not apply reasoning variant ${reasoningId}.`);
        }
        applyProviderConfigState(provider, reasoningState);
      }
      const fastModeEnabled = requestedFastModeEnabled ?? provider.fastModeEnabled;
      if (
        fastModeEnabled !== undefined &&
        provider.fastModeConfigId &&
        provider.fastModeEnabled !== fastModeEnabled
      ) {
        const fastModeState = await connection.setBooleanConfigOption(
          provider.fastModeConfigId,
          fastModeEnabled,
        );
        applyProviderConfigState(provider, fastModeState);
      }
      startupComplete = true;
      provider.error = undefined;
      this.#setStatus(thread, profile.id, "ready");
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

  async selectModel(thread: ThreadState, profileId: string, modelId: string) {
    const provider = thread.providers[profileId];
    const previousModelId = provider.selectedModelId;
    provider.selectedModelId = modelId;
    provider.error = undefined;
    if (provider.status !== "ready" || !provider.modelConfigId) {
      applyReasoningForSelectedModel(provider);
      applyFastModeForSelectedModel(provider);
      return;
    }
    try {
      const connection = this.connection(thread.id, profileId);
      if (!connection) throw new Error(`${profileById(profileId).label} is not connected`);
      const updated = await connection.setModel(provider.modelConfigId, modelId);
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
    if (provider.status !== "ready" || !provider.reasoningConfigId) return;
    try {
      const connection = this.connection(thread.id, thread.profileId);
      if (!connection) throw new Error(`${profileById(thread.profileId).label} is not connected`);
      const updated = await connection.setConfigOption(provider.reasoningConfigId, reasoningId);
      if (updated.selectedReasoningId !== reasoningId) {
        throw new Error(
          `${profileById(thread.profileId).label} did not apply reasoning variant ${reasoningId}.`,
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
    if (provider.status !== "ready") return;
    try {
      const connection = this.connection(thread.id, thread.profileId);
      if (!connection) throw new Error(`${profileById(thread.profileId).label} is not connected`);
      const updated = await connection.setBooleanConfigOption(provider.fastModeConfigId, enabled);
      if (updated.fastModeEnabled !== enabled) {
        throw new Error(`${profileById(thread.profileId).label} did not apply Fast mode.`);
      }
      applyProviderConfigState(provider, updated);
    } catch (error) {
      provider.fastModeEnabled = previousFastModeEnabled;
      const message = error instanceof Error ? error.message : String(error);
      provider.error = message;
      addMessage(thread, "error", `Could not change Fast mode: ${message}`);
    }
  }

  async removeThread(threadId: string) {
    const threadConnections = profiles.flatMap((profile) => {
      const key = connectionKey(threadId, profile.id);
      this.#tokens.delete(key);
      const connection = this.#connections.get(key);
      this.#connections.delete(key);
      this.#updates.clear(threadId, profile.id);
      return connection ? [connection] : [];
    });
    await Promise.allSettled(threadConnections.map((connection) => connection.stop()));
  }

  cancel(thread: ThreadState) {
    return this.connection(thread.id, thread.profileId)?.cancel();
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

  #setStatus(thread: ThreadState, profileId: string, status: ProviderSessionState["status"]) {
    thread.providers[profileId].status = status;
    if (thread.profileId === profileId) thread.updatedAt = Date.now();
  }

  #setError(thread: ThreadState, profileId: string, message: string) {
    const provider = thread.providers[profileId];
    provider.status = "error";
    provider.error = message;
    if (thread.profileId === profileId) thread.updatedAt = Date.now();
  }

  #reportError(thread: ThreadState, profileId: string, error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    this.#setError(thread, profileId, message);
    if (thread.profileId === profileId) addMessage(thread, "error", message);
  }
}

function connectionKey(threadId: string, profileId: string) {
  return `${threadId}:${profileId}`;
}

export function applyProviderConfigState(provider: ProviderSessionState, state: AcpModelState) {
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
  provider.fastModeDescription = state.fastModeDescription;
}

export type { SessionUpdate };
