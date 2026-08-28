<p align="center">
  <img src="assets/LoopCode.png" width="96" height="96" alt="LoopCode">
</p>

# LoopCode

**Your coding agents, one desktop workspace.**

[![CI](https://github.com/nonlooped/loopcode/actions/workflows/ci.yml/badge.svg)](https://github.com/nonlooped/loopcode/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/nonlooped/loopcode)](https://github.com/nonlooped/loopcode/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

Run Codex and Claude with project-bound threads, visible tool activity, and permission controls. LoopCode is a local desktop workspace for ACP coding agents, not a CLI task orchestrator.

![LoopCode workspace with an agent thread](assets/screenshot.png)

[Latest release](https://github.com/nonlooped/loopcode/releases/latest) · [Build from source](#build-from-source) · [Contribute](CONTRIBUTING.md)

## Why LoopCode?

Agent CLIs work well for one task. They get messy when you juggle projects, providers, terminal tabs, and old conversations. LoopCode keeps the whole job in one window.

- **Keep project context.** Threads, drafts, file references, and history survive restarts.
- **See what changed.** Messages, commands, file edits, questions, and approvals share one timeline.
- **Stay in control.** Restricted mode asks before sensitive actions. Full Access removes the prompts when you trust the agent.
- **Work without bouncing around.** Browse files, preview code, attach images, and open the integrated terminal beside the conversation.
- **Keep it local.** LoopCode starts agent processes on your machine and stores workspace history there.

## Supported agents

| Agent  | Install                                          | Login               |
| ------ | ------------------------------------------------ | ------------------- |
| Codex  | `npm install --global @openai/codex`             | `codex login`       |
| Claude | `npm install --global @anthropic-ai/claude-code` | `claude auth login` |

LoopCode downloads the pinned Codex and Claude ACP adapters on first use.

Models, reasoning levels, and fast modes come from the agent, so every thread gets the controls it supports.

More providers are on the way. LoopCode previously shipped Cursor, OpenCode, Grok, Pi, and fx; they were removed so each one can return with full support for its features rather than a lowest-common-denominator subset.

## Install a release

Download the latest **stable** packages and `SHA256SUMS.txt` from the [latest release](https://github.com/nonlooped/loopcode/releases/latest). Nightly builds are GitHub pre-releases of `master`. Use Stable for daily use.

Windows and Linux packages are unsigned. The macOS app has an ad-hoc signature but is not Developer ID signed or notarized. Verify the checksum before bypassing an operating-system warning.

The macOS DMG supports Apple Silicon and Intel Macs. If macOS reports that LoopCode is damaged, move it to Applications and remove the quarantine attribute:

```sh
xattr -dr com.apple.quarantine /Applications/LoopCode.app
```

Windows SmartScreen may require **More info > Run anyway**. Only bypass Gatekeeper or SmartScreen for a package downloaded from this repository whose checksum matches `SHA256SUMS.txt`.

## Build from source

LoopCode supports Windows 10 or 11, Linux, and macOS.

```sh
git clone https://github.com/nonlooped/loopcode.git
cd loopcode
npm install
npx tauri dev
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

See [CONTRIBUTING.md](CONTRIBUTING.md) for development checks, code conventions, commits, and releases.

## License

LoopCode is [MIT](LICENSE) licensed.

It is not affiliated with OpenAI, Anthropic, or the other agents it can run. Those names are their trademarks.
