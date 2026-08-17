import * as acp from "@agentclientprotocol/sdk";

import { launchHarness, sendRpc, stopHarness, type BrokerEvent } from "./native.ts";
import type { ConnectRequest, PermissionRequest, ThreadStatus } from "../types/index.ts";
import { readModelState, type AcpModelState } from "../utils/model-state.ts";

type RpcId = acp.JsonRpcId;

type PermissionResponse = acp.RequestPermissionResponse;

interface PendingPermission {
  resolve: (response: PermissionResponse) => void;
}

interface PendingElicitation {
  fieldId: string;
  resolve: (response: acp.CreateElicitationResponse) => void;
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
  #loadingSession = false;
  #titleSessions = new Map<string, string[]>();
  #permissions = new Map<RpcId, PendingPermission>();
  #elicitations = new Map<RpcId, PendingElicitation>();
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
        )
        .onRequest(acp.methods.client.elicitation.create, ({ params, requestId }) =>
          this.#requestElicitation(requestId, params),
        );

      this.#connection = client.connect({ readable, writable });
      this.#context = this.#connection.agent;
      const initialized = await this.#context.request(acp.methods.agent.initialize, {
        protocolVersion: acp.PROTOCOL_VERSION,
        clientCapabilities: {
          session: { configOptions: { boolean: {} } },
          elicitation: { form: {} },
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

      let sessionId: string;
      let sessionState:
        | acp.NewSessionResponse
        | acp.LoadSessionResponse
        | acp.ResumeSessionResponse;
      if (request.sessionId) {
        sessionId = request.sessionId;
        this.#sessionId = sessionId;
        if (initialized.agentCapabilities?.sessionCapabilities?.resume) {
          sessionState = await this.#context.request(acp.methods.agent.session.resume, {
            sessionId,
            cwd: request.cwd,
            mcpServers: [],
          });
        } else if (initialized.agentCapabilities?.loadSession) {
          this.#loadingSession = true;
          try {
            sessionState = await this.#context.request(acp.methods.agent.session.load, {
              sessionId,
              cwd: request.cwd,
              mcpServers: [],
            });
          } finally {
            this.#loadingSession = false;
          }
        } else {
          throw new Error("This agent cannot restore its previous session context");
        }
      } else {
        const session = await this.#context.request(acp.methods.agent.session.new, {
          cwd: request.cwd,
          mcpServers: [],
        });
        sessionId = session.sessionId;
        this.#sessionId = sessionId;
        sessionState = session;
      }
      this.#callbacks.ready({
        harnessId,
        sessionId,
        ...readModelState(sessionState),
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

  async setBooleanConfigOption(configId: string, value: boolean) {
    const response = await this.#requireContext().request(
      acp.methods.agent.session.setConfigOption,
      {
        sessionId: this.#requireSessionId(),
        configId,
        value,
        type: "boolean",
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
    if (this.#permissions.has(requestId)) {
      this.#resolvePermission(requestId, {
        outcome: { outcome: "selected", optionId },
      });
    } else {
      this.#resolveElicitation(requestId, optionId);
    }
  }

  cancelPermission(requestId: RpcId) {
    if (this.#permissions.has(requestId)) {
      this.#resolvePermission(requestId, {
        outcome: { outcome: "cancelled" },
      });
    } else {
      this.#resolveElicitation(requestId);
    }
  }

  async stop() {
    if (!this.#harnessId || this.#stopping) return;
    this.#stopping = true;
    const harnessId = this.#harnessId;
    this.#harnessId = undefined;
    this.#sessionId = undefined;
    this.#context = undefined;
    this.#loadingSession = false;
    this.#connection?.close();
    this.#connection = undefined;
    this.#titleSessions.clear();
    this.#cancelInteractions();
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
    this.#loadingSession = false;
    this.#titleSessions.clear();
    this.#cancelInteractions();
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
    if (notification.sessionId === this.#sessionId && !this.#loadingSession) {
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

  #requestElicitation(requestId: RpcId, params: acp.CreateElicitationRequest) {
    const sessionId =
      "sessionId" in params && typeof params.sessionId === "string" ? params.sessionId : undefined;
    if (sessionId && this.#titleSessions.has(sessionId)) {
      return Promise.resolve<acp.CreateElicitationResponse>({ action: "cancel" });
    }
    const question = questionFromElicitation(params);
    if (!question) return Promise.resolve<acp.CreateElicitationResponse>({ action: "cancel" });

    return new Promise<acp.CreateElicitationResponse>((resolve) => {
      this.#elicitations.set(requestId, { fieldId: question.fieldId, resolve });
      this.#callbacks.permission(question.request(requestId));
    });
  }

  #resolvePermission(requestId: RpcId, response: PermissionResponse) {
    const pending = this.#permissions.get(requestId);
    if (!pending) return;
    this.#permissions.delete(requestId);
    pending.resolve(response);
  }

  #resolveElicitation(requestId: RpcId, optionId?: string) {
    const pending = this.#elicitations.get(requestId);
    if (!pending) return;
    this.#elicitations.delete(requestId);
    pending.resolve(
      optionId
        ? { action: "accept", content: { [pending.fieldId]: optionId } }
        : { action: "cancel" },
    );
  }

  #cancelInteractions() {
    for (const pending of this.#permissions.values()) {
      pending.resolve({ outcome: { outcome: "cancelled" } });
    }
    this.#permissions.clear();
    for (const pending of this.#elicitations.values()) {
      pending.resolve({ action: "cancel" });
    }
    this.#elicitations.clear();
  }
}

