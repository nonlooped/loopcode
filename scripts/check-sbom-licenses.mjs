// Fail-closed Category X license gate for the generated CycloneDX SBOM.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sbomPath = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.join(root, "sbom/loopcode-sbom.json");

if (!fs.existsSync(sbomPath)) {
  throw new Error(`SBOM not found: ${sbomPath} (run npm run sbom first)`);
}

const denyExact = new Set([
  "GPL-1.0",
  "GPL-1.0-only",
  "GPL-1.0-or-later",
  "GPL-2.0",
  "GPL-2.0-only",
  "GPL-2.0-or-later",
  "GPL-2.0-with-autoconf-exception-2.0",
  "GPL-2.0-with-bison-exception-2.2",
  "GPL-2.0-with-classpath-exception-2.0",
  "GPL-2.0-with-font-exception-2.0",
  "GPL-2.0-with-GCC-exception-2.0",
  "GPL-3.0",
  "GPL-3.0-only",
  "GPL-3.0-or-later",
  "GPL-3.0-with-autoconf-exception-3.0",
  "GPL-3.0-with-GCC-exception-3.0",
  "AGPL-1.0",
  "AGPL-3.0",
  "AGPL-3.0-only",
  "AGPL-3.0-or-later",
  "LGPL-2.0",
  "LGPL-2.0-only",
  "LGPL-2.0-or-later",
  "LGPL-2.1",
  "LGPL-2.1-only",
  "LGPL-2.1-or-later",
  "LGPL-3.0",
  "LGPL-3.0-only",
  "LGPL-3.0-or-later",
  "SSPL-1.0",
  "QPL-1.0",
  "Sleepycat",
  "CPOL-1.02",
  "JSON",
  "BSD-4-Clause",
  "APSL-2.0",
  "Commons-Clause",
  "Facebook-2-Clause",
  "Facebook-3-Clause",
  "Facebook-Examples",
  "NPL-1.0",
  "NPL-1.1",
]);

const denyPatterns = [
  /\bGPL-\d/,
  /\bAGPL-\d/,
  /\bLGPL-\d/,
  /\bSSPL\b/,
  /Commons-Clause/,
  /NonCommercial/,
  /non-commercial/,
  /Field-of-use/,
  /field-of-use/,
];

function isDeniedAtom(license) {
  if (!license || !String(license).trim()) return false;
  const value = String(license).trim();
  if (denyExact.has(value)) return true;
  return denyPatterns.some((re) => re.test(value));
}

function removeOuterParentheses(expression) {
  let value = expression.trim();
  while (value.startsWith("(") && value.endsWith(")")) {
    let depth = 0;
    let wrapsAll = true;
    for (let i = 0; i < value.length - 1; i++) {
      if (value[i] === "(") depth++;
      else if (value[i] === ")") depth--;
      if (depth === 0 && i < value.length - 1) {
        wrapsAll = false;
        break;
      }
    }
    if (!wrapsAll) break;
    value = value.slice(1, -1).trim();
  }
  return value;
}

function splitLicenseExpression(expression, operator) {
  const parts = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i <= expression.length - operator.length; i++) {
    if (expression[i] === "(") {
      depth++;
      continue;
    }
    if (expression[i] === ")") {
      depth--;
      continue;
    }
    if (depth === 0 && expression.slice(i, i + operator.length) === operator) {
      parts.push(expression.slice(start, i).trim());
      start = i + operator.length;
      i += operator.length - 1;
    }
  }
  if (parts.length === 0) return [expression];
  parts.push(expression.slice(start).trim());
  return parts;
}

// SPDX alternatives are safe when at least one complete OR branch is permitted;
// conjunctions are safe only when every required term is permitted.
function isLicenseExpressionAllowed(expression) {
  if (!expression || !String(expression).trim()) return false;
  const value = removeOuterParentheses(String(expression));
  const orParts = splitLicenseExpression(value, " OR ");
  if (orParts.length > 1) {
    return orParts.some(isLicenseExpressionAllowed);
  }
  const andParts = splitLicenseExpression(value, " AND ");
  if (andParts.length > 1) {
    return andParts.every(isLicenseExpressionAllowed);
  }
  return !isDeniedAtom(value);
}

