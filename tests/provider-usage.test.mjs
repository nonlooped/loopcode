import assert from "node:assert/strict";
import test from "node:test";

import {
  codexUsageRecord,
  codexRateLimitsSchema,
  loadProviderUsage,
  recordProviderUsage,
} from "../src/utils/provider-usage.ts";

function memoryStorage() {
  const values = new Map();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };
}

void test("provider usage round-trips through storage and keeps other providers", () => {
  const storage = memoryStorage();
  assert.deepEqual(loadProviderUsage(storage), {});

  const limits = [{ id: "five_hour", name: "5h", primary: { usedPercent: 40, resetsAt: 12 } }];
  const usage = recordProviderUsage({}, "claude", { updatedAt: 1000, limits }, storage);
  recordProviderUsage(
    usage,
    "codex",
    { updatedAt: 2000, limits: [{ id: "weekly", name: "weekly", primary: null }] },
    storage,
  );

  const stored = loadProviderUsage(storage);
  assert.deepEqual(stored.claude.limits, limits);
  assert.deepEqual(stored.codex.limits[0].primary, null);
  assert.equal(stored.claude.updatedAt, 1000);
});

void test("unreadable or malformed usage falls back to an empty record", () => {
  const storage = memoryStorage();
  storage.setItem("loopcode.provider-usage", "{ not json");
  assert.deepEqual(loadProviderUsage(storage), {});

  storage.setItem("loopcode.provider-usage", JSON.stringify({ claude: { limits: "nope" } }));
  assert.deepEqual(loadProviderUsage(storage), {});

  assert.deepEqual(loadProviderUsage(undefined), {});
});

void test("Codex rollout limits map to both windows with Codex's own labels", () => {
  const record = codexUsageRecord(
    codexRateLimitsSchema.parse({
      capturedAt: "2026-08-29T11:06:56.569Z",
      rateLimits: {
        limit_id: "codex",
        primary: { used_percent: 27, window_minutes: 10080, resets_at: 1788453754 },
        secondary: { used_percent: 4.5, window_minutes: 300, resets_at: null },
      },
    }),
  );

  assert.equal(record.updatedAt, Date.parse("2026-08-29T11:06:56.569Z"));
  assert.deepEqual(record.limits, [
    { id: "codex-primary", name: "weekly", primary: { usedPercent: 27, resetsAt: 1788453754 } },
    { id: "codex-secondary", name: "5h", primary: { usedPercent: 4.5, resetsAt: null } },
  ]);
});

void test("a Codex rollout without windows produces no usage record", () => {
  assert.equal(codexUsageRecord(null), undefined);
  assert.equal(
    codexUsageRecord(codexRateLimitsSchema.parse({ rateLimits: { limit_id: "codex" } })),
    undefined,
  );
});
