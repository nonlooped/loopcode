# LoopCode

[![Release](https://img.shields.io/badge/release-1.0.1-0a84ff?style=flat-square&logo=semver)](https://github.com/nonlooped/loopcode/releases)
[![License](https://img.shields.io/badge/License-Apache%202.0-0f766e?style=flat-square&logo=apache&logoColor=white)](./LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/nonlooped/loopcode/.github/workflows/ci.yml?branch=main&style=flat-square&label=CI&logo=githubactions)](https://github.com/nonlooped/loopcode/actions/workflows/ci.yml)
[![Node.js](https://img.shields.io/badge/Node.js-20+-0f172a?style=flat-square&logo=nodedotjs&logoColor=white)](https://nodejs.org/)

**LoopCode** is a local AI coding agent built to make powerful code automation feel safe.

You get the speed of an agent workflow without giving up control.

## Why LoopCode exists

AI coding agents are often either:

- Too risky (they can change files without clear approval), or
- Too hard to understand (they hide what is happening behind a black box).

LoopCode is designed for the opposite. It keeps the user in control through explicit approvals, clear timelines, and a permission model that’s enforced by a native Rust core.

## What makes it different

- **Approachability-first cockpit**
  - Friendly layout: conversations, project browser, files, diffs, terminal, and diagnostics.
  - Read-only planning mode, safer build mode, and guided debug mode.

- **Safe by default**
  - Clear security gates before risky actions like shell/network/secret tools.
  - Full audit trail and redacted exports for privacy.
  - Existing-file edits and destructive actions include pre-image checks.

- **Built for local trust**
  - SQLite-backed local history and checkpoints.
  - No hidden cloud sync of your code.
  - Run status, errors, and tool usage are preserved consistently.

- **Flexible for power users**
  - Multiple model providers and custom protocol profiles.
  - MCP + skills integration with explicit trust controls.
  - Command palette and keyboard-first workflows.

- **Production-minded reliability**
  - Retry and timeout handling, integrity checks, backup flow, and restart recovery.
  - Diagnostics are local-first and optional for networking.

## What you can do with it

- Open a project and run conversational coding sessions.
- Let the agent inspect files, propose changes, and run safe commands.
- Review each step in a timeline before it lands in your working tree.
- Compare and restore from checkpoints quickly.
- Pause, stop, and continue with confidence.

## Quick start

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) stable (1.97+ for this project)
- WebView runtime on your OS
  - Windows: WebView2
  - macOS: WebKit
  - Linux: WebKitGTK

### Run locally

```bash
npm install
npm run tauri dev
```

### Common scripts

```bash
npm run tauri build   # package a desktop app
npm run build         # build frontend only (Vite)
npm run test:web      # frontend tests
npm run lint         # type checks + clippy
```

## Architecture snapshot

LoopCode uses a **Tauri 2 desktop shell** with a clear split:

- **Core (`src-tauri/`)**: persistence, security, runtime, tooling, and file operations.
- **WebView (`src/`)**: interface and interaction layer.
- **IPC (`src/ipc/`)**: typed command boundary between UI and Core.

This split keeps privileged actions in Core and enforces capability boundaries in one place.

## Release and docs

- [Install guidance](./docs/install.md)
- [Security model](./docs/SECURITY.md)
- [Capability checks](./scripts/check-capabilities.mjs)
- [Known limitations](./docs/known-limitations.md)
- [Release checklist](./docs/release-checklist.md)

## Repository at a glance

```text
├── src/                 # React + TypeScript webview
├── src-tauri/           # Rust core + Tauri configuration
├── src/ipc/             # typed IPC contract
├── docs/                # design, security, and release docs
├── scripts/             # capability checks, SBOM checks, catalog tools
└── .github/workflows/   # CI and release gates
```

## License

Apache-2.0 — see [LICENSE](./LICENSE).