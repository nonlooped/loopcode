import type { HighlighterCore, LanguageInput } from "shiki/core";

const languageLoaders: Record<string, () => Promise<LanguageInput>> = {
  bash: () => import("shiki/langs/bash.mjs").then((module) => module.default),
  css: () => import("shiki/langs/css.mjs").then((module) => module.default),
  diff: () => import("shiki/langs/diff.mjs").then((module) => module.default),
  html: () => import("shiki/langs/html.mjs").then((module) => module.default),
  javascript: () => import("shiki/langs/javascript.mjs").then((module) => module.default),
  json: () => import("shiki/langs/json.mjs").then((module) => module.default),
  markdown: () => import("shiki/langs/markdown.mjs").then((module) => module.default),
  powershell: () => import("shiki/langs/powershell.mjs").then((module) => module.default),
  python: () => import("shiki/langs/python.mjs").then((module) => module.default),
  rust: () => import("shiki/langs/rust.mjs").then((module) => module.default),
  svelte: () => import("shiki/langs/svelte.mjs").then((module) => module.default),
  toml: () => import("shiki/langs/toml.mjs").then((module) => module.default),
  tsx: () => import("shiki/langs/tsx.mjs").then((module) => module.default),
  typescript: () => import("shiki/langs/typescript.mjs").then((module) => module.default),
  yaml: () => import("shiki/langs/yaml.mjs").then((module) => module.default),
};

const languages: Record<string, string> = {
  bash: "bash",
  cjs: "javascript",
  css: "css",
  cts: "typescript",
  diff: "diff",
  htm: "html",
  html: "html",
  js: "javascript",
  javascript: "javascript",
  json: "json",
  jsonc: "json",
  md: "markdown",
  markdown: "markdown",
  mjs: "javascript",
  mts: "typescript",
  patch: "diff",
  powershell: "powershell",
  ps1: "powershell",
  py: "python",
  python: "python",
  rs: "rust",
  rust: "rust",
  sh: "bash",
  shell: "bash",
  svelte: "svelte",
  toml: "toml",
  ts: "typescript",
  tsx: "tsx",
  typescript: "typescript",
  yaml: "yaml",
  yml: "yaml",
};

const languageLoads = new Map<string, Promise<HighlighterCore>>();
let highlighterLoad: Promise<HighlighterCore> | undefined;

export function normalizeLanguage(language: string): string {
  return languages[language.trim().toLowerCase()] ?? "";
}

export function languageForPath(path: string): string {
  return normalizeLanguage(path.split(".").pop() ?? "");
}

export function fallbackHighlight(content: string): string {
  const lines = content
    .split("\n")
    .map((line) => `<span class="line">${escapeHtml(line) || " "}</span>`)
    .join("\n");
  return `<pre class="shiki-fallback"><code>${lines}</code></pre>`;
}

export async function highlightCode(content: string, requestedLanguage: string): Promise<string> {
  const language = normalizeLanguage(requestedLanguage);
  if (!language) return fallbackHighlight(content);
  try {
    const highlighter = await loadLanguage(language);
    return highlighter.codeToHtml(content, { lang: language, theme: "github-dark" });
  } catch {
    return fallbackHighlight(content);
  }
}

export async function highlightFile(content: string, path: string): Promise<string> {
  const language = languageForPath(path);
  return language ? highlightCode(content, language) : fallbackHighlight(content);
}

function loadLanguage(language: string): Promise<HighlighterCore> {
  const existing = languageLoads.get(language);
  if (existing) return existing;

  const loading = Promise.all([languageLoaders[language](), getHighlighter()])
    .then(async ([grammar, highlighter]) => {
      await highlighter.loadLanguage(grammar);
      return highlighter;
    })
    .catch((cause) => {
      languageLoads.delete(language);
      throw cause;
    });
  languageLoads.set(language, loading);
  return loading;
}

function getHighlighter(): Promise<HighlighterCore> {
  if (highlighterLoad) return highlighterLoad;

  const loading = Promise.all([
    import("shiki/core"),
    import("shiki/engine/javascript"),
    import("shiki/themes/github-dark.mjs"),
  ])
    .then(([shiki, engine, theme]) =>
      shiki.createHighlighterCore({
        engine: engine.createJavaScriptRegexEngine(),
        themes: [theme.default],
      }),
    )
    .catch((cause) => {
      if (highlighterLoad === loading) highlighterLoad = undefined;
      throw cause;
    });
  highlighterLoad = loading;
  return loading;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
