# LoopCode

LoopCode is a lightweight native orchestrator for existing coding-agent harnesses. LoopCode owns its thread identity, titles, workspace, and visible history. ACP session IDs are private per-agent metadata, and agent history must never be imported into the LoopCode sidebar.

Keep ACP integrations thin: each agent remains responsible for its loop, authentication, tools, and configuration. Product-facing UI and copy should remain agent-neutral; model names may be shown where useful.

- Optimize early features for usefulness and low code volume.
- Prefer maintained crates and protocol APIs over custom infrastructure.
- Run tests, checks, formatters, and other validation only for the files, packages, or components touched. Do not run app-wide or repo-wide validation unless the change itself is app-wide/repo-wide or the user explicitly requests it.

## App execution

- UI changes may be verified by launching LoopCode and using computer-control, GUI automation, screenshots, or window inspection.
- Check for an existing LoopCode process before launching the app. Treat every pre-existing instance as user-owned: do not stop, replace, attach to, or interact with it.
- Track any LoopCode process started for verification and stop only that process when verification is complete.
- Run the scoped non-interactive checks required above before visual verification.
