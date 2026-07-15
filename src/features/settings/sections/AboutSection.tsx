import { Wordmark } from "../../../design";
import { useHealth } from "../../../ipc/hooks";
import { isTauri } from "../../../ipc/client";
import packageJson from "../../../../package.json";
import { SettingRow, SettingsGroup } from "../layout";

export function AboutSection() {
  const health = useHealth();
  const version = health.data?.version ?? packageJson.version;
  const coreName = health.data?.name;

  return (
    <div className="flex flex-col gap-7">
      <div className="rounded-card border border-line bg-surface px-5 py-6 shadow-soft-1">
        <Wordmark className="text-[22px]" />
        <p className="mt-2 max-w-md text-[14px] leading-relaxed text-ink-2">
          Local AI coding agent — your project, your models, your machine.
        </p>
        <p className="mt-3 text-[13px] font-medium text-ink-3">
          Version {version}
          {coreName ? ` · Core ${coreName}` : !isTauri() ? " · browser preview" : ""}
        </p>
      </div>

      <SettingsGroup label="Product">
        <SettingRow title="License" description="Open source under the Apache License 2.0.">
          <span className="text-[13px] font-medium text-ink-2">Apache-2.0</span>
        </SettingRow>
        <SettingRow
          title="Privacy"
          description="Secrets stay in the OS keychain. Diagnostics require an explicit opt-in."
        >
          <span className="text-[13px] text-ink-3">Local-first</span>
        </SettingRow>
        <SettingRow
          title="Package"
          description={packageJson.description ?? "LoopCode"}
        >
          <span className="font-mono text-[12.5px] text-ink-2">{packageJson.name}</span>
        </SettingRow>
      </SettingsGroup>
    </div>
  );
}
