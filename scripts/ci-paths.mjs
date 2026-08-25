#!/usr/bin/env node
import { readFileSync } from "node:fs";

export function classifyChangedPaths(changed) {
  const paths = changed.split(/\r?\n/).filter(Boolean);
  const native = paths.some(
    (path) =>
      path.startsWith("src-tauri/") || path === "rust-toolchain.toml" || path === "rust-toolchain",
  );
  return { native };
}

if (import.meta.main) {
  const { native } = classifyChangedPaths(readFileSync(0, "utf8"));
  process.stdout.write(`native=${native}\n`);
}
