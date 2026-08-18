# LoopCode Design System

This document is the visual contract for LoopCode. It is derived from the current app, not from a generic component library. Preserve the character first; reuse exact values where practical; use judgment where content or platform behavior requires an exception.

## 1. Design character

LoopCode is a **quiet, native-feeling, dark desktop workspace**. It should feel like one continuous acrylic surface, not a web dashboard placed inside a window.

The design is:

- **Content-first:** the transcript and prompt are primary; chrome recedes.
- **Low-chroma:** neutral greys carry hierarchy. Color communicates status, native identity, or file type—not decoration.
- **Translucent:** the native acrylic is visible through the shell and subordinate surfaces.
- **Compact:** controls are intentionally dense and desktop-sized.
- **Softly structured:** spacing, alignment, and small alpha shifts do more work than borders.
- **Quietly responsive:** transitions explain structural changes without calling attention to themselves.
- **Agent-neutral:** provider identity may appear where useful, but no provider owns the product aesthetic.

### Non-negotiable principles

1. Do not turn the UI into a collection of opaque cards.
2. Do not introduce a bright brand color as the default accent.
3. Do not use color where spacing, weight, or opacity can express the hierarchy.
4. Do not add borders to every region. Borders mark real boundaries, not every container.
5. Do not add decorative gradients, glows, large shadows, or glass effects without a functional layering reason.
6. Do not replace compact desktop controls with oversized mobile-style controls.
7. Do not hide essential actions behind hover alone; hover-only shortcuts must remain keyboard reachable or have another visible route.
8. Respect reduced-motion and reduced-transparency preferences.
9. Do not abstract one-off values merely to make the system look more systematic. Repeated semantic values belong in tokens; deliberate local optical corrections may remain local.

## 2. Source of truth

The system currently lives in plain CSS imported by `src/app.css`:

- `src/styles/base.css` — global tokens, typography, focus, selection, scrollbars
- `src/styles/context-menu.css` — context menus and image preview overlay
- `src/styles/shell.css` — acrylic shell, titlebar, window chrome, structural grid
- `src/styles/sidebar.css`, `thread-list.css`, `sidebar-footer.css` — navigation hierarchy
- `src/styles/project-explorer.css` — project tree and right rail
- `src/styles/conversation.css` — settings and conversation framing
- `src/styles/messages.css`, `activity.css`, `tools.css` — transcript content and agent activity
- `src/styles/composer.css`, `composer-controls.css` — prompt surface and primary actions
- `src/styles/model-picker.css`, `reasoning-picker.css` — floating selection surfaces
- `src/styles/modal.css` — permission decisions
- `src/styles/responsive.css` — compact layouts and user preferences

This is intentionally not a component-library project. Add a shared component only when behavior and markup genuinely repeat; shared visual rules can remain CSS.

## 3. Color and surfaces

### Core tokens

Use the tokens in `base.css` before adding a literal color.

| Token                |                Current value | Role                                   |
| -------------------- | ---------------------------: | -------------------------------------- |
| `--shell-tint`       |     `rgba(13, 14, 17, 0.66)` | Main acrylic tint                      |
| `--shell-solid`      |     `rgba(23, 24, 27, 0.88)` | Reduced-transparency fallback          |
| `--sidebar-tint`     | `rgba(255, 255, 255, 0.026)` | Side-rail separation                   |
| `--floating-surface` | `rgba(255, 255, 255, 0.058)` | Elevated translucent surface           |
| `--panel`            | `rgba(255, 255, 255, 0.032)` | Quiet contained surface                |
| `--panel-hover`      | `rgba(255, 255, 255, 0.048)` | Hovered surface                        |
| `--panel-active`     | `rgba(255, 255, 255, 0.072)` | Selected/current surface               |
| `--line`             | `rgba(255, 255, 255, 0.065)` | Subtle divider or border               |
| `--line-strong`      | `rgba(255, 255, 255, 0.115)` | Focused/elevated boundary              |
| `--text`             |  `rgba(249, 249, 250, 0.94)` | Primary text                           |
| `--text-soft`        |         `rgb(210, 211, 214)` | Secondary but readable text            |
| `--muted`            |         `rgb(174, 176, 182)` | Metadata and inactive controls         |
| `--faint`            |         `rgb(145, 148, 156)` | Tertiary metadata and decorative icons |
| `--accent`           |  `rgba(236, 236, 238, 0.92)` | Primary action fill                    |
| `--accent-hover`     |  `rgba(255, 255, 255, 0.98)` | Primary action hover                   |
| `--success`          |  `rgba(136, 180, 154, 0.92)` | Running/ready/success                  |
| `--danger`           |  `rgba(215, 126, 137, 0.92)` | Failure, destructive action            |
| `--warning`          |  `rgba(212, 170, 114, 0.92)` | Caution                                |

