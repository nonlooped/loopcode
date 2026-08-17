# LoopCode

LoopCode is a lightweight native orchestrator for existing coding-agent harnesses. LoopCode owns its thread identity, titles, workspace, and visible history. ACP session IDs are private per-agent metadata, and agent history must never be imported into the LoopCode sidebar.

Keep ACP integrations thin: each agent remains responsible for its loop, authentication, tools, and configuration. Product-facing UI and copy should remain agent-neutral; model names may be shown where useful.

- Optimize early features for usefulness and low code volume.
- Prefer maintained crates and protocol APIs over custom infrastructure.
- Verify changes with the smallest relevant non-interactive Cargo command, such as `cargo check`, `cargo test`, or formatting checks.

## App execution

- Never launch, run, or restart the LoopCode application. This includes `cargo run`, invoking a built `loopcode` executable, or starting it through PowerShell, a shell, an IDE, a debugger, or any automation tool.
- Never use computer-control, GUI-automation, screenshot, browser, or window-inspection tools to exercise or visually inspect the running application.
- Do not stop, replace, attach to, or interact with a LoopCode process that is already running. Treat every running instance as user-owned.
- For UI changes, limit verification to static code inspection and non-interactive checks such as `cargo fmt -- --check`, `cargo check`, and relevant tests. Describe any remaining manual visual verification for the user instead of performing it.
