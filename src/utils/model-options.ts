import type { FastModeOption, ReasoningModelOption } from "../types/index.ts";
import type { AcpModelState } from "./model-state.ts";

interface ModelConfigConnection {
  setModel: (configId: string, modelId: string) => Promise<AcpModelState>;
}

export async function discoverModelOptions(
  connection: ModelConfigConnection,
  state: AcpModelState,
) {
  const reasoningOptionsByModel: Record<string, ReasoningModelOption> = {};
  const fastModeOptionsByModel: Record<string, FastModeOption> = {};
  if (!state.modelConfigId) return { reasoningOptionsByModel, fastModeOptionsByModel };

  for (const model of state.models) {
    try {
      const modelState =
        model.id === state.selectedModelId
          ? state
          : await connection.setModel(state.modelConfigId, model.id);
      if (modelState.selectedModelId !== model.id) continue;
      reasoningOptionsByModel[model.id] = {
        options: modelState.reasoningOptions,
        selectedId: modelState.selectedReasoningId,
      };
      if (!modelState.fastModeConfigId || modelState.fastModeEnabled === undefined) continue;
      fastModeOptionsByModel[model.id] = {
        configId: modelState.fastModeConfigId,
        enabled: modelState.fastModeEnabled,
        description: modelState.fastModeDescription,
      };
    } catch {
      // A model that cannot be selected during discovery is simply unavailable.
    }
  }
  return { reasoningOptionsByModel, fastModeOptionsByModel };
}
