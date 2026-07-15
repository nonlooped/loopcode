# Security policy

## Reporting a vulnerability

**Do not** open a public GitHub issue with exploit details, secret material, or
proof-of-concept payloads against production users.

Report security issues privately:

1. Email **security@loopcode.dev** (preferred), or
2. Contact the project maintainers via a private channel linked from the
   GitHub organization / repository security advisories, or
3. If neither is available yet, message the repository owner privately
   (see [CONTRIBUTING.md](../CONTRIBUTING.md)).

Please include:

- Affected version / commit
- Description of the impact (e.g. approval bypass, secret leakage, path escape)
- Steps to reproduce (private)
- Whether you plan public disclosure and preferred timeline

We aim to acknowledge reports within **7 days** and to coordinate fixes before
any public write-up.

## Supported versions

LoopCode v1 RC (`1.0.0` package line) is the supported branch for security
fixes during the v1 lifecycle. Pre-release / development trees may not receive
backports.

## Security model (summary)

- **Core owns** tools, shell, PTY, secrets, SQLite, approvals, and network
  adapters. The WebView is capability-gated (`core:default` only by default).
- **Shell, network, MCP, skill scripts** always require approval under the
  fixed matrix (`security-v1`).
- **Secrets** are stored as refs; export redacts secret-shaped content.
- **Diagnostics upload** is off by default and never includes source, prompts,
  tool args, or secrets when disabled (no upload traffic).

See also: [known-limitations.md](./known-limitations.md),
[install.md](./install.md).
