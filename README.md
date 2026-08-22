# LoopCode

**Your coding agents, one desktop workspace.**

Run Codex, Claude, Cursor, OpenCode, Grok, Pi, and fx with project-bound threads, visible tool activity, and permission controls.

[Releases](https://github.com/nonlooped/loopcode/releases) · [Build from source](#build-from-source) · [Contribute](CONTRIBUTING.md)

## Why LoopCode?

Agent CLIs work well for one task. They get messy when you juggle projects, providers, terminal tabs, and old conversations. LoopCode keeps the whole job in one window.

- **Pick the right agent.** Use any supported provider per thread.
- **Keep project context.** Threads, drafts, file references, and history survive restarts.
- **See what changed.** Messages, commands, file edits, questions, and approvals share one timeline.
- **Stay in control.** Restricted mode asks before sensitive actions. Full Access removes the prompts when you trust the agent.
- **Work without bouncing around.** Browse files, preview code, attach images, and open the integrated terminal beside the conversation.
- **Keep it local.** LoopCode starts agent processes on your machine and stores workspace history there.

## Supported agents

| Agent    | Install                                                                                                                                             | Login                         |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| Codex    | `npm install --global @openai/codex`                                                                                                                | `codex login`                 |
| Claude   | `npm install --global @anthropic-ai/claude-code`                                                                                                    | `claude auth login`           |
| Cursor   | Linux: `curl https://cursor.com/install -fsS \| bash`<br>Windows: `irm 'https://cursor.com/install?win32=true' \| iex`                              | `agent login`                 |
| OpenCode | `npm install --global opencode-ai`                                                                                                                  | `opencode auth login`         |
| Grok     | Linux: `curl -fsSL https://x.ai/cli/install.sh \| bash -s 1.0.5`<br>Windows: `irm https://x.ai/cli/install.ps1 \| iex; grok update --version 1.0.5` | `grok login`                  |
| Pi       | `npm install --global @earendil-works/pi-coding-agent@0.84.2`                                                                                       | Run `pi`, then enter `/login` |
| fx       | `curl -fsSL https://fx.sh/setup.sh \| bash` on Linux                                                                                                | `fx login` or `fx setup`      |

LoopCode downloads the pinned Codex, Claude, and Pi ACP adapters on first use. fx is visible but unavailable on Windows because its official release supports Linux and macOS only.

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

### Browser preview

Run the real frontend with Tauri's web mocks for browser-driven UI checks:

```sh
npm run web
agent-browser open http://127.0.0.1:1420
agent-browser snapshot -i
```

The preview persists workspace state in local storage and exposes deterministic provider models. Native filesystem, terminal, and agent-process behavior remains available only in the desktop app.

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
