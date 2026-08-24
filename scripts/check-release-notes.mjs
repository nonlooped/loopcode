#!/usr/bin/env node
import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync("package.json", "utf8")).version;
const tag = process.argv[2] ?? `v${packageVersion}`;
const path = process.argv[3] ?? "RELEASE_NOTES.md";
const fail = (message) => {
  console.error(`${path}: ${message}`);
  process.exit(1);
};

if (!/^v\d+\.\d+\.\d+$/.test(tag)) fail(`invalid release tag ${tag}`);

const notes = readFileSync(path, "utf8");
const lines = notes.split(/\r?\n/);
const titlePrefix = `# LoopCode ${tag}: `;

if (!lines[0]?.startsWith(titlePrefix) || lines[0] === titlePrefix)
  fail(`first line must be "${titlePrefix}<user-facing title>"`);

const firstSection = lines.findIndex((line) => line.startsWith("## "));
if (firstSection < 0 || !lines.slice(1, firstSection).join("\n").trim())
  fail("add a short summary before the first section");

const highlights = notes.match(/(?:^|\n)## Highlights\r?\n([\s\S]*?)(?=\r?\n## |\s*$)/)?.[1];
if (!highlights || !/^\s*-\s+\S/m.test(highlights))
  fail("add at least one bullet under ## Highlights");

console.log(`release notes for ${tag} are ready`);
