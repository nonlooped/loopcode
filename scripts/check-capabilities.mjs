// Enforces the deliberately narrow Tauri capability contract.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const capPath = path.join(root, "src-tauri/capabilities/default.json");

const failures = [];
const reports = [];

if (!fs.existsSync(capPath)) {
  console.error(`Missing capability file: ${capPath}`);
  process.exit(1);
}

const json = JSON.parse(fs.readFileSync(capPath, "utf8"));
reports.push(`capability_file=${capPath}`);
reports.push(`identifier=${json.identifier}`);

const allowedPermissions = [
  "core:default",
  "dialog:allow-open",
  "core:window:allow-close",
  "core:window:allow-minimize",
  "core:window:allow-maximize",
  "core:window:allow-unmaximize",
  "core:window:allow-toggle-maximize",
  "core:window:allow-start-dragging",
];

if (!json.permissions) {
  failures.push("capabilities/default.json has no permissions array");
} else {
  const perms = json.permissions.map(String);
  reports.push(`permissions=${perms.join(",")}`);

  for (const p of perms) {
    if (!allowedPermissions.includes(p)) {
      failures.push(`Permission is not in the explicit allowlist: ${p}`);
    }
  }
  for (const p of allowedPermissions) {
    if (!perms.includes(p)) {
      failures.push(`Required narrow permission is missing: ${p}`);
    }
  }

  const cargo = fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8");
  if (/tauri-plugin-opener/.test(cargo)) {
    failures.push("src-tauri/Cargo.toml still depends on tauri-plugin-opener");
  } else {
    reports.push("opener_plugin=absent");
  }

  const pkg = fs.readFileSync(path.join(root, "package.json"), "utf8");
  if (/@tauri-apps\/plugin-opener/.test(pkg)) {
    failures.push("package.json still depends on @tauri-apps/plugin-opener");
  } else {
    reports.push("opener_npm=absent");
  }

  const lib = fs.readFileSync(path.join(root, "src-tauri/src/lib.rs"), "utf8");
  if (/tauri_plugin_opener/.test(lib)) {
    failures.push("lib.rs still initializes tauri_plugin_opener");
  } else {
    reports.push("opener_init=absent");
  }
}

const conf = JSON.parse(
  fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8"),
);
const windows = conf.app?.windows ?? [];
if (windows.length < 1) {
  failures.push("tauri.conf.json has no windows");
} else {
  const { label, title } = windows[0];
  reports.push(`window_label=${label}`);
  reports.push(`window_title=${title}`);
  if (title !== "LoopCode") {
    failures.push(`Expected window title 'LoopCode', got '${title}'`);
  }
  const capWindows = json.windows ?? [];
  if (capWindows.includes("*")) {
    failures.push("Capability windows must name the main window; wildcard is forbidden");
  }
  if (label && !capWindows.includes(label)) {
    failures.push(`Capability windows list does not include window label '${label}'`);
  }
}

for (const line of reports) console.log(line);

if (failures.length > 0) {
  console.log(`FAIL count=${failures.length}`);
  for (const f of failures) console.log(`FAIL: ${f}`);
  process.exit(1);
}

console.log("PASS capability-locked empty shell");
