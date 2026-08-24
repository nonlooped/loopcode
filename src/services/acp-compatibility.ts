import * as acp from "@agentclientprotocol/sdk";
import { z } from "zod";

import type { QuestionAnswer, QuestionRequest } from "../types/index.ts";
import type { JsonValue } from "../utils/json.ts";
import {
  readCursorAvailableModels,
  type AcpAvailableModel,
  type AcpModelState,
} from "../utils/model-state.ts";

type RpcId = acp.JsonRpcId;

type CompatibilityQuestion = {
  id: string;
  title: string;
  prompt: string;
  options: QuestionRequest["options"];
  allowMultiple: boolean;
  allowCustomAnswer: boolean;
  required: boolean;
};

interface CompatibilityQuestions {
  questions: CompatibilityQuestion[];
  answers: QuestionAnswer[];
  resolve: (answers?: QuestionAnswer[]) => void;
}

interface CompatibilityHooks {
  requestQuestions: (requestId: RpcId, pending: CompatibilityQuestions) => void;
}

export interface AcpCompatibility {
  clientCapabilities: acp.ClientCapabilities;
  registerClientHandlers: (client: acp.ClientApp, hooks: CompatibilityHooks) => void;
  authenticate: (
    context: acp.ClientContext,
    initialized: acp.InitializeResponse,
    stderr: (line: string) => void,
  ) => Promise<void>;
  listModels: (context: acp.ClientContext) => Promise<AcpAvailableModel[]>;
  setModel: (
    context: acp.ClientContext,
    sessionId: string,
    modelId: string,
    state: AcpModelState,
  ) => Promise<AcpModelState | undefined>;
}

const standardCapabilities: acp.ClientCapabilities = {
  session: { configOptions: { boolean: {} } },
  elicitation: { form: {} },
};

const standardCompatibility: AcpCompatibility = {
  clientCapabilities: standardCapabilities,
  registerClientHandlers() {},
  async authenticate(_context, initialized, stderr) {
    if (initialized.authMethods?.length) {
      stderr(
        "This harness advertises an authentication flow. LoopCode expects its CLI to be signed in already.",
      );
    }
  },
  async listModels() {
    return [];
  },
  async setModel() {
    return undefined;
  },
};

const cursorCompatibility: AcpCompatibility = {
  ...standardCompatibility,
  clientCapabilities: {
    ...standardCapabilities,
    _meta: { parameterizedModelPicker: true },
  },
  registerClientHandlers(client, hooks) {
    client
      .onRequest(
        "cursor/ask_question",
        cursorAskQuestionSchema,
        ({ params, requestId }) =>
          new Promise<CursorAskQuestionResponse>((resolve) => {
            const questions = params.questions.map((question) => ({
              id: question.id,
              title: params.title ?? "Agent question",
              prompt: question.prompt,
              options: question.options.map((option) => ({
                optionId: option.id,
                name: option.label,
              })),
              allowMultiple: question.allowMultiple === true,
              allowCustomAnswer: false,
              required: true,
            }));
            hooks.requestQuestions(requestId, {
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
          }),
      )
      .onRequest<unknown, { outcome: { outcome: "rejected"; reason: string } }>(
        "cursor/create_plan",
        (params) => params,
        () => ({
          outcome: { outcome: "rejected", reason: "LoopCode does not support plan approval." },
        }),
      );
  },
  async listModels(context) {
    const response = await context.request<JsonValue>("cursor/list_available_models", {});
    return readCursorAvailableModels(response);
  },
};

const grokCompatibility: AcpCompatibility = {
  ...standardCompatibility,
  async authenticate(context, initialized) {
    const authMethod = initialized.authMethods?.find(
      (candidate) => candidate.id === "cached_token" || candidate.id === "xai.api_key",
    );
    if (authMethod) {
      await context.request(acp.methods.agent.authenticate, { methodId: authMethod.id });
    }
  },
  async setModel(context, sessionId, modelId, state) {
    await context.request<unknown>("session/set_model", { sessionId, modelId });
    return { ...state, selectedModelId: modelId };
  },
};

export function compatibilityFor(profileId: string | undefined): AcpCompatibility {
  if (profileId === "cursor") return cursorCompatibility;
  if (profileId === "grok") return grokCompatibility;
  return standardCompatibility;
}

type CursorAskQuestionResponse = {
  outcome:
    | {
        outcome: "answered";
        answers: Array<{ questionId: string; selectedOptionIds: string[] }>;
      }
    | { outcome: "cancelled" };
};

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
