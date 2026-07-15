/**
 * Shared interaction language — every pressable should feel like it listens.
 * Press feedback is instant (Apple: respond on pointer-down via :active).
 */

/** Base pressable: transform-only press scale, named transitions (never `all`). */
export const pressable =
  "cursor-pointer select-none " +
  "transition-[background-color,border-color,color,transform,opacity,box-shadow] " +
  "duration-[var(--duration-press)] ease-[var(--ease-out)] " +
  "active:scale-[0.98] " +
  "disabled:opacity-50 disabled:pointer-events-none disabled:active:scale-100";

/** List / rail / tree rows. */
export const listRow =
  pressable +
  " w-full truncate rounded-md px-2 py-1.5 text-left text-[13.5px] " +
  "focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent";

export const listRowIdle = "text-ink-2 fine-hover:bg-surface";
export const listRowActive = "bg-accent-tint font-semibold text-accent";

/** Compact control chrome (selects, small chips-as-buttons). */
export const controlChrome =
  pressable +
  " inline-flex items-center gap-2 rounded-control border border-line-2 bg-surface " +
  "px-3 py-1.5 text-[13px] font-semibold text-ink fine-hover:border-ink-3 " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent";

/** Text inputs: border focus with short named transition. */
export const inputChrome =
  "w-full rounded-control border border-line-2 bg-surface px-3.5 py-2.5 text-[15px] text-ink " +
  "placeholder:text-ink-3 " +
  "transition-[border-color,box-shadow] duration-[var(--duration-fast)] ease-[ease] " +
  "focus:border-accent focus:outline-2 focus:outline-offset-1 focus:outline-accent";

/** Choice cards (onboarding provider picker). */
export const choiceCard =
  pressable +
  " rounded-card border px-4 py-3.5 text-left " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent " +
  "fine-hover:border-accent-line";

export const choiceCardSelected =
  "border-accent bg-accent-tint text-accent shadow-[0_0_0_3px_var(--accent-tint)]";
export const choiceCardIdle = "border-line-2 bg-surface text-ink";
