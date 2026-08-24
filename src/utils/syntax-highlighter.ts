import bash from "shiki/langs/bash.mjs";
import css from "shiki/langs/css.mjs";
import diff from "shiki/langs/diff.mjs";
import html from "shiki/langs/html.mjs";
import javascript from "shiki/langs/javascript.mjs";
import json from "shiki/langs/json.mjs";
import markdown from "shiki/langs/markdown.mjs";
import powershell from "shiki/langs/powershell.mjs";
import python from "shiki/langs/python.mjs";
import rust from "shiki/langs/rust.mjs";
import svelte from "shiki/langs/svelte.mjs";
import toml from "shiki/langs/toml.mjs";
import tsx from "shiki/langs/tsx.mjs";
import typescript from "shiki/langs/typescript.mjs";
import yaml from "shiki/langs/yaml.mjs";
import { createHighlighterCoreSync } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import githubDark from "shiki/themes/github-dark.mjs";
import githubLight from "shiki/themes/github-light.mjs";
import { escapeHtml, type ShikiHighlighter } from "@humanspeak/svelte-markdown/extensions/shiki";

const highlighter = createHighlighterCoreSync({
  engine: createJavaScriptRegexEngine(),
  langs: [
    bash,
    css,
    diff,
    html,
    javascript,
    json,
    markdown,
    powershell,
    python,
    rust,
    svelte,
    toml,
    tsx,
    typescript,
    yaml,
  ],
  themes: [githubDark, githubLight],
});
const loadedLanguages = new Set(highlighter.getLoadedLanguages());

export const syntaxHighlighter: ShikiHighlighter = {
  hasLang: (language) => loadedLanguages.has(language),
  highlight(code, language) {
    if (!language || !loadedLanguages.has(language)) {
      return `<pre class="shiki-fallback"><code>${escapeHtml(code)}</code></pre>`;
    }
    try {
      return highlighter.codeToHtml(code, {
        lang: language,
        themes: { light: "github-light", dark: "github-dark" },
        defaultColor: false,
      });
    } catch {
      return `<pre class="shiki-fallback"><code>${escapeHtml(code)}</code></pre>`;
    }
  },
};

const languages = {
  bash: "bash",
  cjs: "javascript",
  css: "css",
  cts: "typescript",
  diff: "diff",
  htm: "html",
  html: "html",
  js: "javascript",
  json: "json",
  jsonc: "json",
  markdown: "markdown",
  md: "markdown",
  mjs: "javascript",
  mts: "typescript",
  patch: "diff",
  ps1: "powershell",
  py: "python",
  rs: "rust",
  sh: "bash",
  svelte: "svelte",
  toml: "toml",
  ts: "typescript",
  tsx: "tsx",
  yaml: "yaml",
  yml: "yaml",
};

export function languageForPath(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  if (!Object.hasOwn(languages, extension)) return "";
  // SAFETY: Object.hasOwn established that extension is present in languages.
  return languages[extension as keyof typeof languages];
}

export function highlightFile(content: string, path: string): string {
  const language = languageForPath(path);
  if (language && syntaxHighlighter.hasLang(language)) {
    return syntaxHighlighter.highlight(content, language);
  }
  const lines = content
    .split("\n")
    .map((line) => `<span class="line">${escapeHtml(line) || " "}</span>`)
    .join("\n");
  return `<pre class="shiki-fallback"><code>${lines}</code></pre>`;
}