### Text hierarchy

Use this order; do not invent a new grey for each component:

1. `--text` — authored content, headings, active primary labels
2. `--text-soft` — important UI labels and selected values
3. `--muted` — inactive controls, descriptions, metadata
4. `--faint` — timestamps, paths, tertiary icons, placeholders when appropriate

`--text-soft`, `--muted`, and `--faint` remain opaque so their contrast is stable over acrylic. Semantic body text must retain at least 4.5:1 contrast against the effective dark shell.

### Surface hierarchy

Use elevation sparingly:

1. **Shell:** native acrylic plus `--shell-tint`, `blur(44px) saturate(118%)`.
2. **Rails:** transparent white tint via `--sidebar-tint`; separate with one 1px low-alpha edge.
3. **Inline states:** `--panel-hover` and `--panel-active`; do not create a card.
4. **Contained cards:** approximately `rgba(255,255,255,0.022–0.032)` with `--line`, only when grouping is useful.
5. **Floating menus/modals:** `rgba(30,31,35,0.96)`, 20px blur, a fine top highlight, and a deep directional shadow.
6. **Code/detail wells:** translucent black (`0.16–0.24`) to recess rather than elevate.

Surface colors that sit over the shell must retain alpha. An opaque dark replacement breaks the acrylic design.

### Permitted color exceptions

- Native red/yellow/green window controls
- Provider logos
- Material file and folder icons
- Semantic success, danger, and warning states
- Syntax highlighting inside code blocks

Keep those colors local. They must not leak into general navigation, buttons, or decoration.

## 4. Typography

### Families

- UI and content: `"Segoe UI Variable Text", "Segoe UI", "Helvetica Neue", sans-serif`
- Code and machine-readable values: `"Cascadia Mono", Consolas, monospace`

Do not add a webfont. The native system stack is part of the desktop character and avoids font-loading noise.

### Scale

The app uses a compact optical scale rather than a large marketing-site scale:

| Use                       |      Size | Typical weight/line height |
| ------------------------- | --------: | -------------------------- |
| Tiny labels, sources      |    9–10px | 520–600                    |
| Metadata, picker controls | 10–10.5px | 500–600                    |
| Dense rows, tool labels   | 11–11.5px | 540–600                    |
| Supporting copy           |   12–13px | 400–550, 1.4–1.5           |
| Transcript body           |    13.5px | regular, 1.55              |
| Settings row title        |    13.5px | 550                        |
| Modal title               |      15px | 590                        |
| Page title                |      16px | 650                        |

Use weight before size to strengthen hierarchy. Avoid bold weights above 650. UI headings use slight negative tracking only at 15–16px; labels should not look editorial or promotional.

Markdown headings are relative to transcript text: `1.55em`, `1.35em`, `1.18em`, then `1em`. They remain restrained and never become page-sized display type.

Uppercase is reserved for tiny category labels such as permission field names. Pair it with modest tracking (`0.04–0.055em`); never uppercase ordinary navigation or actions.

## 5. Spacing and density

The working spacing vocabulary is compact: **2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 20, and 24px**. Prefer the smaller values inside controls and the larger values between content groups.

Common patterns:

- Icon-to-label: 5–8px
- Adjacent compact controls: 2–4px
- Row horizontal padding: 7–10px in rails, 14–20px in settings/modals
- Section separation: 12–24px
- Transcript message rhythm: 17–28px depending on authorship and grouping
- Main content gutters: 24–28px on desktop, 15px around the transcript on narrow windows

Do not force every gap onto a mathematical scale. Optical alignment wins when icons, text baselines, or asymmetric controls need a 1px correction.

