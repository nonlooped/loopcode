import * as acp from "@agentclientprotocol/sdk";

import { launchHarness, sendRpc, stopHarness, type BrokerEvent } from "./native.ts";
import type { ConnectRequest, PermissionRequest, ThreadStatus } from "../types/index.ts";
import { readModelState, type AcpModelState } from "../utils/model-state.ts";

type RpcId = acp.JsonRpcId;

type PermissionResponse = acp.RequestPermissionResponse;

interface PendingPermission {
  resolve: (response: PermissionResponse) => void;
}

export interface AcpTransport {
  launch: (request: ConnectRequest, onEvent: (event: BrokerEvent) => void) => Promise<string>;
  send: (harnessId: string, message: acp.AnyMessage) => Promise<void>;
  stop: (harnessId: string) => Promise<void>;
}

const nativeTransport: AcpTransport = {
  launch: launchHarness,
  send: sendRpc,
  stop: stopHarness,
};

export type PromptImage = acp.ImageContent & { type: "image" };

export interface AcpCallbacks {
  status: (status: ThreadStatus) => void;
  ready: (session: AcpSessionInfo) => void;
  update: (update: acp.SessionUpdate) => void;
  permission: (request: PermissionRequest) => void;
  stderr: (line: string) => void;
  error: (message: string) => void;
  exited: (code: number | null) => void;
}

export interface AcpSessionInfo extends AcpModelState {
  harnessId: string;
  sessionId: string;
}

export class AcpConnection {
  #callbacks: AcpCallbacks;
  #transport: AcpTransport;
  #harnessId?: string;
  #sessionId?: string;
  #context?: acp.ClientContext;
  #connection?: acp.ClientConnection;
  #incoming?: ReadableStreamDefaultController<acp.AnyMessage>;
  #incomingClosed = false;
  #titleSessions = new Map<string, string[]>();
  #permissions = new Map<RpcId, PendingPermission>();
  #stopping = false;

  constructor(callbacks: AcpCallbacks, transport: AcpTransport = nativeTransport) {
    this.#callbacks = callbacks;
    this.#transport = transport;
  }

  get harnessId() {
    return this.#harnessId;
  }