function permissionFromAcp(
  requestId: RpcId,
  params: acp.RequestPermissionRequest,
): PermissionRequest {
  const rawInput = params.toolCall.rawInput;
  const question = questionFromInput(rawInput);
  const detail = question?.text ?? requestDetail(rawInput);
  return {
    requestId,
    type: question ? "question" : "permission",
    title: question?.title ?? params.toolCall.title ?? "Allow the harness to continue?",
    detail,
    options: params.options.map((option) => ({
      optionId: option.optionId,
      name: option.name,
      description: question?.options.get(option.name),
      kind: option.kind,
    })),
  };
}

function questionFromElicitation(params: acp.CreateElicitationRequest) {
  if (!acp.CreateElicitationRequest.isForm(params)) return;
  const [fieldId, schema] = Object.entries(params.requestedSchema.properties ?? {})[0] ?? [];
  if (!fieldId || !schema || !acp.ElicitationPropertySchema.isString(schema)) return;
  const options = schema.oneOf;
  if (!options) return;

  return {
    fieldId,
    request: (requestId: RpcId) => ({
      requestId,
      type: "question" as const,
      title: schema.title ?? "Agent question",
      detail: schema.description ?? params.message,
      options: options.map((option) => ({
        optionId: option.const,
        name: option.title,
        description: option.description ?? undefined,
      })),
    }),
  };
}

function requestDetail(rawInput: unknown) {
  if (typeof rawInput === "string") return rawInput;
  if (rawInput) return JSON.stringify(rawInput, null, 2);
  return "Review this request from the active coding agent.";
}

function questionFromInput(rawInput: unknown) {
  if (!isRecord(rawInput)) return;
  const rawQuestion = Array.isArray(rawInput.questions) ? rawInput.questions[0] : rawInput;
  if (!isRecord(rawQuestion) || typeof rawQuestion.question !== "string") return;

  const options = new Map<string, string>();
  if (Array.isArray(rawQuestion.options)) {
    for (const option of rawQuestion.options) {
      if (!isRecord(option) || typeof option.label !== "string") continue;
      if (typeof option.description === "string") options.set(option.label, option.description);
    }
  }

  return {
    title: typeof rawQuestion.header === "string" ? rawQuestion.header : "Agent question",
    text: rawQuestion.question,
    options,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export { readModelState } from "../utils/model-state.ts";
export type { AcpModelState } from "../utils/model-state.ts";
