import type { FastModeOption, ReasoningModelOption } from "../types/index.ts";
import type { AcpModelState } from "./model-state.ts";

interface ModelConfigConnection {
  setModel: (configId: string, modelId: string) => Promise<AcpModelState>;
}

export async function discoverModelOptions(
  connection: ModelConfigConnection,
  state: AcpModelState,
  selectedModelId: string,
) {
  const reasoningOptionsByModel: Record<string, ReasoningModelOption> = {};
  const fastModeOptionsByModel: Record<string, FastModeOption> = {};
  if (!state.modelConfigId) return { reasoningOptionsByModel, fastModeOptionsByModel };

  try {
    const modelState =
      selectedModelId === state.selectedModelId
        ? state
        : await connection.setModel(state.modelConfigId, selectedModelId);
    if (modelState.selectedModelId !== selectedModelId) {
      return { reasoningOptionsByModel, fastModeOptionsByModel };
    }
    reasoningOptionsByModel[selectedModelId] = {
      options: modelState.reasoningOptions,
      selectedId: modelState.selectedReasoningId,
    };
    if (modelState.fastModeConfigId && modelState.fastModeEnabled !== undefined) {
      fastModeOptionsByModel[selectedModelId] = {
        configId: modelState.fastModeConfigId,
        enabled: modelState.fastModeEnabled,
        description: modelState.fastModeDescription,
      };
    }
  } catch {
    // Capability probing is best-effort; selection still uses the advertised model list.
  }
  return { reasoningOptionsByModel, fastModeOptionsByModel };
}
