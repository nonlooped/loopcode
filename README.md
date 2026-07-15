# LoopCode

Local AI coding agent desktop app (**v1.0.0 RC**).

LoopCode is a capability-locked Tauri 2 desktop application. Its Rust Core owns
privileged behavior and persistence; the TypeScript WebView renders the UI via
typed IPC. It is licensed Apache-2.0 and uses a complete CycloneDX SBOM with a
Category X license policy during release validation.

Core-owned SQLite persistence covers projects, chats, runs, events,
artifacts, usage, settings, secret_refs; WAL + migrations; project open/rebind
with orphan handling; chat CRUD; event append/replay; and redacted exports.

The agent runtime state machine provides host
lifecycle phases, Ask/Plan/Build/Debug mutation boundaries, mock tool runner,
semantic events, cancel/suspend, and error taxonomy.

The security layer provides a project-root path boundary, fixed approval
matrix (`security-v1`), secret store (refs only in SQLite), audit log, export
redaction, and MCP/skill trust controls.

Built-in tools include `glob`/`read`/`grep`/`edit`/`write`/`patch`,
`exec`, `webfetch`, plan artifacts), context compiler (`AGENTS.md` +
budget manifest), and skill discovery (listed in system prompt; body via `skill`). No Git tools.

Four first-party provider adapters (OpenAI, Anthropic, OpenRouter,
OpenCode Zen), custom protocol profiles, bundled models.dev-style catalog,
fixture probes, and minimal onboarding UI (heroes + custom + catalog).

The agent cockpit UI provides left project/chat navigation and top chrome
(mode/model/cost/run status), center timeline + composer, right Files|Diffs|
Terminal|Diagnostics tabs; timeline projection from Core events; inline
approvals; keyboard map for expert actions.

Integrated Files (tree explorer; open in OS default editor), Diffs (provider-neutral
attribution), Terminal (xterm + Core session/shell discovery), and
content-addressed checkpoints with Files/Conversation/Both restore axes.

The extensibility layer provides skills listed in the system
prompt with on-demand `skill` body load, MCP tools merged into the agent catalog
(server trust + per-tool approval, fingerprint reset), and non-exec prompt
templates with progressive disclosure in Advanced/palette.

Reliability features include bounded provider retries, tool timeout classes,
cost/time soft limits, SQLite integrity + VACUUM INTO backup, local redacted
logs, diagnostics opt-in (off = no upload), and signature-gated update check
(default off).

Accessibility and desktop polish include keyboard focus order, live
regions, default + high-contrast themes, reduced motion, single-instance lock,
app menu, approval notification redaction, and install friction docs.

Release validation includes `npm run release-gate`, the SBOM and Category X
license policy, known limitations/security contact, and the release checklist.
Multi-OS qualitative validation remains environment-limited.

## Architecture (closed decision)

