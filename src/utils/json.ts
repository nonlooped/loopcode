export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function stringValue(value: unknown) {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

export function finiteNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

export function formattedValue(value: unknown) {
  if (typeof value === "string") return value;
  if (isObject(value) || Array.isArray(value)) return JSON.stringify(value, null, 2);
  return undefined;
}
