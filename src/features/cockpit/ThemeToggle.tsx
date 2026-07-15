import { IconMoon, IconSun } from "@tabler/icons-react";
import { Button, cn } from "../../design";
import { useUi } from "../../store/ui";

/** Segmented sun/moon control — readable as a theme switch, not a form toggle. */
export function ThemeToggle() {
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);
  const isDark = theme === "dark";

  return (
    <div
      role="group"
      aria-label="Color theme"
      className="inline-flex items-center rounded-full border border-line-2 bg-surface-2 p-0.5"
    >
      <Button
        variant="ghost"
        type="button"
        aria-label="Light theme"
        aria-pressed={!isDark}
        title="Light"
        onClick={() => setTheme("light")}
        className={cn(
          "inline-flex h-7 w-7 items-center justify-center rounded-full p-0 outline-none",
          "transition-[background-color,color,box-shadow] duration-[var(--duration-fast)] ease-[var(--ease-out)]",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
          !isDark
            ? "bg-surface text-amber shadow-soft-1"
            : "text-ink-3 fine-hover:text-ink",
        )}
      >
        <IconSun size={15} stroke={1.75} aria-hidden />
      </Button>
      <Button
        variant="ghost"
        type="button"
        aria-label="Dark theme"
        aria-pressed={isDark}
        title="Dark"
        onClick={() => setTheme("dark")}
        className={cn(
          "inline-flex h-7 w-7 items-center justify-center rounded-full p-0 outline-none",
          "transition-[background-color,color,box-shadow] duration-[var(--duration-fast)] ease-[var(--ease-out)]",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
          isDark
            ? "bg-surface text-accent shadow-soft-1"
            : "text-ink-3 fine-hover:text-ink",
        )}
      >
        <IconMoon size={15} stroke={1.75} aria-hidden />
      </Button>
    </div>
  );
}
