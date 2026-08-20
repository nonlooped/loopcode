import * as acp from "@agentclientprotocol/sdk";
import { z } from "zod";

import { launchHarness, sendRpc, stopHarness, type BrokerEvent } from "./native.ts";
import type {
  AcpErrorDetails,
  ConnectRequest,
  ConnectionStatus,
  PermissionMode,
  PermissionRequest,
  QuestionAnswer,
  QuestionRequest,
  ThreadStatus,
  TurnStatus,
} from "../types/index.ts";
import type { JsonValue } from "../utils/json.ts";
import {
  readCursorAvailableModels,
  readModelState,
  type AcpModelState,
} from "../utils/model-state.ts";

type RpcId = acp.JsonRpcId;

type PermissionResponse = acp.RequestPermissionResponse;

interface PendingPermission {
  resolve: (response: PermissionResponse) => void;
}

interface QuestionSpec {
  id: string;
  title: string;
  prompt: string;
  options: QuestionRequest["options"];
  allowMultiple: boolean;
  allowCustomAnswer: boolean;
  customFieldId?: string;
  required: boolean;
}

interface PendingQuestions {
  questions: QuestionSpec[];
  answers: QuestionAnswer[];
  resolve: (answers?: QuestionAnswer[]) => void;
}

interface CursorAskQuestionRequest {
  title?: string;
  questions: Array<{
    id: string;
    prompt: string;
    options: Array<{ id: string; label: string }>;
    allowMultiple?: boolean;
  }>;
}

type CursorAskQuestionResponse = {
  outcome:
    | {
        outcome: "answered";
        answers: Array<{ questionId: string; selectedOptionIds: string[] }>;
      }
    | { outcome: "cancelled" };
};

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

function cursorCapabilities(profileId: string | undefined): acp.ClientCapabilities {
  const capabilities: acp.ClientCapabilities = {
    session: { configOptions: { boolean: {} } },
    elicitation: { form: {} },
  };
  if (profileId === "cursor") capabilities._meta = { parameterizedModelPicker: true };
  return capabilities;
}

function isTextPrompt(value: string | PromptContent[]): value is string {
  return typeof value === "string";
}

function isBooleanConfigValue(value: string | boolean): value is boolean {
  return typeof value === "boolean";
}

export type PromptImage = acp.ImageContent & { type: "image" };
export type PromptContent = acp.ContentBlock;

export interface AcpCallbacks {
  connectionStatus?: (status: ConnectionStatus) => void;
  turnStatus?: (status: TurnStatus) => void;
  status?: (status: ThreadStatus) => void;
  ready: (session: AcpSessionInfo) => void;
  update: (update: acp.SessionUpdate) => void;
  permission: (request: PermissionRequest) => void;
  permissionMode?: () => PermissionMode;
  stderr: (line: string) => void;
  error: (error: AcpErrorDetails) => void;
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
  #questions = new Map<RpcId, PendingQuestions>();
  #activeToolIds = new Set<string>();
  #turnActive = false;
  #stopping = false;

  constructor(callbacks: AcpCallbacks, transport: AcpTransport = nativeTransport) {
    this.#callbacks = callbacks;
    this.#transport = transport;
  }

  get harnessId() {
    return this.#harnessId;
  }

