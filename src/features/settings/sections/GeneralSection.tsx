import { IconMoon, IconSun } from "@tabler/icons-react";
import { Button, cn } from "../../../design";
import { useUi, type Theme } from "../../../store/ui";
import { SettingRow, SettingsGroup } from "../layout";

const THEMES: {
  id: Theme;
  label: string;
  description: string;
  icon: typeof IconSun;
}[] = [
  {
    id: "light",
    label: "Light",
    description: "Bright surfaces for daytime work",
    icon: IconSun,
  },
  {
    id: "dark",
    label: "Dark",
    description: "Low glare for long sessions",
    icon: IconMoon,
  },
];

export function GeneralSection() {
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);

  return (
    <div className="flex flex-col gap-7">
      <SettingsGroup
        label="Appearance"
        hint="Theme applies instantly across the whole app — no reload."
      >
        <SettingRow
          title="Color theme"
          description="Pick the look that matches your environment."
          align="start"
        >
          <div
            role="radiogroup"
            aria-label="Color theme"
            className="inline-flex rounded-full border border-line-2 bg-surface-2 p-0.5"
          >
            {THEMES.map((t) => {
              const Icon = t.icon;
              const active = theme === t.id;
              return (
                <Button
                  key={t.id}
                  variant="ghost"
                  role="radio"
                  aria-checked={active}
                  title={t.description}
                  onClick={() => setTheme(t.id)}
                  className={cn(
                    "inline-flex h-8 items-center gap-1.5 rounded-full px-3 text-[12.5px] font-semibold outline-none",
                    "transition-[background-color,color,box-shadow,transform] duration-[var(--duration-press)] ease-[var(--ease-out)]",
                    "active:scale-[0.97]",
                    "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
                    active
                      ? "bg-surface text-ink shadow-soft-1"
                      : "text-ink-3 fine-hover:text-ink",
                  )}
                >
                  <Icon
                    size={14}
                    stroke={1.75}
                    className={active ? (t.id === "dark" ? "text-accent" : "text-amber") : undefined}
                    aria-hidden
                  />
                  {t.label}
                </Button>
              );
            })}
          </div>
        </SettingRow>
      </SettingsGroup>

      <SettingsGroup
        label="Layout"
        hint="Sidebar widths are drag-resized in the main app. Collapse with Ctrl/⌘ B."
      >
        <SettingRow
          title="Left sidebar"
          description="Projects and sessions — or settings categories when you open Settings."
        >
          <span className="text-[13px] text-ink-3">Resizable</span>
        </SettingRow>
        <SettingRow
          title="Right sidebar"
          description="Files and diffs. Hidden automatically while Settings is open."
        >
          <span className="text-[13px] text-ink-3">Resizable</span>
        </SettingRow>
      </SettingsGroup>
    </div>
  );
}
