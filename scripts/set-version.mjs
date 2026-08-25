#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-nightly\.\d{8}\.\d+)?$/.test(version)) {
  console.error("usage: node scripts/set-version.mjs X.Y.Z");
  process.exit(1);
}

for (const file of ["package.json", "package-lock.json"]) {
  const json = JSON.parse(readFileSync(file, "utf8"));
  json.version = version;
  if (file === "package-lock.json") json.packages[""].version = version;
  writeFileSync(file, `${JSON.stringify(json, null, 2)}\n`);
}

for (const file of ["src-tauri/tauri.conf.json"]) {
  const source = readFileSync(file, "utf8");
  const next = source.replace(/("version"\s*:\s*")[^"]*(")/, `$1${version}$2`);
  if (next === source) {
    console.error(`no version field in ${file}`);
    process.exit(1);
  }
  writeFileSync(file, next);
}

const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const nextCargo = cargo.replace(/^version = "[^"]*"/m, `version = "${version}"`);
if (nextCargo === cargo) {
  console.error("no version field in src-tauri/Cargo.toml");
  process.exit(1);
}
writeFileSync("src-tauri/Cargo.toml", nextCargo);

execFileSync(
  "cargo",
  ["metadata", "--manifest-path", "src-tauri/Cargo.toml", "--format-version", "1"],
  {
    stdio: "ignore",
  },
);
