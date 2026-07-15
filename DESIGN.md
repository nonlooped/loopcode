---
name: LoopCode
description: Calm, premium desktop cockpit for a local AI coding agent
colors:
  deep-spruce: "#1f6f63"
  deep-spruce-strong: "#185a50"
  deep-spruce-tint: "#e8f2f0"
  deep-spruce-line: "#b9d4ce"
  harvest-amber: "#a16207"
  harvest-amber-tint: "#f5edd8"
  terracotta: "#b4533a"
  terracotta-tint: "#f3e4df"
  gallery-paper: "#fafafa"
  gallery-surface: "#ffffff"
  gallery-surface-2: "#f5f5f5"
  ink: "#171717"
  ink-2: "#525252"
  ink-3: "#a3a3a3"
  hairline: "#e5e5e5"
  hairline-2: "#d4d4d4"
typography:
  display:
    fontFamily: "Iowan Old Style, Palatino Linotype, Palatino, Book Antiqua, Georgia, serif"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  body:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 600
  code:
    fontFamily: "ui-monospace, SF Mono, Cascadia Code, Consolas, monospace"
    fontSize: "13px"
rounded:
  control: "7px"
  card: "11px"
  panel: "16px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.deep-spruce}"
    textColor: "#ffffff"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-primary-hover:
    backgroundColor: "{colors.deep-spruce-strong}"
  button-secondary:
    backgroundColor: "{colors.gallery-surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-danger:
    backgroundColor: "{colors.terracotta-tint}"
    textColor: "{colors.terracotta}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  chip-accent:
    backgroundColor: "{colors.deep-spruce-tint}"
    textColor: "{colors.deep-spruce}"
    rounded: "{rounded.pill}"
    padding: "4px 10px"
  card:
    backgroundColor: "{colors.gallery-surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.panel}"
  input:
    backgroundColor: "{colors.gallery-surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
---

# Design System: LoopCode

## 1. Overview

**Creative North Star: "The Quiet Studio"**

LoopCode's interface is a serene workspace in the Apple-material tradition: translucent chrome, soft light, hairline structure, work treated as craft. An AI agent that edits files and runs commands is inherently intimidating the first time; the visual system's job is to lower the user's pulse. Nothing shouts. Color always means something, motion always conveys state, and the highest-traffic paths (palette, tab switches, mode cycling) stay perfectly still. The system explicitly rejects the generic AI SaaS template (purple gradients, decorative glassmorphism, sparkle icons), enterprise admin blandness, and the dark-only mono-everywhere dev-terminal aesthetic — all named anti-references in PRODUCT.md.

Density is calm, not sparse: an 8px rhythm with generous padding, hairline dividers instead of boxes-in-boxes, and two neutral surface steps (paper for the conversation, a barely-lifted second neutral for rails and sidebars). Both light and dark themes are first-class, driven by `data-theme` tokens, with high-contrast, reduced-motion, and reduced-transparency variants honored throughout.

**Key Characteristics:**
- True-neutral grounds (equal-RGB grays) — no warm paper cast, no cool slate cast
- One spruce accent that *means* "calm / safe / read-only"; amber and clay are states, never decoration
- Serif display voice over a humanist system-sans body; mono strictly for machine output
- Ambient-whisper elevation: hairline borders and surface steps carry hierarchy, shadows are barely-there
- Quietly tactile interaction: instant press feedback (scale 0.97–0.98), sub-300ms transitions, stillness on hot paths

## 2. Colors

A restrained palette where every hue is a semantic state on a true-neutral ground.