  async connect(request: ConnectRequest) {
    this.#emitConnectionStatus("connecting");
    let method = "launch";
    try {
      const readable = new ReadableStream<acp.AnyMessage>({
        start: (controller) => {
          this.#incoming = controller;
        },
      });

      this.#harnessId = await this.#transport.launch(
        {
          command: request.command,
          args: request.args,
          cwd: request.cwd,
          profileId: request.profileId,
          threadId: request.threadId,
        },
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
        )
        .onRequest("cursor/ask_question", cursorAskQuestionSchema, ({ params, requestId }) =>
          this.#requestCursorQuestion(requestId, params),
        )
        .onRequest<unknown, { outcome: { outcome: "rejected"; reason: string } }>(
          "cursor/create_plan",
          (params) => params,
          () => ({
            outcome: { outcome: "rejected", reason: "LoopCode does not support plan approval." },
          }),
        );

      this.#connection = client.connect({ readable, writable });
      this.#context = this.#connection.agent;
      method = "initialize";
      const initialized = await this.#context.request(acp.methods.agent.initialize, {
        protocolVersion: acp.PROTOCOL_VERSION,
        clientCapabilities: cursorCapabilities(request.profileId),
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
          method = "session/resume";
          sessionState = await this.#context.request(acp.methods.agent.session.resume, {
            sessionId,
            cwd: request.cwd,
            mcpServers: [],
          });
        } else if (initialized.agentCapabilities?.loadSession) {
          method = "session/load";
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
        method = "session/new";
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
      this.#emitConnectionStatus("ready");
    } catch (error) {
      this.#callbacks.error(errorDetails(error, "connection", method));
      this.#emitConnectionStatus("error");
      await this.stop();
      throw error;
    }
  }

  async listCursorModels() {
    // TODO: Unsafe undocumented Cursor extension; replace it when stable ACP discovery is sufficient.
    const response = await this.#requireContext().request<JsonValue>(
      "cursor/list_available_models",
      {},
    );
    return readCursorAvailableModels(response);
  }

  async prompt(prompt: string | PromptContent[], images: PromptImage[] = []) {
    if (this.#turnActive) throw new Error("This ACP session already has an active turn");
    const context = this.#requireContext();
    const sessionId = this.#requireSessionId();
    this.#turnActive = true;
    this.#activeToolIds.clear();
    this.#emitTurnStatus("running");
    let blockedReported = false;
    try {
      await context.request(acp.methods.agent.session.prompt, {
        sessionId,
        prompt: isTextPrompt(prompt)
          ? [...(prompt ? [{ type: "text" as const, text: prompt }] : []), ...images]
          : prompt,
      });
      if (this.#activeToolIds.size > 0) {
        const error = new Error(
          `ACP session/prompt completed with ${this.#activeToolIds.size} tool call(s) still active`,
        );
        this.#callbacks.error(errorDetails(error, "turn", "session/prompt"));
        this.#emitTurnStatus("blocked");
        blockedReported = true;
        throw error;
      }
      this.#turnActive = false;
      this.#emitTurnStatus("idle");
    } catch (error) {
      if (this.#activeToolIds.size > 0) {
        if (!blockedReported) {
          this.#callbacks.error(errorDetails(error, "turn", "session/prompt"));
          this.#emitTurnStatus("blocked");
        }
        throw error;
      }
      this.#turnActive = false;
      this.#callbacks.error(errorDetails(error, "turn", "session/prompt"));
      this.#emitTurnStatus("failed");
      throw error;
    }
  }

  async cancel() {
    if (!this.#sessionId || !this.#context) return;
    await this.#context.notify(acp.methods.agent.session.cancel, {
      sessionId: this.#sessionId,
    });
  }

  async setModel(configId: string, modelId: string) {
    return this.setConfigOption(configId, modelId);
  }

  async setConfigOption(configId: string, value: string | boolean) {
    const sessionId = this.#requireSessionId();
    const params: acp.SetSessionConfigOptionRequest = isBooleanConfigValue(value)
      ? { sessionId, configId, value, type: "boolean" }
      : { sessionId, configId, value };
    const response = await this.#requireContext().request(
      acp.methods.agent.session.setConfigOption,
      params,
    );
    return readModelState(response);
  }

  setFastModeConfigOption(
    configId: string,
    value: boolean,
    valueType: "boolean" | "string" = "boolean",
  ) {
    return this.setConfigOption(configId, valueType === "string" ? String(value) : value);
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
      return;
    }
    this.#resolveQuestion(requestId, { selectedOptionIds: [optionId] });
  }

  answerQuestion(requestId: RpcId, answer: QuestionAnswer) {
    if (this.#permissions.has(requestId)) {
      const [optionId] = answer.selectedOptionIds;
      if (optionId) this.answerPermission(requestId, optionId);
      else this.cancelPermission(requestId);
      return;
    }
    this.#resolveQuestion(requestId, answer);
  }

  cancelPermission(requestId: RpcId) {
    if (this.#permissions.has(requestId)) {
      this.#resolvePermission(requestId, {
        outcome: { outcome: "cancelled" },
      });
      return;
    }
    this.#resolveQuestion(requestId);
  }

  async stop() {
    if (!this.#harnessId || this.#stopping) return;
    this.#stopping = true;
    const harnessId = this.#harnessId;
    this.#harnessId = undefined;
    this.#sessionId = undefined;
    this.#context = undefined;
    this.#loadingSession = false;
    this.#turnActive = false;
    this.#activeToolIds.clear();
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

  #emitConnectionStatus(status: ConnectionStatus) {
    this.#callbacks.connectionStatus?.(status);
    this.#callbacks.status?.(status);
  }

  #emitTurnStatus(status: TurnStatus) {
    this.#callbacks.turnStatus?.(status);
    if (status === "running") this.#callbacks.status?.("running");
    else if (status === "blocked") this.#callbacks.status?.("error");
    else this.#callbacks.status?.("ready");
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
      this.#callbacks.error({ scope: "transport", message: event.data.message });
      this.#emitConnectionStatus("error");
      return;
    }

    const wasStopping = this.#stopping;
    this.#stopping = false;
    this.#harnessId = undefined;
    this.#sessionId = undefined;
    this.#context = undefined;
    this.#connection = undefined;
    this.#loadingSession = false;
    this.#turnActive = false;
    this.#activeToolIds.clear();
    this.#titleSessions.clear();
    this.#cancelInteractions();
    if (!this.#incomingClosed) {
      this.#incomingClosed = true;
      this.#incoming?.close();
    }
    if (!wasStopping) this.#callbacks.exited(event.data.code);
  }

  #receiveSessionUpdate(notification: acp.SessionNotification) {
    this.#trackToolUpdate(notification);
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

  #trackToolUpdate(notification: acp.SessionNotification) {
    if (notification.sessionId !== this.#sessionId) return;
    const update = notification.update;
    if (update.sessionUpdate !== "tool_call" && update.sessionUpdate !== "tool_call_update") return;
    if (update.status === "pending" || update.status === "in_progress") {
      this.#activeToolIds.add(update.toolCallId);
    } else if (update.status) {
      this.#activeToolIds.delete(update.toolCallId);
    }
  }

  #requestPermission(requestId: RpcId, params: acp.RequestPermissionRequest) {
    if (this.#titleSessions.has(params.sessionId)) {
      return Promise.resolve<PermissionResponse>({ outcome: { outcome: "cancelled" } });
    }
    const request = permissionFromAcp(requestId, params);
    const allowedOptionId = preferredAllowOptionId(request);
    if (
      request.type === "permission" &&
      this.#callbacks.permissionMode?.() === "full" &&
      allowedOptionId
    ) {
      return Promise.resolve<PermissionResponse>({
        outcome: { outcome: "selected", optionId: allowedOptionId },
      });
    }
    return new Promise<PermissionResponse>((resolve) => {
      this.#permissions.set(requestId, { resolve });
      this.#callbacks.permission(request);
    });
  }

  #requestElicitation(requestId: RpcId, params: acp.CreateElicitationRequest) {
    const sessionId =
      "sessionId" in params ? z.string().safeParse(params.sessionId).data : undefined;
    if (sessionId && this.#titleSessions.has(sessionId)) {
      return Promise.resolve<acp.CreateElicitationResponse>({ action: "cancel" });
    }
    const questions = questionsFromElicitation(params);
    if (questions.length === 0) {
      return Promise.resolve<acp.CreateElicitationResponse>({ action: "cancel" });
    }

    return new Promise<acp.CreateElicitationResponse>((resolve) => {
      this.#requestQuestions(requestId, {
        questions,
        answers: [],
        resolve: (answers) =>
          resolve(
            answers
              ? { action: "accept", content: elicitationContent(questions, answers) }
              : { action: "cancel" },
          ),
      });
    });
  }

  #requestCursorQuestion(requestId: RpcId, params: CursorAskQuestionRequest) {
    return new Promise<CursorAskQuestionResponse>((resolve) => {
      const questions = params.questions.map((question) => ({
        id: question.id,
        title: params.title ?? "Agent question",
        prompt: question.prompt,
        options: question.options.map((option) => ({ optionId: option.id, name: option.label })),
        allowMultiple: question.allowMultiple === true,
        allowCustomAnswer: false,
        required: true,
      }));
      this.#requestQuestions(requestId, {
        questions,
        answers: [],
        resolve: (answers) =>
          resolve(
            answers
              ? {
                  outcome: {
                    outcome: "answered",
                    answers: answers.map((answer, index) => ({
                      questionId: questions[index].id,
                      selectedOptionIds: answer.selectedOptionIds,
                    })),
                  },
                }
              : { outcome: { outcome: "cancelled" } },
          ),
      });
    });
  }

  #requestQuestions(requestId: RpcId, pending: PendingQuestions) {
    this.#questions.set(requestId, pending);
    this.#showNextQuestion(requestId);
  }

  #showNextQuestion(requestId: RpcId) {
    const pending = this.#questions.get(requestId);
    if (!pending) return;
    const question = pending.questions[pending.answers.length];
    this.#callbacks.permission({
      requestId,
      type: "question",
      title: question.title,
      detail: question.prompt,
      options: question.options,
      allowMultiple: question.allowMultiple,
      allowCustomAnswer: question.allowCustomAnswer,
      required: question.required,
    });
  }

  #resolvePermission(requestId: RpcId, response: PermissionResponse) {
    const pending = this.#permissions.get(requestId);
    if (!pending) return;
    this.#permissions.delete(requestId);
    pending.resolve(response);
  }

  #resolveQuestion(requestId: RpcId, answer?: QuestionAnswer) {
    const pending = this.#questions.get(requestId);
    if (!pending) return;
    if (!answer) {
      this.#questions.delete(requestId);
      pending.resolve();
      return;
    }

    const question = pending.questions[pending.answers.length];
    const optionIds = new Set(question.options.map((option) => option.optionId));
    const selectedOptionIds = answer.selectedOptionIds.filter((optionId) =>
      optionIds.has(optionId),
    );
    pending.answers.push({
      selectedOptionIds: question.allowMultiple ? selectedOptionIds : selectedOptionIds.slice(0, 1),
      customAnswer: question.allowCustomAnswer
        ? answer.customAnswer?.trim() || undefined
        : undefined,
    });
    if (pending.answers.length < pending.questions.length) {
      this.#showNextQuestion(requestId);
      return;
    }

    this.#questions.delete(requestId);
    pending.resolve(pending.answers);
  }

  #cancelInteractions() {
    for (const pending of this.#permissions.values()) {
      pending.resolve({ outcome: { outcome: "cancelled" } });
    }
    this.#permissions.clear();
    for (const pending of this.#questions.values()) pending.resolve();
    this.#questions.clear();
  }
}

