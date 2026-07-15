import { useState } from "react";
import { IconChevronRight } from "@tabler/icons-react";
import { Button, cn } from "../../../design";
import { pressable } from "../../../design/interactive";
import { useCatalog, useHeroes } from "../../../ipc/hooks";
import { EmptyState, MonoBadge, SettingsCard } from "../layout";

/** Advisory catalog only: connecting is limited to providers with Core adapters. */
export function CatalogBrowsePanel() {
  const [open, setOpen] = useState(false);
  const { data: catalog = [], isLoading } = useCatalog();
  const { data: heroes = [] } = useHeroes();
  const heroIds = new Set(heroes.map((hero) => hero.id));
  const entries = catalog.filter((provider) => !heroIds.has(provider.id));

  return (
    <SettingsCard>
      <Button
        variant="ghost"
        onClick={() => setOpen((value) => !value)}
        className={cn(pressable, "flex w-full items-center justify-between gap-3 px-4 py-3 text-left")}
        aria-expanded={open}
      >
        <div>
          <p className="text-[14px] font-medium text-ink">Browse catalog</p>
          <p className="text-[13px] text-ink-3">Bundled models.dev reference data</p>
        </div>
        <IconChevronRight
          size={16}
          stroke={2}
          className={cn("shrink-0 text-ink-3 transition-transform duration-[var(--duration-fast)]", open && "rotate-90")}
          aria-hidden
        />
      </Button>
      {open && (
        <div className="border-t border-line px-4 py-4">
          <p className="mb-3 text-[13px] leading-relaxed text-ink-2">
            This catalog is informational. Connect providers from the supported provider cards above; Core must own a provider adapter before LoopCode can safely test or use a credential.
          </p>
          {isLoading ? (
            <p className="text-[13px] text-ink-3">Loading catalog…</p>
          ) : entries.length === 0 ? (
            <EmptyState title="No extra catalog entries" description="First-party provider cards cover the available connections." />
          ) : (
            <ul className="max-h-56 divide-y divide-line overflow-auto rounded-control border border-line">
              {entries.map((provider) => (
                <li key={provider.id} className="flex items-center justify-between gap-2 px-3 py-2.5">
                  <span className="min-w-0">
                    <span className="block truncate text-[13px] font-medium text-ink">{provider.name}</span>
                    <span className="block truncate text-[13px] text-ink-3">{provider.id}</span>
                  </span>
                  {provider.defaultModel && <MonoBadge>{provider.defaultModel}</MonoBadge>}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </SettingsCard>
  );
}
