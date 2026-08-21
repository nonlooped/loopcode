#!/usr/bin/env bash
# Bump the version everywhere, regenerate CHANGELOG.md, commit and tag.
# CI builds installers and publishes the GitHub Release when the tag lands.
# Usage: scripts/release.sh X.Y.Z   then: git push --follow-tags
set -euo pipefail

ver="${1:?usage: scripts/release.sh X.Y.Z}"
[[ "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not semver: $ver" >&2; exit 1; }
git diff --quiet --exit-code || { echo "working tree not clean" >&2; exit 1; }

node -e '
const fs = require("fs");
const ver = process.argv[1];
for (const f of ["package.json", "src-tauri/tauri.conf.json"]) {
  const j = JSON.parse(fs.readFileSync(f, "utf8"));
  j.version = ver;
  fs.writeFileSync(f, JSON.stringify(j, null, 2) + "\n");
}
let t = fs.readFileSync("src-tauri/Cargo.toml", "utf8");
t = t.replace(/^version = "[^"]*"/m, `version = "${ver}"`);
fs.writeFileSync("src-tauri/Cargo.toml", t);
' "$ver"

cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1 > /dev/null # sync Cargo.lock without building
npx --yes git-cliff --tag "v$ver" -o CHANGELOG.md

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md
git commit -m "chore(release): v$ver"
git tag -a "v$ver" -m "v$ver"
echo "tagged v$ver — push with: git push --follow-tags"