function errorDetails(
  cause: unknown,
  scope: AcpErrorDetails["scope"],
  method?: string,
): AcpErrorDetails {
  const details: AcpErrorDetails = {
    scope,
    method,
    message: cause instanceof Error ? cause.message : String(cause),
  };
  const parsed = z
    .object({ code: z.number().optional(), data: z.json().optional() })
    .safeParse(cause);
  if (!parsed.success) return details;
  details.code = parsed.data.code;
  details.data = parsed.data.data;
  return details;
}

function permissionFromAcp(
  requestId: RpcId,
  params: acp.RequestPermissionRequest,
): PermissionRequest {
  const parsedRawInput = z.json().safeParse(params.toolCall.rawInput);
  const rawInput = parsedRawInput.success ? parsedRawInput.data : null;
  const question = questionFromInput(rawInput);
  const detail = question?.text ?? requestDetail(rawInput);
  const options = params.options.map((option) => ({
    optionId: option.optionId,
    name: option.name,
    description: question?.options.get(option.name),
    kind: option.kind,
  }));
  if (question) {
    return {
      requestId,
      type: "question",
      title: question.title,
      detail,
      options,
      allowMultiple: false,
      allowCustomAnswer: false,
      required: true,
    };
  }
  return {
    requestId,
    type: "permission",
    title: params.toolCall.title ?? "Allow the harness to continue?",
    detail,
    options,
  };
}

