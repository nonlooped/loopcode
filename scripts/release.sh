#!/usr/bin/env bash
# Bump the version everywhere, regenerate CHANGELOG.md, and commit.
# CI tags the commit and starts the release after the commit passes.
# Usage: scripts/release.sh X.Y.Z
set -euo pipefail

ver="${1:?usage: scripts/release.sh X.Y.Z}"
[[ "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not semver: $ver" >&2; exit 1; }
root="$(git rev-parse --show-toplevel)"
[[ "$(pwd -P)" == "$root" ]] || { echo "run from repository root: $root" >&2; exit 1; }
[[ "$(git branch --show-current)" == "master" ]] || { echo "release from master" >&2; exit 1; }
[[ -z "$(git status --porcelain=v1)" ]] || { echo "working tree not clean" >&2; exit 1; }
git fetch --quiet origin master
[[ "$(git rev-parse HEAD)" == "$(git rev-parse refs/remotes/origin/master)" ]] || { echo "local master is not at origin/master" >&2; exit 1; }

npm version "$ver" --no-git-tag-version

node -e '
const fs = require("fs");
const ver = process.argv[1];
for (const f of ["src-tauri/tauri.conf.json"]) {
  const s = fs.readFileSync(f, "utf8");
  // Replace only the version field; a full JSON round-trip would reformat the file.
  const next = s.replace(/("version"\s*:\s*")[^"]*(")/, `$1${ver}$2`);
  if (next === s) { console.error(`no version field in ${f}`); process.exit(1); }
  fs.writeFileSync(f, next);
}
let t = fs.readFileSync("src-tauri/Cargo.toml", "utf8");
t = t.replace(/^version = "[^"]*"/m, `version = "${ver}"`);
fs.writeFileSync("src-tauri/Cargo.toml", t);
' "$ver"

cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1 > /dev/null # sync Cargo.lock without building
node scripts/check-versions.mjs "v$ver"
npx --yes git-cliff@2.13.1 --tag "v$ver" -o CHANGELOG.md
npx vp check --fix CHANGELOG.md

git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md
git commit -m "chore(release): v$ver"
echo "push the commit: git push origin master"
echo "CI will tag v$ver and start the release after the gate passes"
