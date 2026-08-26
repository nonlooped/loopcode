import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite-plus";

import { materialIconMapsFromManifest } from "./src/utils/material-icon-lookup.ts";

const tauriHost = process.env.TAURI_DEV_HOST;
const ignorePatterns = [
  ".agent/**",
  ".agents/**",
  ".claude/**",
  ".codex/**",
  ".continue/**",
  ".cursor/**",
  ".gemini/**",
  ".opencode/**",
  ".pi/**",
  ".roo/**",
  ".windsurf/**",
  "src-tauri/gen/**",
];

function materialIconManifest() {
  const id = "virtual:material-icon-manifest";
  const resolved = `\0${id}`;
  return {
    name: "material-icon-manifest",
    resolveId(source: string) {
      if (source === id) return resolved;
    },
    async load(source: string) {
      if (source !== resolved) return;
      const { generateManifest } = await import("material-icon-theme");
      return `export default ${JSON.stringify(materialIconMapsFromManifest(generateManifest()))};`;
    },
  };
}

export default defineConfig({
  // SAFETY: vite-plus currently bundles a different Vite plugin type version than Svelte.
  plugins: [svelte() as never, materialIconManifest()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: tauriHost || false,
    hmr: tauriHost
      ? {
          protocol: "ws",
          host: tauriHost,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: process.env.TAURI_ENV_DEBUG ? false : "oxc",
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG),
  },
  lint: {
    ignorePatterns,
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  fmt: {
    ignorePatterns,
  },
});
