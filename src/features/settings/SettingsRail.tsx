import { IconArrowLeft } from "@tabler/icons-react";
import { Button, cn, listRow, listRowActive, listRowIdle } from "../../design";
import { useUi } from "../../store/ui";
import { SETTINGS_CATEGORIES } from "./categories";

/**
 * Left sidebar while settings mode is active — category nav with the same
 * width/sizing as the projects rail, but a dedicated layout.
 */
export function SettingsRail() {
  const category = useUi((s) => s.settingsCategory);
  const setCategory = useUi((s) => s.setSettingsCategory);
  const setSettingsOpen = useUi((s) => s.setSettingsOpen);

  return (
    <aside
      className="flex h-full min-h-0 flex-col border-r border-line bg-surface-2"
      aria-label="Settings categories"
    >
      <div className="flex shrink-0 items-center border-b border-line px-3 py-2.5">
        <h2 className="text-[13px] font-semibold text-ink-2">
          Settings
        </h2>
      </div>

      <nav className="min-h-0 flex-1 overflow-y-auto px-1.5 py-1.5">
        <ul className="flex flex-col gap-px" role="list">
          {SETTINGS_CATEGORIES.map((item) => {
            const Icon = item.icon;
            const active = item.id === category;
            return (
              <li key={item.id}>
                <Button
                  variant="ghost"
                  onClick={() => setCategory(item.id)}
                  aria-current={active ? "page" : undefined}
                  className={cn(
                    listRow,
                    "flex items-center gap-2.5 py-1.5 pr-2 pl-2 text-[13px]",
                    active ? listRowActive : listRowIdle,
                  )}
                >
                  <Icon
                    size={16}
                    stroke={1.75}
                    className={cn("shrink-0", active ? "text-accent" : "text-ink-3")}
                    aria-hidden
                  />
                  <span className="min-w-0 flex-1 truncate text-left">{item.label}</span>
                </Button>
              </li>
            );
          })}
        </ul>
      </nav>

      <div className="shrink-0 border-t border-line px-2 py-2">
        <Button
          variant="ghost"
          className="w-full justify-start gap-2.5 px-2.5"
          onClick={() => setSettingsOpen(false)}
        >
          <IconArrowLeft size={16} stroke={1.75} aria-hidden />
          Back
        </Button>
      </div>
    </aside>
  );
}
