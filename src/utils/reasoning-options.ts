import type { ProviderSessionState } from "../types/index.ts";

export function applyReasoningForSelectedModel(provider: ProviderSessionState) {
  if (!provider.selectedModelId || !provider.reasoningOptionsByModel) return;
  const state = provider.reasoningOptionsByModel[provider.selectedModelId];
  provider.reasoningOptions = state?.options ?? [];
  if (
    !provider.selectedReasoningId ||
    !provider.reasoningOptions.some((option) => option.id === provider.selectedReasoningId)
  ) {
    provider.selectedReasoningId = state?.selectedId;
  }
}
