import type { HarnessProfile, ModelOption, PermissionMode } from "../types/index.ts";

const PERMISSION_MODE_KEY = "loopcode.permission-mode";
const LEFT_SIDEBAR_WIDTH_KEY = "loopcode.left-sidebar-width";
const RIGHT_SIDEBAR_WIDTH_KEY = "loopcode.right-sidebar-width";
const TERMINAL_HEIGHT_KEY = "loopcode.terminal-height";

export type SettingsCategory =
  | "general"
  | "appearance"
  | "conversation"
  | "composer"
  | "agents"
  | "providers"
  | "terminal"
  | "diagnostics";
export type SendShortcut = "enter" | "modifier-enter";
export type ColorMode = "system" | "light" | "dark";

export const THEME_OPTIONS = [
  { id: "graphite", label: "Graphite" },
  { id: "rose", label: "Rose" },
  { id: "sand", label: "Sand" },
  { id: "ember", label: "Ember" },
  { id: "amber", label: "Amber" },
  { id: "forest", label: "Forest" },
  { id: "teal", label: "Teal" },
  { id: "ocean", label: "Ocean" },
  { id: "indigo", label: "Indigo" },
  { id: "violet", label: "Violet" },
] as const;

export type ThemeId = (typeof THEME_OPTIONS)[number]["id"];

export interface ProviderPreference {
  enabled?: boolean;
  command?: string;
  models?: ModelOption[];
}

export interface AppPreferences {
  colorMode: ColorMode;
  theme: ThemeId;
  compactSessionRows: boolean;
  startupBehavior: "last-thread" | "new-thread";
  newThreadProject: "selected" | "default-folder";
  defaultWorkingFolder: string;
  motionMode: "system" | "reduced";
  interfaceZoom: number;
  transcriptDensity: "comfortable" | "compact";
  contentWidth: number;
  wrapCode: boolean;
  showMessageTimestamps: boolean;
  composerSpellcheck: boolean;
  defaultProviderId: string;
  automaticTitleGeneration: boolean;
  providerModelDefaults: Record<string, string>;
  providerSettings: Record<string, ProviderPreference>;
  titleProviderId: string;
  titleModelId: string;
  terminalFontSize: number;
  terminalScrollback: number;
  sendShortcut: SendShortcut;
}

export const DEFAULT_APP_PREFERENCES: AppPreferences = {
  colorMode: "system",
  theme: "violet",
  compactSessionRows: false,
  startupBehavior: "last-thread",
  newThreadProject: "selected",
  defaultWorkingFolder: "",
  motionMode: "system",
  interfaceZoom: 100,
  transcriptDensity: "comfortable",
  contentWidth: 720,
  wrapCode: true,
  showMessageTimestamps: false,
  composerSpellcheck: true,
  defaultProviderId: "codex",
  automaticTitleGeneration: true,
  providerModelDefaults: {},
  providerSettings: {},
  titleProviderId: "codex",
  titleModelId: "",
  terminalFontSize: 12,
  terminalScrollback: 5_000,
  sendShortcut: "enter",
};

const LEGACY_PREFERENCE_KEYS = [
  "loopcode.auto-follow-output",
  "loopcode.composer-autocomplete",
] as const;

const PREFERENCE_KEYS: Record<keyof AppPreferences, string> = {
  colorMode: "loopcode.color-mode",
  theme: "loopcode.theme",
  compactSessionRows: "loopcode.compact-session-rows",
  startupBehavior: "loopcode.startup-behavior",
  newThreadProject: "loopcode.new-thread-project",
  defaultWorkingFolder: "loopcode.default-working-folder",
  motionMode: "loopcode.motion-mode",
  interfaceZoom: "loopcode.interface-zoom",
  transcriptDensity: "loopcode.transcript-density",
  contentWidth: "loopcode.content-width",
  wrapCode: "loopcode.wrap-code",
  showMessageTimestamps: "loopcode.show-message-timestamps",
  composerSpellcheck: "loopcode.composer-spellcheck",
  defaultProviderId: "loopcode.default-provider",
  automaticTitleGeneration: "loopcode.automatic-title-generation",
  providerModelDefaults: "loopcode.provider-model-defaults",
  providerSettings: "loopcode.provider-settings",
  titleProviderId: "loopcode.title-provider",
  titleModelId: "loopcode.title-model",
  terminalFontSize: "loopcode.terminal-font-size",
  terminalScrollback: "loopcode.terminal-scrollback",
  sendShortcut: "loopcode.send-shortcut",
};

