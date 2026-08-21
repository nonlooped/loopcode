# Contributing to LoopCode

## Getting started

The [README](README.md) covers prerequisites, setup, and the development commands (`npm run dev`, checks, tests).

## Code conventions

- Read [AGENTS.md](AGENTS.md) before changing code. It defines the ownership boundaries between the Svelte 5 frontend and the Rust shell, and where new code belongs.
- UI changes must satisfy [DESIGN_GUIDELINES.md](DESIGN_GUIDELINES.md), including its release checklist.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org): `feat:`, `fix:`, `perf:`, with an optional scope — e.g. `feat(provider): add retry`. `CHANGELOG.md` is generated from these commits by git-cliff, so write subjects as user-facing changelog entries, not internal notes.

## Releases

1. Decide the version per semver against everything since the last tag: new user-facing behavior bumps minor, fixes and performance work bump patch, breaking changes bump major (minor instead while 0.x).
2. On master with a clean tree, run `scripts/release.sh X.Y.Z`. It bumps the version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` (+ lockfile), regenerates `CHANGELOG.md`, commits, and tags.
3. `git push --follow-tags`. CI runs the checks, builds Windows and Linux installers into a draft GitHub Release, and publishes it.
