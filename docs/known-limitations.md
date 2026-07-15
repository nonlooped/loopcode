# Known limitations (v1 RC)

Honest product and environment limits for LoopCode v1. These are **not**
temporary bugs unless marked otherwise; they define the release boundary.

## Unsigned desktop trust

- Releases are **unsigned** (no paid Apple/Microsoft code signing).
- Windows SmartScreen, macOS Gatekeeper, and Linux AppImage exec bits require
  user-mediated open steps — see [install.md](./install.md).
- Update-channel authenticity (if enabled) uses app-signed artifacts; that is
  **not** OS code signing.

## Security & sandbox

- WebView remains capability-locked; unrestricted OS access stays in Core.
- Native OS sandbox strength can vary; degraded sandbox is surfaced honestly
  when detected (does not disable the fixed approval matrix).
- First-party **Git tools / worktree model** are **out of scope** for v1
  (Git only via approved `exec` or the integrated terminal).

## Runtime honesty

- Crash / restart: in-flight runs become **suspended**, never guessed
  **completed**.
- Ask/Plan modes refuse source-mutating tools; Build may mutate under policy;
  Debug is diagnostics-oriented without free source mutation.
- Provider auto-retry is bounded (≤2) for transport/rate-limit only; auth/quota
  fail immediately.
- Interactive terminal sessions are a user escape hatch; they do **not**
  auto-approve agent `exec`.
- Agent `exec` on Windows prefers `pwsh`, then Windows PowerShell, then
  `cmd`. Timeouts kill the process tree (Job Object). Prefer
  `edit` / `patch` / `write` for file edits.
- Follow-up turns include prior chat history (capped). Tool rounds are soft-capped
  with a final-answer nudge rather than a hard mid-task abort when possible.

## Extensibility

- MCP servers are not auto-started at app launch; server trust ≠ tool allow-all.
- Skill script bodies/scripts never auto-run on folder open; hash change
  re-prompts.
- No marketplace, plugin SDK, or lifecycle hooks outside the approval matrix.

## Accessibility & multi-OS qualitative

- WCAG certification is not claimed; AA-oriented keyboard/SR structure is
  shipped and tested with automated/static gates.
- Full three-OS Narrator / VoiceOver / Orca lab sessions and moderated persona
  studies may be **env-limited** for a given RC build machine; when not run,
  they remain open checklist items (see [release-checklist.md](./release-checklist.md)).

## Diagnostics & data

- Network diagnostic upload is **off by default** (zero upload traffic when
  off).
- Checkpoints are content-addressed outside the project tree and outside Git;
  they never write Git refs.
- SQLite backups use safe APIs (`VACUUM INTO`); do not copy a live WAL DB with
  a naive multi-file copy.

## Packaging

- SBOM is regenerable via `npm run sbom` (CycloneDX JSON under `sbom/`).
- Dependency advisory scanning (cargo audit / npm audit class) is **documented
  policy** for release branches; CI may not fail on every advisory in all
  environments—owners should run audits before a public tag.