export const LEFT_SIDEBAR_WIDTH_RANGE = { min: 190, max: 480 } as const;
export const RIGHT_SIDEBAR_WIDTH_RANGE = { min: 210, max: 520 } as const;
export const TERMINAL_HEIGHT_RANGE = { min: 140, max: 520 } as const;
export const DEFAULT_TERMINAL_HEIGHT = 260;
export const INTERFACE_ZOOM_RANGE = { min: 60, max: 200 } as const;
export const CONTENT_WIDTH_RANGE = { min: 600, max: 900 } as const;
export const TERMINAL_FONT_SIZE_RANGE = { min: 10, max: 16 } as const;
export interface SidebarWidths {
  left: number | null;
  right: number | null;
}

export function loadAppPreferences(
  storage: Pick<Storage, "getItem"> = localStorage,
): AppPreferences {
  try {
    return {
      colorMode: storedColorMode(storage.getItem(PREFERENCE_KEYS.colorMode)),
      theme: storedTheme(storage.getItem(PREFERENCE_KEYS.theme)),
      compactSessionRows:
        storedBoolean(storage.getItem(PREFERENCE_KEYS.compactSessionRows)) ??
        DEFAULT_APP_PREFERENCES.compactSessionRows,
      startupBehavior:
        storage.getItem(PREFERENCE_KEYS.startupBehavior) === "new-thread"
          ? "new-thread"
          : "last-thread",
      newThreadProject:
        storage.getItem(PREFERENCE_KEYS.newThreadProject) === "default-folder"
          ? "default-folder"
          : "selected",
      defaultWorkingFolder: storedPath(storage.getItem(PREFERENCE_KEYS.defaultWorkingFolder)),
      motionMode: storage.getItem(PREFERENCE_KEYS.motionMode) === "reduced" ? "reduced" : "system",
      interfaceZoom:
        storedNumber(storage.getItem(PREFERENCE_KEYS.interfaceZoom), INTERFACE_ZOOM_RANGE) ??
        DEFAULT_APP_PREFERENCES.interfaceZoom,
      transcriptDensity:
        storage.getItem(PREFERENCE_KEYS.transcriptDensity) === "compact"
          ? "compact"
          : "comfortable",
      contentWidth:
        storedNumber(storage.getItem(PREFERENCE_KEYS.contentWidth), CONTENT_WIDTH_RANGE) ??
        DEFAULT_APP_PREFERENCES.contentWidth,
      wrapCode:
        storedBoolean(storage.getItem(PREFERENCE_KEYS.wrapCode)) ??
        DEFAULT_APP_PREFERENCES.wrapCode,
      showMessageTimestamps:
        storedBoolean(storage.getItem(PREFERENCE_KEYS.showMessageTimestamps)) ??
        DEFAULT_APP_PREFERENCES.showMessageTimestamps,
      composerSpellcheck:
        storedBoolean(storage.getItem(PREFERENCE_KEYS.composerSpellcheck)) ??
        DEFAULT_APP_PREFERENCES.composerSpellcheck,
      defaultProviderId:
        storage.getItem(PREFERENCE_KEYS.defaultProviderId)?.trim() ||
        DEFAULT_APP_PREFERENCES.defaultProviderId,
      automaticTitleGeneration:
        storedBoolean(storage.getItem(PREFERENCE_KEYS.automaticTitleGeneration)) ??
        DEFAULT_APP_PREFERENCES.automaticTitleGeneration,
      providerModelDefaults: storedStringRecord(
        storage.getItem(PREFERENCE_KEYS.providerModelDefaults),
      ),
      providerSettings: storedProviderSettings(storage.getItem(PREFERENCE_KEYS.providerSettings)),
      titleProviderId:
        storage.getItem(PREFERENCE_KEYS.titleProviderId)?.trim() ||
        DEFAULT_APP_PREFERENCES.titleProviderId,
      titleModelId: storage.getItem(PREFERENCE_KEYS.titleModelId)?.trim() ?? "",
      terminalFontSize:
        storedNumber(storage.getItem(PREFERENCE_KEYS.terminalFontSize), TERMINAL_FONT_SIZE_RANGE) ??
        DEFAULT_APP_PREFERENCES.terminalFontSize,
      terminalScrollback: storedScrollback(storage.getItem(PREFERENCE_KEYS.terminalScrollback)),
      sendShortcut:
        storage.getItem(PREFERENCE_KEYS.sendShortcut) === "modifier-enter"
          ? "modifier-enter"
          : "enter",
    };
  } catch {
    return { ...DEFAULT_APP_PREFERENCES, providerModelDefaults: {}, providerSettings: {} };
  }
}

