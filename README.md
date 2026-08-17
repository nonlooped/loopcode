# LoopCode

LoopCode is a small desktop workbench for coding agents that speak the Agent Client Protocol (ACP). It launches each agent harness as a local subprocess, creates an ephemeral ACP session for a working folder, streams messages and tool activity, persists the visible LoopCode thread, and presents permission requests without importing an agent's private history.

The application uses Tauri 2, Svelte 5, TypeScript, Vite+, and the official `@agentclientprotocol/sdk`. Its Windows shell combines Tauri's native acrylic effect with a transparent webview and a solid CSS fallback for reduced-transparency environments.

## Requirements

- Windows 10 or 11 with WebView2
- Node.js 24 or newer and npm 12 or newer
- A current Rust toolchain
- The standard Tauri Windows build prerequisites
- At least one supported ACP harness

Codex and Claude launch pinned npm adapters through `npx`. Their first launch may require network access. OpenCode expects the `opencode` command to be installed. Sign in with the relevant agent CLI before using that provider.

The bundled adapter versions are:

- Codex ACP: `@agentclientprotocol/codex-acp@1.4.0`
- Claude ACP: `@agentclientprotocol/claude-agent-acp@0.69.0`

These pins should be reviewed and updated deliberately with each LoopCode release rather than following `latest` at runtime.

## Install and run

```powershell
npm install
npm run tauri -- dev
```

Tauri starts the frontend through `npm run dev`, which runs `vp dev`.

At startup, LoopCode briefly connects to Codex, Claude, and OpenCode to discover their advertised models. It then stops those discovery harnesses and caches the model lists for the lifetime of the application. New and restored threads remain local and disconnected until their first prompt.

A thread can be associated with the initial working folder or a folder added through the project menu. The first prompt starts only the selected provider, applies the selected model, reasoning option, and advertised Fast mode setting, and sends text and any attached images. Switching to a provider that has not yet been used remains local until the next prompt.

After the first text prompt, LoopCode uses the selected model in a quiet secondary ACP session to generate a concise title. Title-session output never enters the visible transcript.

## Persistence

Visible workspace state is stored at `~/.loopcode/threads.json`. The previous successful snapshot is retained at `~/.loopcode/threads.json.bak`, and writes use a temporary file before rotation.

LoopCode persists:

- Threads, drafts, archive state, messages, image attachments, and tool activity
- Added projects and the selected project
- The selected provider for each thread

ACP session IDs are persisted as private per-provider thread metadata so agents can restore context after a restart. Harness process IDs, connection status, provider errors, and discovered runtime configuration remain transient. Agent-replayed history is not imported into LoopCode's visible transcript.

## Verification

```powershell
npm run check
npm test
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo test --manifest-path src-tauri\Cargo.toml
cargo check --manifest-path src-tauri\Cargo.toml
```

`npm run check` runs Svelte diagnostics plus Vite+ formatting, linting, and TypeScript checks. Frontend tests exercise extracted state transformations rather than matching implementation text.

## Current boundaries

- LoopCode targets stable ACP v1. Experimental ACP v2 is not enabled.
- Authentication flows, loading agent-owned sessions, MCP server injection, and non-model configuration UI beyond advertised Fast mode are not implemented.
- Permission requests are supported. LoopCode does not advertise filesystem or terminal client capabilities.
- Citations and specialized rich tool renderers are not included.
- Each active thread/provider pair owns its harness process; there is no cross-thread pooling, crash recovery, queue backpressure, or log export.
- Windows acrylic and responsive behavior still require manual visual verification on target machines.

## Structure

- `src/App.svelte` — top-level application coordination and page composition
- `src/components/` — title bar, sidebar, transcript, composer, pickers, settings, permissions, and Markdown rendering
- `src/config/` — pinned provider definitions and provider presentation metadata
- `src/services/acp.ts` — official ACP SDK integration over the Tauri broker transport
- `src/services/provider-runtime.ts` — provider discovery, connection lifecycle, model selection, and transient runtime state
- `src/services/native.ts` — typed Tauri commands and channels
- `src/services/workspace-persistence.ts` — debounced frontend persistence coordination
- `src/types/` — application and timeline types
- `src/utils/` — workspace validation, thread creation, timeline transforms, title helpers, attachments, and model normalization
- `src/styles/` — feature-scoped global styles
- `src-tauri/src/broker.rs` — subprocess lifecycle and newline-delimited JSON relay
- `src-tauri/src/persistence.rs` — recoverable JSON snapshot storage
- `tests/` — behavioral frontend tests
