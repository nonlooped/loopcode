import type { FastModeOption, ModelOption, ReasoningModelOption } from "../types/index.ts";
import type { AcpModelState } from "./model-state.ts";

interface ModelConfigConnection {
  setModel: (configId: string, modelId: string) => Promise<AcpModelState>;
}

type ModelState = AcpModelState & { model: ModelOption };

export function modelOptionsFromStates(states: ModelState[]) {
  const reasoningOptionsByModel: Record<string, ReasoningModelOption> = {};
  const fastModeOptionsByModel: Record<string, FastModeOption> = {};
  for (const { model, ...state } of states) {
    reasoningOptionsByModel[model.id] = {
      options: state.reasoningOptions,
      selectedId: state.selectedReasoningId,
    };
    if (!state.fastModeConfigId || state.fastModeEnabled === undefined) continue;
    fastModeOptionsByModel[model.id] = {
      configId: state.fastModeConfigId,
      enabled: state.fastModeEnabled,
      valueType: state.fastModeValueType,
      description: state.fastModeDescription,
    };
  }
  return { reasoningOptionsByModel, fastModeOptionsByModel };
}

export async function discoverModelOptions(
  connection: ModelConfigConnection,
  state: AcpModelState,
) {
  if (!state.modelConfigId) return modelOptionsFromStates([]);
  const states: ModelState[] = [];
  for (const model of state.models) {
    try {
      const modelState =
        model.id === state.selectedModelId
          ? state
          : await connection.setModel(state.modelConfigId, model.id);
      if (modelState.selectedModelId === model.id) states.push({ model, ...modelState });
    } catch {
      // A model that cannot be selected during discovery is simply unavailable.
    }
  }
  return modelOptionsFromStates(states);
}
