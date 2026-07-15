import { Popover } from "@base-ui/react/popover";
import { Slider } from "@base-ui/react/slider";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconBrain, IconChevronDown } from "@tabler/icons-react";
import { cn } from "./cn";
import { pressable } from "./interactive";

export interface ReasoningSliderOption {
  value: string;
  /** Short label shown on the trigger and as the tick's current-value readout. */
  label: string;
  description: string;
}

interface ReasoningSliderProps {
  value: string;
  onValueChange: (value: string) => void;
  /** Stops in low-to-high order — leftmost is fastest, rightmost is smartest. */
  items: ReasoningSliderOption[];
  "aria-label"?: string;
  disabled?: boolean;
  triggerClassName?: string;
}

const ghostChrome =
  pressable +
  " inline-flex items-center gap-1.5 rounded-control border border-transparent bg-transparent " +
  "px-2 py-1 text-[13px] font-medium text-ink-2 " +
  "fine-hover:bg-surface-2 fine-hover:text-ink " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent";

/**
 * Codex-style reasoning picker: a compact trigger opens a popover holding a
 * discrete slider from Faster (least reasoning) to Smarter (most reasoning).
 * The slider operates on the stop *index*, not the effort string, so spacing
 * stays even regardless of how many stops a model exposes.
 */
export function ReasoningSlider({
  value,
  onValueChange,
  items,
  "aria-label": ariaLabel,
  disabled = false,
  triggerClassName,
}: ReasoningSliderProps) {
  const reduceMotion = useReducedMotion();
  const activeIndex = Math.max(
    0,
    items.findIndex((i) => i.value === value),
  );
  const active = items[activeIndex] ?? items[0];
  const lastIndex = Math.max(0, items.length - 1);

  const tickPositions = items.map((_, i) => (lastIndex === 0 ? 0 : (i / lastIndex) * 100));

  if (items.length === 0) return null;

  return (
    <Popover.Root>
      <Popover.Trigger
        aria-label={ariaLabel}
        disabled={disabled}
        className={cn(
          ghostChrome,
          "min-w-0",
          disabled && "opacity-50",
          triggerClassName,
        )}
      >
        <IconBrain
          size={14}
          stroke={1.75}
          className={cn("shrink-0", value === "off" ? "opacity-40" : "opacity-80")}
          aria-hidden
        />
        {/* Stack every label invisibly so the trigger keeps the width of the
            longest one — switching modes must not reflow the row or move the
            popover anchored to this trigger. */}
        <span className="grid min-w-0 text-left">
          {items.map((item) => (
            <span
              key={item.value}
              aria-hidden
              className="invisible col-start-1 row-start-1 whitespace-nowrap"
            >
              {item.label}
            </span>
          ))}
          <span
            className={cn(
              "col-start-1 row-start-1 truncate",
              value === "off" && "text-ink-2",
            )}
          >
            {active?.label}
          </span>
        </span>
        <IconChevronDown size={14} stroke={2} className="shrink-0 text-ink-3" aria-hidden />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner sideOffset={8} align="center" className="z-50">
          <Popover.Popup
            className={cn(
              "surface-popover w-64 rounded-card border border-line bg-surface p-3.5 shadow-soft-3 outline-none",
            )}
          >
            {/* Motion (not CSS remount) animates the readout: interruptible
                crossfades stay fluid when a fast drag crosses several stops,
                where restarting CSS animations read as flicker. */}
            <div className="flex items-baseline gap-1.5 text-[13px] font-bold">
              <span className="text-ink">Effort</span>
              <span className="relative min-w-0">
                <AnimatePresence mode="popLayout" initial={false}>
                  <motion.span
                    key={active?.value}
                    initial={{ opacity: 0, y: reduceMotion ? 0 : 5 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: reduceMotion ? 0 : -5 }}
                    transition={{ duration: 0.16, ease: [0.23, 1, 0.32, 1] }}
                    className="inline-block truncate text-accent"
                  >
                    {active?.label}
                  </motion.span>
                </AnimatePresence>
              </span>
            </div>
            <div className="relative mt-0.5 min-h-[2lh] text-[12px] leading-snug text-ink-2">
              <AnimatePresence mode="popLayout" initial={false}>
                <motion.p
                  key={active?.value}
                  initial={{ opacity: 0, y: reduceMotion ? 0 : 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: reduceMotion ? 0 : -4 }}
                  transition={{ duration: 0.16, ease: [0.23, 1, 0.32, 1] }}
                >
                  {active?.description}
                </motion.p>
              </AnimatePresence>
            </div>

            <div className="mt-3.5 flex items-center justify-between text-[13px] font-semibold text-ink-2">
              <span>Faster</span>
              <span>Smarter</span>
            </div>
            <div className="mt-1.5">
              <Slider.Root
                className="relative block"
                min={0}
                max={lastIndex}
                step={1}
                value={activeIndex}
                disabled={disabled}
                onValueChange={(v) => {
                  const next = items[v as number];
                  if (next) onValueChange(next.value);
                }}
                format={{ style: "decimal" }}
                aria-label={ariaLabel}
                aria-valuetext={active?.label}
              >
                <Slider.Control className="relative flex h-6 w-full touch-none items-center">
                  <Slider.Track className="relative h-3 w-full rounded-full bg-surface-2">
                    <Slider.Indicator
                      className="absolute h-full rounded-full bg-accent motion-safe:transition-[width] motion-safe:duration-[var(--duration-fast)] motion-safe:ease-[var(--ease-out)]"
                    />
                    {tickPositions.map((pct, i) => (
                      <span
                        key={i}
                        aria-hidden
                        className={cn(
                          "pointer-events-none absolute top-1/2 h-1.5 w-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full",
                          "transition-colors duration-[var(--duration-fast)] ease-[var(--ease-out)]",
                          i <= activeIndex ? "bg-accent-foreground/60" : "bg-line-2",
                        )}
                        style={{ left: `${pct}%` }}
                      />
                    ))}
                  </Slider.Track>
                  {/* Base UI positions the thumb via inset-inline-start; transition
                      it so keyboard/click stop changes glide instead of teleport. */}
                  <Slider.Thumb
                    className={cn(
                      pressable,
                      "slider-thumb-glide block h-6 w-6 rounded-full border-[2.5px] border-accent bg-surface shadow-soft-2",
                      "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
                    )}
                  />
                </Slider.Control>
              </Slider.Root>
            </div>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  );
}
