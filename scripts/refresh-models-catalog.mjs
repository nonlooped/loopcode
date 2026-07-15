/**
 * Refresh the bundled models.dev catalog snapshot.
 *
 * Usage: node scripts/refresh-models-catalog.mjs
 *
 * Fetches https://models.dev/api.json, filters to chat-capable models, and
 * writes src-tauri/assets/catalog/models-dev-snapshot.json.
 * npm package names are rewritten with a DO_NOT_EXECUTE_ prefix so Core never
 * treats catalog metadata as installable/runnable recipes.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const OUT = path.join(ROOT, "src-tauri/assets/catalog/models-dev-snapshot.json");
const SOURCE = "https://models.dev/api.json";

const HERO_ORDER = ["openai", "anthropic", "openrouter", "opencode"];

function isChatModel(m) {
  const out = m.modalities?.output ?? [];
  if (!out.includes("text")) return false;
  const id = (m.id || "").toLowerCase();
  const family = (m.family || "").toLowerCase();
  if (/embedding|whisper|tts|transcribe|dall-e|image|realtime|moderation|omni-moderation/.test(id)) {
    return false;
  }
  if (/embedding|whisper|tts|image/.test(family)) return false;
  return true;
}

// Canonical low→high order for effort levels seen in models.dev data.
const EFFORT_ORDER = ["none", "minimal", "low", "medium", "high", "xhigh", "max"];

/**
 * Normalize models.dev `reasoning_options` into flat, defensive fields:
 * - reasoningEfforts: known effort names, deduped, canonical low→high order
 * - reasoningToggle: model supports a plain on/off switch
 * The upstream data has warts (null entries, descending lists) — sort against
 * the canonical order instead of trusting the array.
 */
function reasoningFields(m) {
  const out = { reasoning: Boolean(m.reasoning), reasoningEfforts: [], reasoningToggle: false };
  for (const opt of Array.isArray(m.reasoning_options) ? m.reasoning_options : []) {
    if (!opt || typeof opt !== "object") continue;
    if (opt.type === "toggle") out.reasoningToggle = true;
    if (opt.type === "effort" && Array.isArray(opt.values)) {
      out.reasoningEfforts = EFFORT_ORDER.filter((e) => opt.values.includes(e));
    }
  }
  return out;
}

// models.dev provides no per-provider default, so the default is simply the
// first model in our sorted list (newest release first).
function pickDefault(models) {
  return models[0]?.id ?? null;
}

const res = await fetch(SOURCE);
if (!res.ok) {
  console.error(`Failed to fetch ${SOURCE}: ${res.status}`);
  process.exit(1);
}
const api = await res.json();

const providers = [];
for (const [id, p] of Object.entries(api)) {
  if (!p || typeof p !== "object" || !p.models) continue;
  const chatModels = Object.values(p.models).filter(isChatModel);
  if (chatModels.length === 0) continue;

  chatModels.sort((a, b) => {
    const rd = (b.release_date || "").localeCompare(a.release_date || "");
    if (rd) return rd;
    return (a.name || a.id).localeCompare(b.name || b.id);
  });

  const models = chatModels.map((m) => ({
    id: m.id,
    name: m.name || m.id,
    default: false,
    ...reasoningFields(m),
  }));

  const defaultModel = pickDefault(models);
  if (defaultModel) {
    const hit = models.find((m) => m.id === defaultModel);
    if (hit) hit.default = true;
  }

  const npm = p.npm ? `DO_NOT_EXECUTE_${String(p.npm).replace(/^@/, "")}` : "DO_NOT_EXECUTE";

  providers.push({
    id,
    name: p.name || id,
    docsUrl: p.doc || null,
    api: p.api || null,
    npm,
    env: Array.isArray(p.env) ? p.env : null,
    defaultModel,
    models,
  });
}

providers.sort((a, b) => {
  const ai = HERO_ORDER.indexOf(a.id);
  const bi = HERO_ORDER.indexOf(b.id);
  if (ai === -1 && bi === -1) return a.name.localeCompare(b.name);
  if (ai === -1) return 1;
  if (bi === -1) return -1;
  return ai - bi;
});

const snapshot = {
  snapshotVersion: `models-dev-${new Date().toISOString().slice(0, 10)}`,
  source: `${SOURCE} (bundled snapshot; advisory only; npm never executed)`,
  generatedAt: new Date().toISOString(),
  providers,
};

fs.mkdirSync(path.dirname(OUT), { recursive: true });
fs.writeFileSync(OUT, `${JSON.stringify(snapshot, null, 2)}\n`);

const heroes = HERO_ORDER.map((id) => providers.find((p) => p.id === id)).filter(Boolean);
console.log(`Wrote ${path.relative(ROOT, OUT)}`);
console.log(`providers=${providers.length} sizeKB=${Math.round(fs.statSync(OUT).size / 1024)}`);
for (const h of heroes) {
  console.log(`  ${h.id}: ${h.models.length} models, default=${h.defaultModel}`);
}