### Primary
- **Deep Spruce** (#1f6f63, dark theme #5cb4a6): the single brand accent. Marks primary actions, current selection, focus rings, and the calm/read-only agent modes (Ask, Plan). Its rarity is what makes it legible.
- **Deep Spruce Strong** (#185a50): hover deepening of primary actions.
- **Deep Spruce Tint** (#e8f2f0) with **Deep Spruce Line** (#b9d4ce): selected-state fills for rows, chips, and choice cards.

### Secondary
- **Harvest Amber** (#a16207, dark #d4a017) on **Amber Tint** (#f5edd8): the attention state — Build mode, pending approvals, warnings. Amber says "look here before proceeding," never "error."

### Tertiary
- **Terracotta** (#b4533a, dark #d98063) on **Terracotta Tint** (#f3e4df): danger and denial — destructive actions, deny buttons, validation errors. Always paired with its tint, never full-saturation fills.

### Neutral
- **Gallery Paper** (#fafafa, dark #181818): the main conversation ground.
- **Gallery Surface** (#ffffff, dark #222222): elevated cards, the composer, menus.
- **Gallery Surface 2** (#f5f5f5, dark #1c1c1c): rails and sidebars — one barely-perceptible step below paper.
- **Ink** (#171717) / **Ink 2** (#525252) / **Ink 3** (#a3a3a3): primary text, secondary text, placeholders and disabled hints.
- **Hairline** (#e5e5e5) / **Hairline 2** (#d4d4d4): dividers and control borders.

### Named Rules
**The Spruce Means Safe Rule.** Color is state, never decoration. Spruce = calm/safe/selected, amber = attention/Build, clay = danger/deny. A hue outside its meaning is a bug.

**The One Voice Rule.** The spruce accent covers ≤10% of any screen. When everything is accented, nothing is.

## 3. Typography

**Display Font:** Iowan Old Style (with Palatino, Georgia serif fallbacks)
**Body Font:** system-ui humanist sans (Segoe UI / SF / Roboto per platform)
**Label/Mono Font:** ui-monospace (SF Mono, Cascadia Code, Consolas)

**Character:** A warm serif voice for moments of address (onboarding welcomes, section headings) over a quiet, native-feeling sans for everything operational. The pairing contrasts on a real axis — serif vs. humanist sans — and makes the app read as considered rather than default.

### Hierarchy
- **Display** (serif, 600, line-height 1.15, letter-spacing -0.02em): onboarding heroes and view headings; the only place the serif appears at size.
- **Body** (sans, 400, 16px default / 15px in inputs, line-height ~1.5): conversation, prose, descriptions. Prose runs are capped near 70ch.
- **Label** (sans, 600, 13–13.5px): field labels, chips, list rows, control text.
- **Code** (mono, 13px): code blocks, terminal, file paths, diffs.

### Named Rules
**The Mono Is For Machines Rule.** Monospace appears only where the machine is speaking — code, terminal, paths, diffs. Never in UI labels, buttons, or headings.

## 4. Elevation

Depth is an ambient whisper, not a structural device. Surfaces at rest are defined by hairline borders (#e5e5e5) and the two-step neutral ladder (paper → surface-2 → surface); shadows exist but read as air, not weight. Shadow strength grows only with floating height: inline cards wear shadow-1, the composer and menus shadow-2, drawers and dialogs shadow-3. Overlaid chrome (top bar, floating panels) uses the translucent material treatment — `--material-chrome` at ~78% surface opacity with 20px backdrop blur and 180% saturation — which collapses to solid surfaces when the OS asks for reduced transparency.

### Shadow Vocabulary
- **Whisper** (`0 1px 2px rgba(0,0,0,0.04), 0 1px 1px rgba(0,0,0,0.03)`): resting cards and the primary button.
- **Lift** (`0 4px 14px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04)`): the composer, popovers, menus.
- **Float** (`0 18px 48px rgba(0,0,0,0.10), 0 4px 12px rgba(0,0,0,0.06)`): drawers, dialogs, the command palette.

### Named Rules
**The Height Earns Shadow Rule.** A surface's shadow step matches how far it floats above the page. Nothing at rest gets more than Whisper; nothing borrows Float for emphasis.

## 5. Components

Quietly tactile: every pressable responds on pointer-down (scale 0.97–0.98, 100ms ease-out) but stays visually reserved. Transitions name their properties (background-color, border-color, transform…) — never `all`.

### Buttons
- **Shape:** softly squared (7px radius); text 14px semibold.
- **Primary:** Deep Spruce fill, white text, Whisper shadow (padding 10px 16px); hover deepens to Spruce Strong.
- **Hover / Focus:** hover states fire only on fine pointers (`hover: hover`); focus-visible draws a 2px spruce outline offset 2px, everywhere.
- **Secondary:** white surface, ink text, Hairline 2 border; hover darkens the border. **Ghost:** transparent, ink-2 text; hover fills with surface. **Danger:** Terracotta Tint fill with Terracotta text — tinted, never full-saturation.

### Chips
- **Style:** full-pill (999px), 13px semibold, tinted fill + matching border per tone (neutral / accent / amber).
- **State:** optional 6px status dot so state never relies on color alone; a slow opacity pulse (1.6s) on the dot only — never the chip — for live/running status. A `quiet` variant drops border and fill for secondary metrics.

### Cards / Containers
- **Corner Style:** panels 16px, choice cards 11px.
- **Background:** Gallery Surface on Gallery Paper.
- **Shadow Strategy:** Whisper at rest (see Elevation).
- **Border:** always a Hairline — borders carry the structure, the shadow is atmosphere.
- **Internal Padding:** 16–24px on the 8px rhythm.

### Inputs / Fields
- **Style:** white surface, Hairline 2 border, 7px radius, 15px text, 13px semibold label above (padding 10px 14px); placeholders in Ink 3.
- **Focus:** border shifts to spruce plus the standard 2px spruce outline (160ms transition). Inside the composer card, the card carries focus (focus-within) and the field ring is suppressed.
- **Error / Disabled:** error text in Terracotta below the field; disabled drops to 50% opacity with interaction removed.

### Navigation
- **Style:** list rows at 13.5px in Ink 2, 6px radius, truncating; hover fills with surface (fine pointers only); the active row takes Spruce Tint + spruce semibold text. Rails sit on Gallery Surface 2, one hairline away from paper.

### Choice Cards (signature)
- Onboarding's provider/mode pickers: 11px-radius bordered cards; hover warms the border to Spruce Line; the selected card takes a spruce border, Spruce Tint fill, and a 3px tint halo.

## 6. Do's and Don'ts

### Do:
- **Do** spend spruce only on meaning: primary action, current selection, focus, calm/read-only modes. ≤10% of any screen.
- **Do** keep high-frequency keyboard paths (command palette, tab switch, mode cycle) motionless; reserve motion for rare, state-bearing moments (an approval arriving, onboarding steps, a message landing).
- **Do** honor every OS preference: reduced motion collapses travel to fades, reduced transparency solidifies chrome, increased contrast strengthens ink and hairlines.
- **Do** build hierarchy from hairlines and the paper → surface-2 → surface ladder before reaching for shadow.
- **Do** pair every status color with a non-color cue (dot, icon, label) — state never relies on hue alone.

### Don't:
- **Don't** ship the "generic AI SaaS" look PRODUCT.md bans: purple gradients, gradient text, sparkle icons, hero metrics, glassmorphism as decoration. Translucency here is chrome material, not card styling.
- **Don't** drift into "enterprise admin blandness": no default-framework gray tables and forms without a point of view.
- **Don't** lapse into the "dev-tool terminal aesthetic": no dark-only assumption, no mono outside machine output, no dense hacker chrome — the beginner is the primary user.
- **Don't** animate for decoration, use bounce/elastic curves, or exceed ~300ms; ease out with the exponential house curves.
- **Don't** use full-saturation fills for warning or danger states — amber and clay always sit on their tints.
- **Don't** transition `all`, and don't attach hover states to coarse pointers.