  async connect(request: ConnectRequest) {
    this.#callbacks.status("connecting");
    try {
      const readable = new ReadableStream<acp.AnyMessage>({
        start: (controller) => {
          this.#incoming = controller;
        },
      });

      this.#harnessId = await this.#transport.launch(
        { command: request.command, args: request.args, cwd: request.cwd },
        (event) => this.#receiveBrokerEvent(event),
      );
      const harnessId = this.#harnessId;
      const writable = new WritableStream<acp.AnyMessage>({
        write: (message) => this.#transport.send(harnessId, message),
      });
      const client = acp
        .client({ name: "loopcode" })
        .onNotification(acp.methods.client.session.update, ({ params }) => {
          this.#receiveSessionUpdate(params);
        })
        .onRequest(acp.methods.client.session.requestPermission, ({ params, requestId }) =>
          this.#requestPermission(requestId, params),
        );

      this.#connection = client.connect({ readable, writable });
      this.#context = this.#connection.agent;
      const initialized = await this.#context.request(acp.methods.agent.initialize, {
        protocolVersion: acp.PROTOCOL_VERSION,
        clientCapabilities: {
          session: { configOptions: {} },
        },
        clientInfo: {
          name: "loopcode",
          title: "LoopCode",
          version: "0.1.0",
        },
      });

      if (initialized.authMethods?.length) {
        this.#callbacks.stderr(
          "This harness advertises an authentication flow. LoopCode currently expects its CLI to be signed in already.",
        );
      }

      const session = await this.#context.request(acp.methods.agent.session.new, {
        cwd: request.cwd,
        mcpServers: [],
      });
      this.#sessionId = session.sessionId;
      this.#callbacks.ready({
        harnessId,
        sessionId: session.sessionId,
        ...readModelState(session),
      });
      this.#callbacks.status("ready");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.#callbacks.error(message);
      this.#callbacks.status("error");
      await this.stop();
      throw error;
    }
  }

  async prompt(text: string, images: PromptImage[] = []) {
    const context = this.#requireContext();
    const sessionId = this.#requireSessionId();
    this.#callbacks.status("running");
    try {
      await context.request(acp.methods.agent.session.prompt, {
        sessionId,
        prompt: [...(text ? [{ type: "text" as const, text }] : []), ...images],
      });
      this.#callbacks.status("ready");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.#callbacks.error(message);
      this.#callbacks.status("error");
      throw error;
    }
  }

  async cancel() {
    if (!this.#sessionId || !this.#context) return;
    await this.#context.notify(acp.methods.agent.session.cancel, {
      sessionId: this.#sessionId,
    });
    this.#callbacks.status("ready");
  }

  async setModel(configId: string, modelId: string) {
    return this.setConfigOption(configId, modelId);
  }

  async setConfigOption(configId: string, value: string) {
    const response = await this.#requireContext().request(
      acp.methods.agent.session.setConfigOption,
      {
        sessionId: this.#requireSessionId(),
        configId,
        value,
      },
    );
    return readModelState(response);
  }

  async generateTitle(cwd: string, prompt: string, selectedModelId: string) {
    const context = this.#requireContext();
    const session = await context.request(acp.methods.agent.session.new, {
      cwd,
      mcpServers: [],
    });
    const modelState = readModelState(session);
    if (!modelState.modelConfigId) {
      throw new Error("The title session did not expose a model configuration");
    }

    const chunks: string[] = [];
    this.#titleSessions.set(session.sessionId, chunks);
    try {
      if (modelState.selectedModelId !== selectedModelId) {
        await context.request(acp.methods.agent.session.setConfigOption, {
          sessionId: session.sessionId,
          configId: modelState.modelConfigId,
          value: selectedModelId,
        });
      }
      await context.request(acp.methods.agent.session.prompt, {
        sessionId: session.sessionId,
        prompt: [{ type: "text", text: prompt }],
      });
      return chunks.join("");
    } finally {
      this.#titleSessions.delete(session.sessionId);
    }
  }

  answerPermission(requestId: RpcId, optionId: string) {
    this.#resolvePermission(requestId, {
      outcome: { outcome: "selected", optionId },
    });
  }

  cancelPermission(requestId: RpcId) {
    this.#resolvePermission(requestId, {
      outcome: { outcome: "cancelled" },
    });
  }

  async stop() {
    if (!this.#harnessId || this.#stopping) return;
    this.#stopping = true;
    const harnessId = this.#harnessId;
    this.#harnessId = undefined;
    this.#sessionId = undefined;
    this.#context = undefined;
    this.#connection?.close();
    this.#connection = undefined;
    this.#titleSessions.clear();
    this.#cancelPermissions();
    try {
      await this.#transport.stop(harnessId);
    } catch (error) {
      this.#stopping = false;
      throw error;
    }
  }

  #requireContext() {
    if (!this.#context) throw new Error("The harness is not running");
    return this.#context;
  }

  #requireSessionId() {
    if (!this.#sessionId) throw new Error("Connect the thread before sending a prompt");
    return this.#sessionId;
  }

  #receiveBrokerEvent(event: BrokerEvent) {
    if (event.event === "rpc") {
      this.#incoming?.enqueue(event.data.message);
      return;
    }
    if (event.event === "stderr") {
      this.#callbacks.stderr(event.data.line);
      return;
    }
    if (event.event === "error") {
      this.#callbacks.error(event.data.message);
      return;
    }

    const wasStopping = this.#stopping;
    this.#stopping = false;
    this.#harnessId = undefined;
    this.#sessionId = undefined;
    this.#context = undefined;
    this.#connection = undefined;
    this.#titleSessions.clear();
    this.#cancelPermissions();
    if (!this.#incomingClosed) {
      this.#incomingClosed = true;
      this.#incoming?.close();
    }
    if (!wasStopping) this.#callbacks.exited(event.data.code);
  }

  #receiveSessionUpdate(notification: acp.SessionNotification) {
    const titleChunks = this.#titleSessions.get(notification.sessionId);
    if (titleChunks) {
      if (
        notification.update.sessionUpdate === "agent_message_chunk" &&
        notification.update.content.type === "text"
      ) {
        titleChunks.push(notification.update.content.text);
      }
      return;
    }
    if (notification.sessionId === this.#sessionId) {
      this.#callbacks.update(notification.update);
    }
  }

  #requestPermission(requestId: RpcId, params: acp.RequestPermissionRequest) {
    if (this.#titleSessions.has(params.sessionId)) {
      return Promise.resolve<PermissionResponse>({ outcome: { outcome: "cancelled" } });
    }
    return new Promise<PermissionResponse>((resolve) => {
      this.#permissions.set(requestId, { resolve });
      this.#callbacks.permission(permissionFromAcp(requestId, params));
    });
  }

  #resolvePermission(requestId: RpcId, response: PermissionResponse) {
    const pending = this.#permissions.get(requestId);
    if (!pending) return;
    this.#permissions.delete(requestId);
    pending.resolve(response);
  }

  #cancelPermissions() {
    for (const pending of this.#permissions.values()) {
      pending.resolve({ outcome: { outcome: "cancelled" } });
    }
    this.#permissions.clear();
  }
}

function permissionFromAcp(
  requestId: RpcId,
  params: acp.RequestPermissionRequest,
): PermissionRequest {
  const rawInput = params.toolCall.rawInput;
  const detail =
    typeof rawInput === "string"
      ? rawInput
      : rawInput
        ? JSON.stringify(rawInput, null, 2)
        : "Review this request from the active coding agent.";
  return {
    requestId,
    title: params.toolCall.title ?? "Allow the harness to continue?",
    detail,
    options: params.options.map((option) => ({
      optionId: option.optionId,
      name: option.name,
      kind: option.kind,
    })),
  };
}

export { readModelState } from "../utils/model-state.ts";
export type { AcpModelState } from "../utils/model-state.ts";
