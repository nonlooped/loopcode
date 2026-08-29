import { z } from "zod";

import type { RateLimitState } from "../types/index.ts";

const PROVIDER_USAGE_KEY = "loopcode.provider-usage";

export interface ProviderUsageRecord {
  /** Epoch milliseconds when the provider last reported these windows. */
  updatedAt: number;
  limits: RateLimitState[];
}

export type ProviderUsage = Record<string, ProviderUsageRecord>;

const providerUsageSchema = z.record(
  z.string().min(1),
  z.object({
    updatedAt: z.number(),
    limits: z.array(
      z.object({
        id: z.string().min(1),
        name: z.string().min(1),
        primary: z
          .object({ usedPercent: z.number(), resetsAt: z.number().nullable().optional() })
          .nullable()
          .optional(),
      }),
    ),
  }),
);

/** `localStorage` is absent in tests and in non-browser runtimes. */
const webStorage = () => globalThis.localStorage as Storage | undefined;

export function loadProviderUsage(storage = webStorage()): ProviderUsage {
  try {
    const stored = storage?.getItem(PROVIDER_USAGE_KEY);
    return stored ? (providerUsageSchema.safeParse(JSON.parse(stored)).data ?? {}) : {};
  } catch {
    return {};
  }
}

/** Returns the updated record so callers can mirror it into reactive state. */
export function recordProviderUsage(
  usage: ProviderUsage,
  profileId: string,
  record: ProviderUsageRecord,
  storage = webStorage(),
): ProviderUsage {
  const next = { ...usage, [profileId]: record };
  try {
    storage?.setItem(PROVIDER_USAGE_KEY, JSON.stringify(next));
  } catch {
    // Usage persistence is best-effort when web storage is unavailable.
  }
  return next;
}

/**
 * Codex rollout entries, as written by Codex itself. `codex-acp` never forwards these over ACP,
 * so the rollout file is the only structured copy a client can read.
 */
const codexWindowSchema = z
  .object({
    used_percent: z.number(),
    window_minutes: z.number().nullable().optional(),
    resets_at: z.number().nullable().optional(),
  })
  .nullable()
  .optional();

export const codexRateLimitsSchema = z
  .object({
    capturedAt: z.string().nullable().optional(),
    rateLimits: z.object({
      limit_id: z.string().nullable().optional(),
      primary: codexWindowSchema,
      secondary: codexWindowSchema,
    }),
  })
  .nullable();

/** Mirrors the window naming Codex uses in its own `/status` output. */
function codexWindowName(windowMinutes: number | null | undefined) {
  if (!windowMinutes) return "limit";
  if (windowMinutes < 60) return `${windowMinutes}m`;
  if (windowMinutes < 1440) return `${Math.round(windowMinutes / 60)}h`;
  if (windowMinutes < 10080) return `${Math.round(windowMinutes / 1440)}d`;
  return "weekly";
}

export function codexUsageRecord(
  payload: z.infer<typeof codexRateLimitsSchema>,
): ProviderUsageRecord | undefined {
  if (!payload) return;
  const { limit_id: limitId, primary, secondary } = payload.rateLimits;
  const limits = (
    [
      ["primary", primary],
      ["secondary", secondary],
    ] as const
  ).flatMap(([slot, window]) =>
    window
      ? [
          {
            id: `${limitId ?? "codex"}-${slot}`,
            name: codexWindowName(window.window_minutes),
            primary: { usedPercent: window.used_percent, resetsAt: window.resets_at ?? null },
          },
        ]
      : [],
  );
  if (limits.length === 0) return;
  const capturedAt = payload.capturedAt ? Date.parse(payload.capturedAt) : Number.NaN;
  return { updatedAt: Number.isNaN(capturedAt) ? Date.now() : capturedAt, limits };
}
