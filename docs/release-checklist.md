# v1.0.0 RC release checklist

Owner sign-off checklist for LoopCode v1 release candidate. Automated gates that
pass on the implementer OS are checked. Live multi-OS / qualitative items that
cannot run here are left **open** with env notes—not faked.

**Product / package version:** `1.0.0` (npm + Cargo)  
**Policy version:** `security-v1`  
**Date / owner:** _______________

## A. Automated one-OS gates (this environment)

- [ ] `npm run release-gate` exits 0
- [ ] Core unit/integration suites green (runtime, security, persistence/suspend, providers, cockpit, surfaces, extensibility, reliability, a11y)
- [ ] Capability allowlist scan clean (`scripts/check-capabilities.mjs`)
- [ ] SBOM Category X gate clean (`npm run sbom:check`)
- [ ] Frontend production build (`npm run build`) exits 0
- [ ] Complete CycloneDX SBOM written under `sbom/` and reviewed for missing licenses
- [ ] LICENSE present
- [ ] Security contact published (`docs/SECURITY.md`)
- [ ] Known limitations published (`docs/known-limitations.md`)
- [ ] Install friction docs published (`docs/install.md`)

## B. Product journey matrix (automated honesty)

| Journey / invariant | Automated evidence | Owner sign-off |
| --- | --- | --- |
| 1. Open project + connect provider (onboarding) | providers_slice5 + UI structure | [ ] |
| 2. Chat run Ask/Plan (no source mutation) | runtime_slice2 mode matrix + tools_slice4 | [ ] |
| 3. Build with approval for shell | security_slice3 + cockpit approvals + tools shell grant | [ ] |
| 4. Files/diff/checkpoint intervene | surfaces_slice7 | [ ] |
| 5. Skills/MCP progressive disclosure | extensibility_slice8 | [ ] |
| Crash / reopen → **suspended** not completed | runtime reconcile + reliability_slice9 | [ ] |
| Diagnostics upload default **off** | reliability diagnostics zero-transport test | [ ] |
| Update check default off; bad signature rejected | reliability updates tests | [ ] |
| Approval notification body has no secrets | a11y_slice10 notifications | [ ] |

## C. Multi-OS qualitative / SR (env-limited)

Leave unchecked when not executed. Do **not** invent session notes.

- [ ] Windows install + SmartScreen path dry-run
- [ ] macOS install + Gatekeeper path dry-run
- [ ] Linux AppImage / package dry-run
- [ ] Narrator smoke (onboarding, approval, restore)
- [ ] VoiceOver smoke
- [ ] Orca smoke
- [ ] Keyboard-only journeys 1–5 on each OS
- [ ] Persona sessions (≥3 per primary persona) with **zero** unresolved critical/high findings

**Env note (this RC agent):** single Windows host; items in section C remain open. Capture: `slice11-multi-os-qual-env-fail.log`.

## D. Release packaging (human)

- [ ] Optional: `npm run tauri build` when MSVC/WebView available
- [ ] Optional: git tag `v1.0.0` (not required if publish forbidden)
- [ ] Optional: cargo audit / npm audit on release branch; fail high/critical per policy

## Sign-off

| Role | Name | Date | Result |
| --- | --- | --- | --- |
| Implementer automated gates | ________ | ________ | Pass / Fail |
| Release owner | ________ | ________ | Ship RC / Hold |