function questionsFromElicitation(params: acp.CreateElicitationRequest): QuestionSpec[] {
  if (!acp.CreateElicitationRequest.isForm(params)) return [];
  const properties = params.requestedSchema.properties ?? {};
  if (Object.values(properties).some((schema) => !supportsQuestionField(schema))) return [];

  const required = new Set(params.requestedSchema.required ?? []);
  const customFields = new Map<string, string>();
  for (const [fieldId, schema] of Object.entries(properties)) {
    const target = customAnswerTarget(schema);
    if (target) customFields.set(target, fieldId);
  }

  const questions: QuestionSpec[] = [];
  for (const [fieldId, schema] of Object.entries(properties)) {
    if (customAnswerTarget(schema)) continue;
    const customFieldId = customFields.get(fieldId);
    if (acp.ElicitationPropertySchema.isString(schema)) {
      const options = stringOptions(schema);
      questions.push({
        id: fieldId,
        title: schema.title ?? "Agent question",
        prompt: schema.description ?? params.message,
        options,
        allowMultiple: false,
        allowCustomAnswer: Boolean(customFieldId) || options.length === 0,
        customFieldId,
        required: required.has(fieldId),
      });
      continue;
    }
    if (acp.ElicitationPropertySchema.isArray(schema)) {
      const options = multiSelectOptions(schema.items);
      questions.push({
        id: fieldId,
        title: schema.title ?? "Agent question",
        prompt: schema.description ?? params.message,
        options,
        allowMultiple: true,
        allowCustomAnswer: Boolean(customFieldId),
        customFieldId,
        required: required.has(fieldId),
      });
    }
  }
  return questions;
}