export function saveAppPreference<K extends keyof AppPreferences>(
  key: K,
  value: AppPreferences[K],
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  saveSetting(
    storage,
    PREFERENCE_KEYS[key],
    typeof value === "object" ? JSON.stringify(value) : String(value),
  );
}

export function resetAppSettings(storage: Pick<Storage, "removeItem"> = localStorage) {
  try {
    for (const key of [
      ...Object.values(PREFERENCE_KEYS),
      ...LEGACY_PREFERENCE_KEYS,
      PERMISSION_MODE_KEY,
      LEFT_SIDEBAR_WIDTH_KEY,
      RIGHT_SIDEBAR_WIDTH_KEY,
      TERMINAL_HEIGHT_KEY,
    ])
      storage.removeItem(key);
  } catch {
    // Settings persistence is best-effort when web storage is unavailable.
  }
}

export function loadPermissionMode(
  storage: Pick<Storage, "getItem"> = localStorage,
): PermissionMode {
  try {
    return storage.getItem(PERMISSION_MODE_KEY) === "full" ? "full" : "restricted";
  } catch {
    return "restricted";
  }
}

export function savePermissionMode(
  mode: PermissionMode,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  saveSetting(storage, PERMISSION_MODE_KEY, mode);
}

export function loadSidebarWidths(storage: Pick<Storage, "getItem"> = localStorage): SidebarWidths {
  try {
    return {
      left: storedNumber(storage.getItem(LEFT_SIDEBAR_WIDTH_KEY), LEFT_SIDEBAR_WIDTH_RANGE),
      right: storedNumber(storage.getItem(RIGHT_SIDEBAR_WIDTH_KEY), RIGHT_SIDEBAR_WIDTH_RANGE),
    };
  } catch {
    return { left: null, right: null };
  }
}

export function saveSidebarWidth(
  side: keyof SidebarWidths,
  width: number,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  saveSetting(
    storage,
    side === "left" ? LEFT_SIDEBAR_WIDTH_KEY : RIGHT_SIDEBAR_WIDTH_KEY,
    String(width),
  );
}

export function loadTerminalHeight(storage: Pick<Storage, "getItem"> = localStorage) {
  try {
    return (
      storedNumber(storage.getItem(TERMINAL_HEIGHT_KEY), TERMINAL_HEIGHT_RANGE) ??
      DEFAULT_TERMINAL_HEIGHT
    );
  } catch {
    return DEFAULT_TERMINAL_HEIGHT;
  }
}

