# LoopCode

LoopCode is a Windows desktop workbench for coding agents that use the Agent Client Protocol (ACP). It launches harnesses locally, keeps each thread tied to a working folder, streams tool activity, and presents permission requests. LoopCode persists its own visible transcript; an agent's private history never becomes sidebar history.

## Run locally

Requirements:

- Windows 10 or 11 with WebView2, or Linux with WebKit2GTK 4.1 and a compositor (GNOME/KDE recommended)
- Node.js 24+ and npm 12+
- Current Rust and the Tauri prerequisites for your OS ([Windows](https://v2.tauri.app/start/prerequisites/#windows) / [Linux](https://v2.tauri.app/start/prerequisites/#linux))
- A configured ACP harness

```sh
npm install
npm run tauri -- dev
```

Codex and Claude use the npm adapters pinned in `src/config/provider-definitions.ts`; their first launch may need network access. OpenCode expects `opencode` on `PATH`, and Cursor expects `agent` on `PATH`. Authenticate with the provider CLI before using it (`agent login` for Cursor).

## Runtime model

At startup, LoopCode briefly launches each provider to discover advertised models, then caches the results for that app run. A new or restored thread remains disconnected until its first prompt.

The first prompt starts the selected provider for the selected folder and applies its model, reasoning, and advertised Fast mode settings. A separate quiet ACP session generates the thread title after the first text prompt; its output is never added to the transcript.

Visible workspace state is stored in `~/.loopcode/threads.json`. Writes rotate the previous snapshot to `threads.json.bak`. Persisted state includes threads, drafts, messages, attachments, tool activity, projects, selections, and private per-provider ACP session IDs. Process IDs, connection state, provider errors, and discovered runtime configuration remain transient.

## Project map

- `src/App.svelte` — application coordination
- `src/components/` — desktop UI
- `src/config/` — provider commands and presentation metadata
- `src/services/acp.ts` — ACP SDK transport integration
- `src/services/provider-runtime.ts` — discovery and provider lifecycle
- `src/services/native.ts` — typed Tauri commands and channels
- `src/services/workspace-persistence.ts` — debounced persistence
- `src/types/` — workspace and timeline types
- `src/utils/` — state transformations and focused helpers
- `src/styles/` — feature-scoped global CSS
- `src-tauri/src/broker.rs` — subprocess and JSON-line relay
- `src-tauri/src/persistence.rs` — recoverable snapshot storage
- `tests/` — frontend behavior tests

Agent instructions live in [`AGENTS.md`](AGENTS.md). Visual rules live in [`DESIGN.md`](DESIGN.md).

## Validation

The available frontend commands are defined in `package.json`; Rust commands use `src-tauri/Cargo.toml`. Run only the checks covering the changed area.

```sh
npm run check
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## Current boundaries

- Stable ACP v1 only; experimental ACP v2 is disabled.
- No authentication flow, agent-owned session browser, MCP injection, citation UI, specialized tool renderers, or non-model configuration beyond advertised Fast mode.
- Permission requests are supported; filesystem and terminal client capabilities are not advertised.
- Each active thread/provider pair owns one harness process. There is no pooling, crash recovery, queue backpressure, or log export.
- Acrylic (Windows) and responsive behavior require manual verification on target hardware.
