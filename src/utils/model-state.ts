import type {
  ConfigOptionUpdate,
  NewSessionResponse,
  SessionConfigOption,
  SetSessionConfigOptionResponse,
} from "@agentclientprotocol/sdk";
import { z } from "zod";

import type { FastModeValueType, ModelOption } from "../types/index.ts";
import type { JsonValue } from "./json.ts";

export interface AcpModelState {
  modelConfigId?: string;
  models: ModelOption[];
  selectedModelId?: string;
  reasoningConfigId?: string;
  reasoningOptions: ModelOption[];
  selectedReasoningId?: string;
  fastModeConfigId?: string;
  fastModeEnabled?: boolean;
  fastModeValueType?: FastModeValueType;
  fastModeDescription?: string;
}

type ConfigState = Pick<
  NewSessionResponse | SetSessionConfigOptionResponse | ConfigOptionUpdate,
  "configOptions"
>;

export function readModelState(value: ConfigState): AcpModelState {
  const configOptions = value.configOptions ?? [];
  const modelConfig = configOptions.find((option) => option.category === "model");
  const reasoningConfig =
    configOptions.find(isExplicitReasoningConfig) ?? configOptions.find(isReasoningConfig);
  const fastModeConfig = configOptions.find(isFastModeConfig);

  return {
    modelConfigId: modelConfig?.id,
    models: configChoices(modelConfig),
    selectedModelId: modelConfig?.type === "select" ? modelConfig.currentValue : undefined,
    reasoningConfigId: reasoningConfig?.id,
    reasoningOptions: configChoices(reasoningConfig),
    selectedReasoningId:
      reasoningConfig?.type === "select" ? reasoningConfig.currentValue : undefined,
    fastModeConfigId: fastModeConfig?.id,
    fastModeEnabled: configBooleanValue(fastModeConfig),
    fastModeValueType:
      fastModeConfig?.type === "boolean"
        ? "boolean"
        : fastModeConfig?.type === "select"
          ? "string"
          : undefined,
    fastModeDescription: fastModeConfig?.description ?? undefined,
  };
}

function configChoices(config: SessionConfigOption | undefined): ModelOption[] {
  if (!config || config.type !== "select") return [];
  const choices = config.options.flatMap((option) =>
    "group" in option ? option.options : [option],
  );
  return choices.map((option) => ({
    id: option.value,
    name: option.name,
    description: option.description ?? undefined,
  }));
}

function isExplicitReasoningConfig(option: SessionConfigOption) {
  return (
    option.type === "select" && /reason|effort/.test(`${option.id} ${option.name}`.toLowerCase())
  );
}

function isReasoningConfig(option: SessionConfigOption) {
  if (option.type !== "select") return false;
  const category = option.category?.toLowerCase().replaceAll("-", "_");
  if (category === "thought_level" || category === "thoughtlevel") return true;
  if (category !== "model_config") return false;
  return /reason|thought|effort/.test(`${option.id} ${option.name}`.toLowerCase());
}

function isFastModeConfig(option: SessionConfigOption) {
  if (!/fast/.test(`${option.id} ${option.name}`.toLowerCase())) return false;
  if (option.type === "boolean") return true;
  if (option.type !== "select") return false;
  const values = new Set(configChoices(option).map(({ id }) => id));
  return values.has("true") && values.has("false");
}

function configBooleanValue(option: SessionConfigOption | undefined) {
  if (option?.type === "boolean") return option.currentValue;
  if (option?.type !== "select") return undefined;
  if (option.currentValue === "true") return true;
  if (option.currentValue === "false") return false;
  return undefined;
}

export interface AcpAvailableModel extends AcpModelState {
  model: ModelOption;
}

const configChoiceSchema = z.object({
  value: z.string(),
  name: z.string(),
  description: z.string().nullable().optional(),
});
const sessionConfigOptionBase = {
  id: z.string(),
  name: z.string(),
  description: z.string().nullable().optional(),
  category: z.string().nullable().optional(),
};
const sessionConfigOptionSchema = z.discriminatedUnion("type", [
  z.object({
    ...sessionConfigOptionBase,
    type: z.literal("select"),
    currentValue: z.string(),
    options: z.union([
      z.array(configChoiceSchema),
      z.array(
        z.object({
          group: z.string(),
          name: z.string(),
          options: z.array(configChoiceSchema),
        }),
      ),
    ]),
  }),
  z.object({
    ...sessionConfigOptionBase,
    type: z.literal("boolean"),
    currentValue: z.boolean(),
  }),
]);
const cursorModelCatalogSchema = z.object({
  models: z.array(
    z.object({
      value: z.string().trim().min(1),
      name: z.string().trim().min(1),
      configOptions: z.array(sessionConfigOptionSchema).optional(),
    }),
  ),
});

export function readCursorAvailableModels(payload: JsonValue): AcpAvailableModel[] {
  const parsed = cursorModelCatalogSchema.safeParse(payload);
  if (!parsed.success) throw new Error("Cursor returned an invalid model catalog");
  return parsed.data.models.map((entry) => {
    const state = readModelState({ configOptions: entry.configOptions ?? [] });
    return { model: { id: entry.value, name: entry.name }, ...state };
  });
}
