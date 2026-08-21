<p align="center">
  <img src="assets/loopcode-icon.svg" width="112" alt="LoopCode icon">
</p>

<h1 align="center">LoopCode</h1>

<p align="center">
  One desktop app for Codex, Claude, Cursor, and OpenCode.
</p>

LoopCode gives coding agents a proper workspace. Every thread stays attached to its project, tool calls appear alongside the conversation, and sensitive actions wait for your approval.

No pile of terminal tabs. No guessing which agent changed what.

## One place for agent work

- Run Codex, Claude, Cursor, or OpenCode in any project.
- Choose the model, reasoning level, and supported fast modes per thread.
- Follow messages, commands, file changes, and permission requests in one timeline.
- Browse the project and open files without leaving the conversation.
- Mention files and folders with `@`, attach images, and keep drafts between restarts.
- Use Restricted mode to approve actions yourself, or Full Access when you trust the agent.

LoopCode runs providers locally through the [Agent Client Protocol](https://agentclientprotocol.com/). Your projects and thread history stay on your machine.

## Get started

LoopCode currently builds from source. Version tags publish unsigned Windows and Linux packages to [GitHub Releases](https://github.com/nonlooped/loopcode/releases).

You will need:

- Windows 10 or 11, or Linux with WebKit2GTK 4.1
- Node.js 24+, npm 12+, and Rust
- The [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system
- At least one supported agent configured and authenticated

```sh
git clone https://github.com/nonlooped/loopcode.git
cd loopcode
npm install
npm run tauri -- dev
```

Codex and Claude use pinned npm adapters that download on first launch. OpenCode expects `opencode` on `PATH`. Cursor expects `agent` on `PATH`; run `agent login` before opening it in LoopCode.

## How it works

1. Add a project folder.
2. Start a thread and choose an agent and model.
3. Describe what you want built or fixed.
4. Watch the work, answer questions, and approve sensitive actions.
5. Return later without losing the thread, draft, or project context.

Each active thread starts its own agent process. Closing LoopCode saves the visible workspace and stops those processes cleanly.

## Why ACP?

ACP gives editors and coding agents a shared language. LoopCode can support different agents without pretending they all behave the same. Each provider still controls its own models, reasoning options, tools, and authentication.

LoopCode currently targets stable ACP v1.

## Development

Frontend checks:

```sh
npm run check
npm test
npm run build
```

Rust checks:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## Releases

Run `scripts/release.sh X.Y.Z`. It bumps the version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` (+ lockfile), regenerates `CHANGELOG.md` from conventional commits via git-cliff, then commits and tags `vX.Y.Z`. Push with `git push --follow-tags`.

CI runs the checks above, builds Windows and Linux packages, then publishes them together in a GitHub Release.
