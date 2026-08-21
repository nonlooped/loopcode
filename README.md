# LoopCode

**Your coding agents, one desktop workspace.**

Run Codex, Claude, Cursor, and OpenCode with project-bound threads, visible tool activity, and permission controls.

[Releases](https://github.com/nonlooped/loopcode/releases) · [Build from source](#build-from-source) · [Contribute](CONTRIBUTING.md)

## Why LoopCode?

Agent CLIs work well for one task. They get messy when you juggle projects, providers, terminal tabs, and old conversations. LoopCode keeps the whole job in one window.

- **Pick the right agent.** Use Codex, Claude, Cursor, or OpenCode per thread.
- **Keep project context.** Threads, drafts, file references, and history survive restarts.
- **See what changed.** Messages, commands, file edits, questions, and approvals share one timeline.
- **Stay in control.** Restricted mode asks before sensitive actions. Full Access removes the prompts when you trust the agent.
- **Work without bouncing around.** Browse files, preview code, attach images, and open the integrated terminal beside the conversation.
- **Keep it local.** LoopCode starts agent processes on your machine and stores workspace history there.

## Supported agents

| Agent    | Setup                                                              |
| -------- | ------------------------------------------------------------------ |
| Codex    | The pinned ACP adapter downloads on first launch.                  |
| Claude   | The pinned ACP adapter downloads on first launch.                  |
| Cursor   | Install the `agent` CLI, put it on `PATH`, then run `agent login`. |
| OpenCode | Install `opencode` and put it on `PATH`.                           |

Models, reasoning levels, and fast modes come from each agent, so every thread gets the controls its provider supports.

## Build from source

LoopCode supports Windows 10 or 11 and Linux. Release packages are currently unsigned.

```sh
git clone https://github.com/nonlooped/loopcode.git
cd loopcode
npm install
npm run tauri -- dev
```

You will need Node.js 24+, npm 12+, Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system.

## Built on ACP

LoopCode uses the [Agent Client Protocol](https://agentclientprotocol.com/) to talk to each provider while preserving provider-specific models, tools, and authentication. It currently targets stable ACP v1.

Development checks

```sh
npm run check
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for code conventions, commits, and releases.
