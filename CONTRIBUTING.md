# Contributing to LoopCode

## Getting started

The [README](README.md) covers prerequisites, setup, and the development commands (`npm run dev`, checks, tests).

## Code conventions

- Read [AGENTS.md](AGENTS.md) before changing code. It defines the ownership boundaries between the Svelte 5 frontend and the Rust shell, and where new code belongs.
- UI changes must satisfy [DESIGN_GUIDELINES.md](DESIGN_GUIDELINES.md), including its release checklist.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org): `feat:`, `fix:`, `perf:`, with an optional scope, such as `feat(provider): add retry`. Pull request titles follow the same format because LoopCode squash-merges pull requests and generates `CHANGELOG.md` from those commits.

## Pull requests

All changes to `master`, including version bumps, go through a pull request. Keep one user-visible change in each pull request when practical. Use the pull request template to record why the change matters, its release note, visual proof for UI work, and the checks run.

Apply one of the existing `enhancement`, `bug`, `documentation`, or `dependencies` labels so GitHub can group the change in release notes. Use `skip-release-notes` for internal work and release pull requests.

Squash-merge after `CI / gate` passes and review threads are resolved. GitHub deletes the branch after merging. The repository allows squash merges only and requires pull requests for `master`; no approval count is required while LoopCode has one maintainer.

## Releases

LoopCode follows [Semantic Versioning](https://semver.org/), with a documented meaning for the pre-1.0 period:

- `0.Y.0` is a planned feature release. It may include fixes alongside new user-visible behavior.
- `0.Y.Z` is a fixes-only release for regressions, security problems, or other urgent defects.
- Breaking changes before `1.0.0` ship in `0.Y.0` and must be called out in the release note.
- Internal chores, tests, and documentation do not cause a release. They ship with the next feature or fix release.
- After `1.0.0`, incompatible changes bump the major version, backward-compatible features bump the minor version, and fixes bump the patch version.

SemVer chooses the number. It does not require publishing every time `master` changes. LoopCode normally publishes at most one feature release per week, and only when the accumulated work gives users a clear reason to update. A patch may ship sooner when waiting would leave users with a serious defect. Do not publish another feature release minutes or hours after the previous one.

### Prepare a release

1. Start from an up-to-date `master` with no changes except the release note you are about to write.
2. Rewrite `RELEASE_NOTES.md` for the target version. The first line must be `# LoopCode vX.Y.Z: <user-facing title>`, followed by a short summary and at least one bullet under `## Highlights`. Describe outcomes, not commit names. Add screenshots or recordings when they make a visible change easier to understand.
3. Run `scripts/release.sh X.Y.Z`. The script validates the note, creates `release/vX.Y.Z`, updates every version, regenerates the technical changelog, and commits the release.
4. Push the branch and open a pull request titled exactly `chore(release): vX.Y.Z`. Add the `skip-release-notes` label.
5. Review the generated changelog and version diff, then squash-merge after `CI / gate` passes.

Master CI creates the protected tag only after the merged release commit passes again. The release workflow verifies the tag, version, release note, preceding release, and source commit. It then builds the Windows and Linux installers, puts the curated note first, appends GitHub's categorized pull request list, and publishes the release.
