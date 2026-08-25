#!/usr/bin/env node
import { readFileSync } from "node:fs";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const packageJson = readJson("package.json");
const packageLock = readJson("package-lock.json");
const tauriConfig = readJson("src-tauri/tauri.conf.json");
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargoToml.split(/^\[/m)[1]?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const versions = {
  "package.json": packageJson.version,
  "package-lock.json": packageLock.version,
  "package-lock.json root package": packageLock.packages?.[""]?.version,
  "tauri.conf.json": tauriConfig.version,
  "Cargo.toml": cargoVersion,
};
const expected = packageJson.version;
const mismatches = Object.entries(versions).filter(([, version]) => version !== expected);
const tag = process.argv[2];

if (tag && tag !== `v${expected}`) mismatches.push(["release tag", tag]);
if (mismatches.length) {
  for (const [source, version] of mismatches)
    console.error(`${source}: ${version ?? "missing"}; expected ${expected}`);
  process.exit(1);
}

console.log(`version ${expected} is consistent`);
