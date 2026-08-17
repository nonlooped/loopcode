import type {
  ConfigOptionUpdate,
  NewSessionResponse,
  SessionConfigOption,
  SetSessionConfigOptionResponse,
} from "@agentclientprotocol/sdk";

import type { ModelOption } from "../types/index.ts";

export interface AcpModelState {
  modelConfigId?: string;
  models: ModelOption[];
  selectedModelId?: string;
  reasoningConfigId?: string;
  reasoningOptions: ModelOption[];
  selectedReasoningId?: string;
  fastModeConfigId?: string;
  fastModeEnabled?: boolean;
  fastModeDescription?: string;
}

type ConfigState = Pick<
  NewSessionResponse | SetSessionConfigOptionResponse | ConfigOptionUpdate,
  "configOptions"
>;

export function readModelState(value: ConfigState): AcpModelState {
  const configOptions = value.configOptions ?? [];
  const modelConfig = configOptions.find((option) => option.category === "model");
  const reasoningConfig = configOptions.find(isReasoningConfig);
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
    fastModeEnabled: fastModeConfig?.type === "boolean" ? fastModeConfig.currentValue : undefined,
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

function isReasoningConfig(option: SessionConfigOption) {
  const category = option.category?.toLowerCase().replaceAll("-", "_");
  if (category === "thought_level" || category === "thoughtlevel") return true;
  if (category !== "model_config") return false;
  return /reason|thought|effort/.test(`${option.id} ${option.name}`.toLowerCase());
}

function isFastModeConfig(option: SessionConfigOption) {
  return option.type === "boolean" && /fast/.test(`${option.id} ${option.name}`.toLowerCase());
}
