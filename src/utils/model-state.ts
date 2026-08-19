import type {
  ConfigOptionUpdate,
  NewSessionResponse,
  SessionConfigOption,
  SetSessionConfigOptionResponse,
} from "@agentclientprotocol/sdk";

import type { FastModeValueType, ModelOption } from "../types/index.ts";

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

export function readCursorAvailableModels(value: unknown): AcpAvailableModel[] {
  if (!isRecord(value) || !Array.isArray(value.models)) {
    throw new Error("Cursor returned an invalid model catalog");
  }
  return value.models.map((entry) => {
    if (
      !isRecord(entry) ||
      typeof entry.value !== "string" ||
      !entry.value.trim() ||
      typeof entry.name !== "string" ||
      !entry.name.trim()
    ) {
      throw new Error("Cursor returned an invalid model catalog entry");
    }
    // Unsafe: this undocumented response has no maintained schema; replace with stable ACP discovery.
    const state = readModelState({
      configOptions: Array.isArray(entry.configOptions)
        ? (entry.configOptions as SessionConfigOption[])
        : [],
    });
    return { model: { id: entry.value, name: entry.name }, ...state };
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