function supportsQuestionField(schema: acp.ElicitationPropertySchema) {
  if (acp.ElicitationPropertySchema.isString(schema)) {
    return (
      schema.minLength == null &&
      schema.maxLength == null &&
      schema.pattern == null &&
      schema.format == null
    );
  }
  return (
    acp.ElicitationPropertySchema.isArray(schema) &&
    schema.minItems == null &&
    schema.maxItems == null &&
    multiSelectOptions(schema.items).length > 0
  );
}

function stringOptions(schema: acp.StringPropertySchema): QuestionRequest["options"] {
  if (schema.oneOf) return schema.oneOf.map(optionFromEnum);
  return (schema.enum ?? []).map((value) => ({ optionId: value, name: value }));
}

function multiSelectOptions(items: acp.MultiSelectItems): QuestionRequest["options"] {
  if (acp.MultiSelectItems.isTitled(items)) return items.anyOf.map(optionFromEnum);
  if (acp.MultiSelectItems.isString(items)) {
    return items.enum.map((value) => ({ optionId: value, name: value }));
  }
  return [];
}

function optionFromEnum(option: acp.EnumOption): QuestionRequest["options"][number] {
  const optionMeta = option._meta?.["_claude/askUserQuestionOption"];
  const preview = z.object({ preview: z.string() }).safeParse(optionMeta).data?.preview;
  const result: QuestionRequest["options"][number] = {
    optionId: option.const,
    name: option.title,
    description: option.description ?? undefined,
  };
  if (preview) result.preview = preview;
  return result;
}

function customAnswerTarget(schema: acp.ElicitationPropertySchema) {
  const parsed = z
    .object({
      _askUserQuestionCustomAnswer: z
        .object({ isCustomAnswer: z.literal(true), questionId: z.string() })
        .optional(),
      codex: z.object({ isOtherAnswer: z.literal(true), questionId: z.string() }).optional(),
    })
    .safeParse(schema._meta);
  if (!parsed.success) return;
  return parsed.data._askUserQuestionCustomAnswer?.questionId ?? parsed.data.codex?.questionId;
}

function elicitationContent(questions: QuestionSpec[], answers: QuestionAnswer[]) {
  const content: Record<string, string | string[]> = {};
  questions.forEach((question, index) => {
    const answer = answers[index];
    if (answer.customAnswer) {
      content[question.customFieldId ?? question.id] = answer.customAnswer;
    } else if (question.allowMultiple) {
      content[question.id] = answer.selectedOptionIds;
    } else if (answer.selectedOptionIds[0]) {
      content[question.id] = answer.selectedOptionIds[0];
    }
  });
  return content;
}

export function preferredAllowOptionId(request: PermissionRequest) {
  return (
    request.options.find((option) => option.kind === "allow_once")?.optionId ??
    request.options.find((option) => option.kind?.startsWith("allow"))?.optionId
  );
}

function requestDetail(rawInput: JsonValue) {
  const text = z.string().safeParse(rawInput);
  if (text.success) return text.data;
  if (rawInput) return JSON.stringify(rawInput, null, 2);
  return "Review this request from the active coding agent.";
}

const permissionQuestionSchema = z.object({
  header: z.string().optional(),
  question: z.string(),
  options: z.array(z.object({ label: z.string(), description: z.string().optional() })).optional(),
});

function questionFromInput(rawInput: JsonValue) {
  const container = z.object({ questions: z.array(z.json()).min(1) }).safeParse(rawInput);
  const parsed = permissionQuestionSchema.safeParse(
    container.success ? container.data.questions[0] : rawInput,
  );
  if (!parsed.success) return;
  const options = new Map(
    (parsed.data.options ?? []).flatMap((option) =>
      option.description ? [[option.label, option.description] as const] : [],
    ),
  );
  return {
    title: parsed.data.header ?? "Agent question",
    text: parsed.data.question,
    options,
  };
}

const cursorAskQuestionSchema = z.object({
  title: z.string().optional(),
  questions: z
    .array(
      z.object({
        id: z.string(),
        prompt: z.string(),
        options: z.array(z.object({ id: z.string(), label: z.string() })),
        allowMultiple: z.boolean().optional(),
      }),
    )
    .min(1),
});

export { readModelState } from "../utils/model-state.ts";
export type { AcpModelState } from "../utils/model-state.ts";
