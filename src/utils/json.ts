import { z } from "zod";

export const jsonValueSchema = z.json();
export type JsonValue = z.infer<typeof jsonValueSchema>;
export type JsonObject = { [key: string]: JsonValue };

export function isObject(value: JsonValue): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function stringValue(value: JsonValue | undefined) {
  return z.string().min(1).safeParse(value).data;
}

export function finiteNumber(value: JsonValue | undefined) {
  return z.number().finite().safeParse(value).data;
}

export function formattedValue(value: JsonValue) {
  const text = z.string().safeParse(value);
  if (text.success) return text.data;
  if (isObject(value) || Array.isArray(value)) return JSON.stringify(value, null, 2);
  return undefined;
}
