#!/usr/bin/env node
import { readFileSync } from "node:fs";

export function classifyChangedPaths(changed) {
  const paths = changed.split(/\r?\n/).filter(Boolean);
  const native = paths.some(
    (path) =>
      path.startsWith("src-tauri/") || path === "rust-toolchain.toml" || path === "rust-toolchain",
  );
  const app = paths.some(
    (path) =>
      path.startsWith("src/") ||
      path.startsWith("src-tauri/") ||
      path === "rust-toolchain.toml" ||
      path === "rust-toolchain",
  );
  return { native, app };
}

if (import.meta.main) {
  const { native, app } = classifyChangedPaths(readFileSync(0, "utf8"));
  process.stdout.write(`native=${native}\napp=${app}\n`);
}
