# Contributing to LoopCode

## Getting started

The [README](README.md) covers prerequisites, setup, and the development commands (`npm run dev`, checks, tests).

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org): `feat:`, `fix:`, `perf:`, with an optional scope, such as `feat(provider): add retry`. Pull request titles follow the same format because LoopCode squash-merges pull requests and generates `CHANGELOG.md` from those commits.

## Pull requests

Name branches `<type>/<short-description>` using a Conventional Commit type such as `feat/`, `fix/`, `chore/`, or `ci/`. Never use an agent or tool name as the branch prefix.

All changes to `master` go through a pull request. Keep one user-visible change in each pull request when practical. Use the pull request template to record why the change matters, visual proof for UI work, and the checks run.

Squash-merge after `CI / gate` passes and review threads are resolved. GitHub deletes the branch after merging. The repository allows squash merges only and requires pull requests for `master`; no approval count is required while LoopCode has one maintainer.

## Releases

LoopCode releases stable SemVer builds. `0.Y.0` is a feature release. `0.Y.Z` is fixes only. Breaking changes before `1.0.0` ship in `0.Y.0` and must be called out in the release note. After `1.0.0`, incompatible changes bump the major version, backward-compatible features bump the minor version, and fixes bump the patch version.

Batch user-visible work into stable releases instead of publishing a minor for every merged feature. Patch releases may ship as soon as a fix is ready.

### Prepare a stable release

1. Start from an up-to-date `master` with a clean working tree.
2. Run `scripts/release.sh X.Y.Z`. The script creates `release/vX.Y.Z`, updates every version, regenerates the changelog, and commits the release.
3. Push the branch and open a pull request titled exactly `chore(release): vX.Y.Z`.
4. Review the generated changelog and version diff, then squash-merge after `CI / gate` passes.

Master CI creates the protected tag only after the merged release commit passes again. The release workflow runs from that tag, verifies the version, preceding stable release, source commit, and top changelog entry, then builds the Windows, Linux, and universal macOS installers. It creates checksums and a draft GitHub Release from the changelog. Review the title, notes, expected assets, and unsigned-package warning before publishing the draft. A failed release run opens or updates the `Release workflow failed` issue and assigns it to the repository owner.

### Recover a release

Rerun failed jobs when the failure was transient. The publish job can repair an existing draft, but it will not overwrite a published stable release.

If the tagged source or workflow is wrong, do not move or delete the tag. Fix `master` and prepare the next patch version. For a bad published release, add a warning to its notes, leave its tag and assets intact, and ship the patch. This keeps old checksums valid and gives affected users a clear upgrade path.