| Layer | Stack |
| --- | --- |
| Desktop shell | [Tauri 2](https://v2.tauri.app/) |
| Core process | Rust (`src-tauri`) — future runtime, tools, SQLite, secrets |
| UI | TypeScript + Vite WebView (`src/`) — capability-gated, no unrestricted OS access |

## Prerequisites

- [Node.js](https://nodejs.org/) 20+ and npm
- [Rust](https://www.rust-lang.org/) stable (edition 2021)
- Platform WebView (WebView2 on Windows, WebKit on macOS/Linux)
- **Windows:** [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload (MSVC linker). Open a “x64 Native Tools” / VsDevCmd shell, or ensure `link.exe` is on `PATH`, before `cargo` / `npm run tauri build`.

## Install notes (unsigned)

LoopCode ships unsigned desktop builds. OS friction is documented in
[docs/install.md](./docs/install.md) (**SmartScreen** More info → Run anyway,
**Gatekeeper** right-click Open, **AppImage** `chmod +x`).

## Build & run

```bash
# Install JS deps
npm install

# Development (opens the empty LoopCode window)
npm run tauri dev

# Production build
npm run tauri build

# Frontend-only typecheck + Vite build (no desktop window)
npm run build
```

## Lint & test

```bash
npm run lint    # TypeScript check + cargo clippy
npm test        # TypeScript, frontend/Rust tests, capabilities, and SBOM license policy
npm run test:unit
npm run release-gate   # RC gate: npm test + lint + build
```

## Release candidate

| Artifact | Command / path |
| --- | --- |
| Version | `1.0.0` in `package.json` + `src-tauri/Cargo.toml` |
| Release gate | `npm run release-gate` |
| SBOM | `npm run sbom` → `sbom/loopcode-sbom.json` |
| Checklist | [docs/release-checklist.md](./docs/release-checklist.md) |
| Limitations | [docs/known-limitations.md](./docs/known-limitations.md) |
| Security | [docs/SECURITY.md](./docs/SECURITY.md) |
| Install | [docs/install.md](./docs/install.md) |

Dependency advisory policy: run `cargo audit` / `npm audit` on release
branches; treat high/critical findings as fail-closed before a public tag.

## License

[Apache License 2.0](./LICENSE). Contributions are under Apache-2.0 (see
[CONTRIBUTING.md](./CONTRIBUTING.md)). Do not add Category X dependencies to
distributed artifacts; the release SBOM gate checks this.

```bash
npm run sbom:check   # CycloneDX SBOM + Category X denylist
```

## Repository layout

```
├── src/                 # TypeScript WebView UI
├── src-tauri/           # Rust core + Tauri config + capabilities
│   ├── src/db/          # SQLite store, migrations, export redaction
│   ├── src/domain.rs    # Domain types
│   └── capabilities/    # Locked-down WebView permissions
├── scripts/             # capability + SBOM license gates, catalog refresh
├── .github/workflows/   # CI lint/test
└── docs/                # security, release, install, and limitation docs
```


### Persistence

- Database: `%APPDATA%\loopcode\…\app.sqlite` (OS data dir via `directories`), WAL mode
- Override path for tests/tools: set `LOOPCODE_DB_PATH`
- Core IPC: `open_project`, `list_chats`, `create_chat`, `append_event`, `list_events`, `export_chat`, …

### Runtime

- Host phases: `queued` → `preparing` → `model_active` ↔ `tool_active` / `approval_waiting` → terminal (`completed` / `failed` / `cancelled`) or `suspended`
- Modes: **Ask** / **Plan** (read-only), **Build** (mutating OK), **Debug** (investigation, no source mutation)
- Built-in tools: `glob`/`read`/`grep`/`edit`/`write`/`patch`/`exec`/`webfetch`/`skill`/…
- IPC: `runtime_start_run`, `runtime_propose_tool`, `runtime_cancel_run`, `runtime_complete_run`
- Crash reopen: in-flight runs → **suspended** (never guessed completed)

### Security

- Path boundary: in-workspace / protected (`.git`, `.env`, keys) / outside — escapes never free-allow
- Fixed matrix: workspace read + ordinary Build write free; shell/network/secrets/MCP/skills/… always prompt
- Grants: deny / allow once / allow matching for this chat (publish/privilege: once only)
- Secrets: Core secret store; `secret_refs` only in SQLite; logs and exports redact sensitive content
- Audit: `audit_log` table; trust rows invalidate on content hash change

### Tools & context

- Real FS tools under project root; existing-file writes, patches, and deletes require matching pre-image hashes
- `edit` for exact string replacement; `write` for whole-file writes
- Bounded `grep` (ignore `node_modules`/`.git`/`target`, capped matches); `glob` for path patterns
- `exec` / `webfetch` still always approval-gated
- Context compiler: host safety + AGENTS.md + attaches under token budget; skills listed in system prompt until `skill` loads a body
- Catalog explicitly excludes first-party Git tools

### Cockpit UI

- Layout: left (projects/chats), top chrome, center (timeline + composer), right tab shells
- Timeline: pure projection of Core events (`project_timeline`); empty/loading/error/suspended/populated
- Composer: Ask/Plan/Build/Debug + model → `cockpit_send` / `runtime_cancel_run`
- Approvals: inline cards → `cockpit_grant_approval` (Deny / Once / This chat)
- Keyboard map: palette, new chat, focus composer, toggle panes, mode cycle, stop (Core + UI registries)

### Files, diffs, terminal, checkpoints

- Explorer: `surfaces_list_tree` (noise filter + show-excluded); click opens via `surfaces_open_external` (OS default app)
- Diffs: turn/run, cumulative, buffer-vs-disk, vs-checkpoint; agent run/tool vs `manual` attribution
- Checkpoints: content-addressed blobs outside project; restore Files / Conversation / Both; intervening-manual warn
- Terminal: shell discovery (default + WSL optional); xterm host; agent `exec` still approval-gated
- Conflicts: `patch` hash fail-closed + dirty-buffer decision path

### Extensibility

- Skills: discovered skills listed in system prompt; `skill` loads body on demand
- MCP: tools from enabled servers merged into the agent catalog; server trust fingerprint + per-invocation approval; config change resets trust
- Prompt templates: expand to composer text only (`ext_template_*`); palette/Advanced progressive disclosure
- Not on onboarding critical path; MCP not auto-started at launch

### Reliability

- Retry: transport/rate-limit auto-retry ≤2 + backoff / Retry-After; auth/policy fail immediately
- Limits: per-run wall-time + cost ceilings (warn/stop); never weaken security
- Suspend: restart reconcile keeps inflight runs suspended (never completed)
- DB: integrity_check on open; `VACUUM INTO` backups outside live file; fail-closed corruption
- Diagnostics: local redacted logs; network upload off by default (zero transport when off)
- Updates: check default off (no HTTP); signature verify rejects bad artifacts