## 6. Shape, borders, and shadow

### Radius vocabulary

- 4–5px: inline code, tiny tool wells, file rows
- 6–8px: ordinary buttons, rows, inputs, segmented controls
- 9–10px: attachments, icons, search fields, compact cards
- 12–13px: settings cards, menus, modals, shell corners
- 17–18px: user message bubble and composer
- `999px` / 50%: status dots, circular actions, toggles, pills

Radii express scale. Do not apply 12–18px rounding to every small control.

### Borders

- Default boundary: `1px solid var(--line)`
- Strong/focused boundary: `1px solid var(--line-strong)`
- Rail separators: approximately `rgba(255,255,255,0.052)`
- Transparent borders may reserve geometry for hover/selected states.

A region that is already separated by layout, tint, or spacing usually does not need another border.

### Shadows

Use shadows only for the window shell, detached overlays, side drawers, and the composer/scroll affordance. Inline rows and selected navigation items stay shadowless.

- Shell: broad and soft (`0 28px 90px rgba(0,0,0,0.4)`)
- Floating overlay: deep (`0 22–24px 58–65px rgba(0,0,0,0.4–0.42)`)
- Composer: restrained (`0 12px 38px rgba(0,0,0,0.18–0.2)`)
- Add a very faint inset top highlight to glass/elevated surfaces, never a bright rim.

## 7. Layout

### Desktop frame

- Window: frameless, transparent Tauri window with native acrylic and shadow
- Shell radius: 12px; 0 when maximized
- Titlebar height: 38px
- Left sidebar default: 256px
- Right project explorer default: 280px
- Conversation column: `minmax(0, 1fr)`
- Transcript and composer measure: maximum 720px
- Settings measure: maximum 768px

The transcript, composer, and title context align around the central working column. Keep primary reading width constrained even on large windows.

Sidebars may be user-resizable, but resizing must not change typography or row density. The current intended ranges live in `src/utils/app-settings.ts`; use those values rather than duplicating limits in CSS.

### Responsive behavior

- At `1100px`, side rails tighten to roughly 255px/252px.
- At `880px`, the fixed three-column grid becomes an overlay/drawer layout.
- At `680px`, titlebar chrome tightens, secondary title metadata disappears, transcript gutters shrink, and modal actions stack.

Responsive changes should preserve hierarchy, not merely scale everything down. Hide tertiary information before primary labels or actions. Do not introduce a parallel mobile design unless LoopCode becomes a touch product.

## 8. Component rules

### Titlebar and window chrome

- Keep it 38px tall, visually transparent, and draggable in all non-control regions.
- Traffic controls retain native colors and 12px circles.
- Chrome icon buttons are 25px square, transparent at rest, and use the standard hover tint.
- Title text stays small: important context at 11.5px, secondary folder context at 10px.
- Provider identity is monochrome and low-opacity here.

### Sidebars and navigation rows

- Rails are one tinted plane; do not wrap every section in a card.
- Ordinary items are transparent at rest, `--panel-hover` on hover, `--panel-active` when current.
- Standard navigation radius is 8px.
- Primary thread titles are 13px/500. Metadata is 10px and visually subordinate.
- Destructive and archival actions may appear contextually, but must retain labels/tooltips and focus access.
- Empty states are brief, muted, and unillustrated.

### Project tree

- File rows are 25px tall with 14px indentation per depth.
- File type is communicated through material icons; text remains neutral.
- Rows do not become selected cards merely because they can be opened.
- Loading, empty, and failure states stay inline with the tree.

### Transcript

- Maximum reading width is 720px.
- Assistant output sits directly on the shell—no assistant bubble.
- User input may use the single quiet bubble treatment: subtle white tint, subtle border, 17px radius, maximum 78% width (92% narrow).
- Do not add avatars, repeated role labels, timestamps, or per-message toolbars unless a demonstrated use case requires them.
- Markdown rhythm is compact and readable; code wells are recessed and horizontally scrollable.
- Agent work is progressively disclosed in `<details>`, connected by a subtle vertical rule.
- Errors use `--danger`; routine notices remain muted.
- Scroll fades and “Scroll to bottom” are functional affordances, not decoration.

### Composer

