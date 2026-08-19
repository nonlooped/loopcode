import type { ProviderSessionState } from "../types/index.ts";

export function fastModeAvailable(provider: ProviderSessionState) {
  return Boolean(
    provider.fastModeConfigId &&
    provider.fastModeModelId &&
    provider.selectedModelId === provider.fastModeModelId,
  );
}

export function applyFastModeForSelectedModel(provider: ProviderSessionState) {
  const modelId = provider.selectedModelId;
  const option = modelId ? provider.fastModeOptionsByModel?.[modelId] : undefined;
  provider.fastModeConfigId = option?.configId;
  provider.fastModeModelId = option ? modelId : undefined;
  provider.fastModeEnabled = option?.enabled;
  provider.fastModeValueType = option?.valueType;
  provider.fastModeDescription = option?.description;
}
