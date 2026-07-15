# Product

## Register

product

## Platform

web

## Users

Beginner-first, experts secondary. The primary user is new to AI coding agents: they need guidance, safe defaults, and approvals they can actually read. They arrive curious but wary — an agent that edits files and runs commands is inherently scary the first time. Experts are the secondary audience: the keyboard map and command palette keep them fast, but onboarding and defaults never assume their fluency.

## Product Purpose

LoopCode is a local AI coding agent desktop app (Tauri 2: Rust core + React webview). The user connects a provider, opens a project, and works through Ask / Plan / Build / Debug modes in a cockpit: conversation timeline and composer in the center, project navigation left, Files / Diffs / Terminal / Diagnostics right. Every agent action passes a fixed approval matrix and lands in an audit log.

Success is a safe first session: a newcomer gets from install to a first successful, non-scary agent run in minutes, without reading documentation.

## Positioning

Approachable power — the first agent cockpit a non-expert can trust on day one, without giving up the depth experts need.

## Brand Personality

Calm, premium, trustworthy. Quiet confidence and restraint; nothing shouts for attention. The reference feel is Claude / ChatGPT desktop (a conversation-centered layout that makes an AI feel legible and low-stakes) and Apple system apps (translucent materials, spatial consistency, typography discipline — already echoed in the theme tokens).

## Anti-references

- Generic AI SaaS: purple gradients, glassmorphism, sparkle icons, hero metrics — the 2024–26 AI-product template.
- Enterprise admin blandness: gray tables, default-framework forms, no point of view at all.
- Dev-tool terminal aesthetic: dark-only, mono-everywhere, dense hacker chrome — intimidating to the beginner audience.

## Design Principles

1. **Calm is the feature.** Restraint in color and motion is what makes an agent feel safe. High-frequency paths stay still; emphasis is spent only where something genuinely needs attention.
2. **Make agency legible.** The user should always understand what the agent is doing, did, and wants to do next. Approvals read as clear, low-stakes decisions — never as alarming interrupts.
3. **Beginner-first, never beginner-only.** Copy, defaults, and onboarding assume no prior agent experience; depth (palette, keyboard map, advanced settings) stays one summon away rather than on screen by default.
4. **Familiarity earns trust.** Standard affordances and one consistent component vocabulary across every surface; surprise is spent on moments, not screens.

## Accessibility & Inclusion

Best-effort, no formal conformance target. Keep the shipped Slice 10 behaviors: keyboard focus order, live regions, default + high-contrast themes, and honoring reduced motion / reduced transparency / increased contrast preferences.