export function saveTerminalHeight(
  height: number,
  storage: Pick<Storage, "setItem"> = localStorage,
) {
  saveSetting(storage, TERMINAL_HEIGHT_KEY, String(height));
}

function saveSetting(storage: Pick<Storage, "setItem">, key: string, value: string) {
  try {
    storage.setItem(key, value);
  } catch {
    // Settings persistence is best-effort when web storage is unavailable.
  }
}

function storedStringRecord(value: string | null) {
  if (!value) return {};
  try {
    const parsed: unknown = JSON.parse(value);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed).filter(
        (entry): entry is [string, string] =>
          entry[0].length > 0 &&
          entry[0] !== "__proto__" &&
          entry[0] !== "constructor" &&
          entry[0] !== "prototype" &&
          typeof entry[1] === "string" &&
          entry[1].length > 0,
      ),
    );
  } catch {
    return {};
  }
}

function storedProviderSettings(value: string | null): Record<string, ProviderPreference> {
  if (!value) return {};
  try {
    const parsed: unknown = JSON.parse(value);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed).flatMap(([profileId, candidate]) => {
        if (
          !profileId ||
          profileId === "__proto__" ||
          profileId === "constructor" ||
          profileId === "prototype" ||
          !candidate ||
          typeof candidate !== "object" ||
          Array.isArray(candidate)
        )
          return [];
        const setting: ProviderPreference = {};
        if ("enabled" in candidate && typeof candidate.enabled === "boolean") {
          setting.enabled = candidate.enabled;
        }
        if ("command" in candidate && typeof candidate.command === "string") {
          const command = candidate.command.trim().slice(0, 4096);
          if (command) setting.command = command;
        }
        if ("models" in candidate && Array.isArray(candidate.models)) {
          const models = candidate.models.slice(0, 100).flatMap((model: unknown) => {
            if (!model || typeof model !== "object" || Array.isArray(model)) return [];
            const id = "id" in model && typeof model.id === "string" ? model.id.trim() : "";
            const name = "name" in model && typeof model.name === "string" ? model.name.trim() : "";
            return id && name
              ? [{ id: id.slice(0, 256), name: name.slice(0, 256) } satisfies ModelOption]
              : [];
          });
          if (models.length > 0) setting.models = models;
        }
        return Object.keys(setting).length > 0 ? [[profileId, setting] as const] : [];
      }),
    );
  } catch {
    return {};
  }
}

export function configuredProviderProfiles(
  profiles: HarnessProfile[],
  settings: Record<string, ProviderPreference>,
) {
  return profiles.map((profile) => {
    const command = settings[profile.id]?.command || profile.command;
    return {
      ...profile,
      command,
      versionCommand: profile.versionCommand === profile.command ? command : profile.versionCommand,
    };
  });
}

export function providerVersionFromOutput(output: string) {
  return output.match(/\bv?(\d+\.\d+(?:\.\d+)?(?:[-+][0-9A-Za-z.-]+)?)\b/)?.[1];
}

function storedColorMode(value: string | null): ColorMode {
  return value === "light" || value === "dark" ? value : "system";
}

function storedTheme(value: string | null): ThemeId {
  return THEME_OPTIONS.find((theme) => theme.id === value)?.id ?? DEFAULT_APP_PREFERENCES.theme;
}

function storedScrollback(value: string | null) {
  const scrollback = Number(value);
  return scrollback === 1_000 || scrollback === 5_000 || scrollback === 10_000
    ? scrollback
    : DEFAULT_APP_PREFERENCES.terminalScrollback;
}

function storedBoolean(value: string | null) {
  if (value === "true") return true;
  if (value === "false") return false;
  return null;
}

function storedPath(value: string | null) {
  return value?.trim().slice(0, 4096) ?? "";
}

function storedNumber(value: string | null, range: { min: number; max: number }) {
  if (value === null) return null;
  const number = Number(value);
  if (!Number.isFinite(number)) return null;
  return Math.min(range.max, Math.max(range.min, number));
}