- The composer is the strongest persistent control, but remains transparent rather than becoming a heavy panel.
- Maximum width is 720px, radius 18px, and border alpha rises slightly on focus.
- Compact layout keeps attachment, prompt, context, and send action in one row.
- Expanded layout moves the prompt above controls when content requires it; do not use a fixed row count.
- Primary send/cancel actions are 30px circles with the neutral light accent.
- Context controls are small, transparent, and truncate rather than expanding the composer.
- Metadata below the composer is 9.5px and tertiary.

### Menus and pickers

- Menus float above the composer with near-opaque dark glass, 12–13px radius, fine border, and deep shadow.
- Options are transparent at rest. Hover and selection use small white-alpha shifts, not an accent color.
- Search inputs remain integrated into the menu surface.
- Provider rails may show small semantic status dots.
- Long values truncate; menus stay within the viewport.

### Context menus

Context menus are the denser, faster variant of the floating-menu language. They use the same dark glass and neutral states, but should not inherit the larger picker proportions.

- Surface: `rgba(29, 30, 34, 0.96)`, `var(--line-strong)`, 10px radius, 24px blur, and a deep shadow with a faint inset top highlight.
- Geometry: 5px outer padding, 186px minimum width, 280px maximum width, and an 8px viewport safety margin.
- Items: at least 29px tall, 8px horizontal padding, 6px radius, 11.5px text, and transparent at rest.
- Hover and keyboard focus share `--panel-active` with `--text`. A filled focused row replaces the global focus ring inside this `role="menu"` pattern.
- Keep labels short, verb-first, and single-line. Put optional keyboard shortcuts on the right in 10px `--faint` text.
- Use separators only to divide meaningfully different action groups, especially before a destructive final action.
- Use danger color only for destructive or irreversible actions; a danger row receives a quiet danger tint on hover/focus.
- Disabled actions remain in place at reduced opacity when their presence explains capability; otherwise omit them.
- Menus must clamp to the viewport, focus the first enabled item, wrap Arrow Up/Down navigation, close on Escape/outside interaction/window blur, and respect reduced motion.
- Right-click actions must not be the only route to essential functionality. Keyboard context-menu invocation and ordinary visible controls must remain viable where the action is essential.
- Do not add icons by default. Add one only when it improves scanning across a long or structurally mixed menu.

### Settings

- Settings use one restrained card per coherent group, not one card per setting.
- Rows are at least 65px tall with a 36px framed icon well, title/description copy, and control aligned right.
- Separators appear only between rows in the same card.
- Toggles and segmented controls use the neutral accent when selected.

### Permission and question surfaces

- Permission decisions are true modal interruptions with `role="alertdialog"`, focus trapping, Escape behavior, and a dim/blur backdrop.
- The safe/default action may use the light neutral primary fill; rejection remains visually quieter unless danger requires stronger emphasis.
- Commands, paths, and raw details use monospace recessed wells.
- Question prompts occupy the composer position rather than adding another center-screen modal.

### Icons

- Use `@tabler/icons-svelte` for product controls.
- Typical size is 11–17px with strokes around 1.45–1.9; checks and tiny decisive marks may use 2.
- Icons support labels; they do not replace ambiguous labels without `aria-label` and `title`.
- Decorative icons use `aria-hidden` or empty `alt`.
- Do not mix in a second general-purpose icon family. Material icons are reserved for file types; provider assets are reserved for provider identity.

## 9. Interaction states

Every interactive control must define the states it can reach:

- **Rest:** transparent or the lowest necessary surface
- **Hover:** slight surface lift and/or text promotion
- **Active/selected:** `--panel-active` or the neutral accent, depending on control type
- **Focus-visible:** 2px light neutral outline with 2px offset (1px where geometry is very tight)
- **Disabled:** preserve layout, remove emphasis, use default cursor
- **Error:** use `--danger` for the smallest sufficient signal
- **Running/ready:** use `--success` sparingly, commonly as a dot or short status label

Do not use hover transforms that make dense rows jump. A 1px downward active transform is reserved for decisive round/action buttons.

## 10. Motion

Motion explains state and structure; it does not entertain.

