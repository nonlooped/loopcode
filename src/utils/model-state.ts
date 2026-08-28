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
  modeConfigId?: string;
  modes?: ModelOption[];
  selectedModeId?: string;
  collaborationConfigId?: string;
  collaborationModes?: ModelOption[];
  selectedCollaborationModeId?: string;
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
  const modelConfig = findModelConfig(configOptions);
  const reasoningConfig =
    configOptions.find(isExplicitReasoningConfig) ?? configOptions.find(isReasoningConfig);
  const fastModeConfig = configOptions.find(isFastModeConfig);
  const modeConfig = configOptions.find(isModeConfig);
  const collaborationConfig = configOptions.find(isCollaborationModeConfig);

  return {
    modelConfigId: modelConfig?.id,
    models: configChoices(modelConfig),
    selectedModelId: selectedConfigValue(modelConfig),
    reasoningConfigId: reasoningConfig?.id,
    reasoningOptions: configChoices(reasoningConfig),
    selectedReasoningId: selectedConfigValue(reasoningConfig),
    fastModeConfigId: fastModeConfig?.id,
    fastModeEnabled: configBooleanValue(fastModeConfig),
    fastModeValueType: configValueType(fastModeConfig),
    fastModeDescription: fastModeConfig?.description ?? undefined,
    modeConfigId: modeConfig?.id,
    modes: configChoices(modeConfig),
    selectedModeId: selectedConfigValue(modeConfig),
    collaborationConfigId: collaborationConfig?.id,
    collaborationModes: configChoices(collaborationConfig),
    selectedCollaborationModeId: selectedConfigValue(collaborationConfig),
  };
}

/** Codex's approval and sandbox preset: read-only, workspace write, or full access. */
function isModeConfig(option: SessionConfigOption) {
  if (option.type !== "select") return false;
  return option.category === "mode" || option.id.toLowerCase() === "mode";
}

/** Codex's plan-versus-default collaboration preset for subsequent turns. */
function isCollaborationModeConfig(option: SessionConfigOption) {
  if (option.type !== "select") return false;
  const key = `${option.id} ${option.category ?? ""}`.toLowerCase().replaceAll("-", "_");
  return key.includes("collaboration_mode");
}

function findModelConfig(configOptions: SessionConfigOption[]) {
  const modelConfigs = configOptions.filter(
    (option) => option.type === "select" && option.category === "model",
  );
  return (
    modelConfigs.find((option) => option.id.toLowerCase() === "model") ??
    modelConfigs.find((option) => option.name.toLowerCase() === "model") ??
    modelConfigs.find((option) => !/provider/.test(`${option.id} ${option.name}`.toLowerCase())) ??
    modelConfigs[0]
  );
}

function selectedConfigValue(config: SessionConfigOption | undefined) {
  return config?.type === "select" ? config.currentValue : undefined;
}

function configValueType(config: SessionConfigOption | undefined): FastModeValueType | undefined {
  if (config?.type === "boolean") return "boolean";
  if (config?.type === "select") return "string";
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
