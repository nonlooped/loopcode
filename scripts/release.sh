#!/usr/bin/env bash
# Create a release branch, bump the version everywhere, regenerate CHANGELOG.md,
# and commit the prepared release. CI tags the squash-merged release commit.
# Usage: scripts/release.sh X.Y.Z
set -euo pipefail

ver="${1:?usage: scripts/release.sh X.Y.Z}"
[[ "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not semver: $ver" >&2; exit 1; }
root="$(git rev-parse --show-toplevel)"
[[ "$(pwd -P)" == "$root" ]] || { echo "run from repository root: $root" >&2; exit 1; }
[[ "$(git branch --show-current)" == "master" ]] || { echo "release from master" >&2; exit 1; }
[[ -z "$(git status --porcelain=v1 --untracked-files=all)" ]] || { echo "working tree must be clean before a release" >&2; exit 1; }
git fetch --quiet origin master --tags
[[ "$(git rev-parse HEAD)" == "$(git rev-parse refs/remotes/origin/master)" ]] || { echo "local master is not at origin/master" >&2; exit 1; }

release_branch="release/v$ver"
! git show-ref --verify --quiet "refs/heads/$release_branch" || { echo "branch already exists: $release_branch" >&2; exit 1; }
! git ls-remote --exit-code --heads origin "$release_branch" >/dev/null 2>&1 || { echo "remote branch already exists: $release_branch" >&2; exit 1; }
git switch -c "$release_branch"

node scripts/set-version.mjs "$ver"
node scripts/check-versions.mjs "v$ver"
npx --yes git-cliff@2.13.1 --tag "v$ver" --tag-pattern '^v[0-9]+\.[0-9]+\.[0-9]+$' -o CHANGELOG.md
npx vp check --fix CHANGELOG.md

git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md
git commit -m "chore(release): v$ver"
echo "push the branch: git push -u origin $release_branch"
echo "open a PR titled chore(release): v$ver"
echo "squash-merge it after CI passes; master CI will tag v$ver and draft the release"