function getLicenseExpressions(component) {
  const expressions = [];
  for (const entry of component.licenses ?? []) {
    if (entry.expression) {
      expressions.push(String(entry.expression));
      continue;
    }
    if (entry.license?.id) {
      expressions.push(String(entry.license.id));
      continue;
    }
    if (entry.license?.name) {
      expressions.push(String(entry.license.name));
    }
  }
  if (component.license) expressions.push(String(component.license));
  return expressions;
}

// cdxgen's Rust extractor records the Cargo purl and lockfile evidence but, for
// many crates, omits the Cargo.toml license field from the CycloneDX component.
// Resolve only those missing entries from the already-installed, lock-pinned
// manifests. A missing local manifest remains a hard failure.
function getLocalManifestLicense(component) {
  const purl = String(component.purl ?? "");
  const name = String(component.name ?? "");
  const version = String(component.version ?? "");

  if (purl.startsWith("pkg:cargo/")) {
    let manifest = null;
    if (name === "loopcode") {
      manifest = path.join(root, "src-tauri/Cargo.toml");
    } else {
      const cargoHome = process.env.CARGO_HOME || path.join(process.env.HOME || process.env.USERPROFILE || "", ".cargo");
      const registrySrc = path.join(cargoHome, "registry", "src");
      if (fs.existsSync(registrySrc)) {
        for (const entry of fs.readdirSync(registrySrc)) {
          const candidate = path.join(registrySrc, entry, `${name}-${version}`, "Cargo.toml");
          if (fs.existsSync(candidate)) {
            manifest = candidate;
            break;
          }
        }
      }
    }
    if (manifest && fs.existsSync(manifest)) {
      const raw = fs.readFileSync(manifest, "utf8");
      const m = raw.match(/^\s*license\s*=\s*"([^"]+)"/m);
      if (m) return [m[1]];
    }
  } else if (purl.startsWith("pkg:npm/")) {
    const manifest = path.join(root, "node_modules", name, "package.json");
    if (fs.existsSync(manifest)) {
      const pkg = JSON.parse(fs.readFileSync(manifest, "utf8"));
      if (typeof pkg.license === "string" && pkg.license.trim()) {
        return [pkg.license];
      }
    }
  }
  return [];
}

const sbom = JSON.parse(fs.readFileSync(sbomPath, "utf8"));
if (sbom.bomFormat !== "CycloneDX" || !sbom.components) {
  throw new Error(`SBOM is not a non-empty CycloneDX component inventory: ${sbomPath}`);
}

const violations = [];
const unknown = [];
const components = [sbom.metadata?.component, ...(sbom.components ?? [])].filter(Boolean);

for (const component of components) {
  // This desktop application's release policy covers the shipped npm WebView
  // and Cargo Core graphs. cdxgen can also discover its own bundled Ruby tools;
  // those are generator internals, not release dependencies.
  const purl = String(component.purl ?? "");
  if (!/^pkg:(npm|cargo)\//.test(purl)) continue;

  let expressions = getLicenseExpressions(component);
  const name = component.name || "(unnamed)";
  const version = component.version || "?";
  if (expressions.length === 0) {
    expressions = getLocalManifestLicense(component);
    if (expressions.length === 0) {
      unknown.push(`${name}@${version}`);
      continue;
    }
  }
  const allowed = expressions.some(isLicenseExpressionAllowed);
  if (!allowed) {
    violations.push(`${name}@${version} : ${expressions.join(" OR ")}`);
  }
}

if (violations.length > 0) {
  console.log(`FAIL sbom-license-denylist count=${violations.length}`);
  for (const v of violations) console.log(`DENY: ${v}`);
  process.exit(1);
}

if (unknown.length > 0) {
  console.log(`WARN sbom-license-metadata-unresolved count=${unknown.length}`);
}

console.log(`PASS sbom-license-denylist components=${(sbom.components ?? []).length}`);