- Color/background/border feedback: 120–150ms ease
- Context-menu entrance: 90ms ease-out; it must feel immediate
- Small fades: 110–180ms
- Structural panels and layout: 220–240ms with `cubic-bezier(0.32, 0.72, 0, 1)`
- Content reveal: 180ms with `cubic-bezier(0.22, 1, 0.36, 1)`
- Reordering: about 260ms
- Ongoing activity: slow opacity breathing around 1.15–1.4s

Avoid bounce, elastic overshoot, parallax, and decorative looping. When `prefers-reduced-motion: reduce`, remove nonessential animations and structural transitions. Keep state changes immediate and understandable.

## 11. Accessibility and platform behavior

These are design requirements, not optional polish:

- Use native `button`, `input`, `textarea`, `summary`, `fieldset`, and dialog semantics where they fit.
- Every icon-only control needs an accessible name; use a tooltip/title where sighted users also need clarification.
- Preserve keyboard activation and visible focus.
- Menus, trees, selected states, expanded states, and modal relationships must expose the corresponding ARIA state.
- Keep semantic grey text contrast stable over translucency.
- Do not communicate success/error/selection by color alone; pair it with text, position, icon, or state.
- Respect `prefers-reduced-transparency`: remove backdrop filters and provide a solid-enough shell.
- Keep native window drag regions explicit; nested text should not accidentally break dragging.
- Compact pointer targets are acceptable for this desktop app, but primary actions and narrow-window layouts must remain comfortably operable.

## 12. Copy and content

- Use short, direct sentence case: “New thread”, “Add folder…”, “Scroll to bottom”.
- Prefer product concepts such as project, session, model, and provider only where each is accurate.
- Keep product-facing copy agent-neutral.
- Empty states explain the state, not the implementation.
- Errors say what failed and, when possible, offer one clear recovery action.
- Use ellipses for actions that open a chooser or require more input; use the single Unicode ellipsis character.
- Do not add whimsical microcopy, marketing language, or explanatory paragraphs inside dense working surfaces.

## 13. Theming rules

LoopCode currently has one authored theme: dark acrylic. A future theme must be a token override, not a parallel set of component selectors.

Strict rules for any theme work:

1. Override semantic tokens at the root/theme boundary; do not fork component styles.
2. Preserve role relationships: `text > text-soft > muted > faint`, hover < active, line < line-strong.
3. Preserve translucency for shell-facing surfaces when platform acrylic is active.
4. Re-test effective contrast over both the native effect and the reduced-transparency fallback.
5. Keep the neutral accent strategy. A themed hue may tint status or identity, but must not color every selection and action.
6. Keep semantic success/danger/warning distinct from the accent and from one another.
7. Keep code syntax themes dark and visually subordinate to authored content.
8. Preserve density, typography, radii, layout, and motion unless the theme explicitly represents a different accessibility mode.
9. Platform-specific fallbacks may differ technically, but should preserve the same perceived hierarchy.
10. Do not add a theme switcher until a second complete, maintained theme exists.

Literal alpha colors are acceptable for deliberate optical layering. If the same literal acquires the same semantic meaning in three or more places, promote it to a token. Do not tokenise native traffic-light colors, syntax colors, file icons, or one-off optical corrections.

## 14. Review checklist

Before merging a visual change, check:

- Does it still read as one continuous desktop surface?
- Is the transcript/composer still the visual priority?
- Did the change reuse a semantic token or established surface recipe?
- Is any new color genuinely semantic or identity-bearing?
- Could spacing or weight replace a new border/card?
- Are rest, hover, selected, focus-visible, disabled, loading, and error states covered where relevant?
- Does long text truncate, wrap, or scroll intentionally?
- Does it work at wide desktop, around 880px, and below 680px?
- Does it remain usable with reduced motion and reduced transparency?
- Are icon-only controls named and keyboard reachable?
- Did the change avoid introducing a one-use component, token, or abstraction?
- If the change intentionally breaks a rule, is the user benefit clearer than the inconsistency it creates?

## 15. Common-sense exception rule

Consistency serves comprehension; it is not a goal by itself. Break a rule when platform behavior, accessibility, content shape, or a clearly better interaction requires it. Keep the exception local, reuse an existing pattern if one fits, and do not redesign neighboring components merely to justify the exception.
