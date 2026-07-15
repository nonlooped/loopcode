import { useState } from "react";
import { IconDatabaseExport, IconRefresh } from "@tabler/icons-react";
import { Button, Chip } from "../../../design";
import { backupDb, updateCheck } from "../../../ipc/commands";
import { useReliabilitySettings } from "../../../ipc/hooks";
import { isTauri } from "../../../ipc/client";
import {
  SettingRow,
  SettingsGroup,
  SettingsSwitch,
  StatusLine,
} from "../layout";

type ActionState = { tone: "neutral" | "ok" | "error"; text: string } | null;

export function DataSection() {
  const reliability = useReliabilitySettings();
  const [updateEnabled, setUpdateEnabled] = useState(false);
  const [backupMsg, setBackupMsg] = useState<ActionState>(null);
  const [updateMsg, setUpdateMsg] = useState<ActionState>(null);
  const [busy, setBusy] = useState<string | null>(null);

  async function runBackup() {
    setBusy("backup");
    setBackupMsg(null);
    try {
      const path = await backupDb();
      setBackupMsg({ tone: "ok", text: `Backup written: ${path}` });
    } catch (e) {
      setBackupMsg({ tone: "error", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  async function runUpdateCheck() {
    setBusy("update");
    setUpdateMsg(null);
    try {
      const result = await updateCheck(updateEnabled);
      setUpdateMsg({ tone: "ok", text: result });
    } catch (e) {
      setUpdateMsg({ tone: "error", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  if (!isTauri()) {
    return (
      <SettingsGroup label="Desktop only">
        <SettingRow
          title="Data tools require the app"
          description="Backup and update checks talk to the local Core. Open the desktop build to use them."
        />
      </SettingsGroup>
    );
  }

  return (
    <div className="flex flex-col gap-7">
      <SettingsGroup
        label="Backup"
        hint="Creates a point-in-time copy of the LoopCode database. Secrets stay in your OS keychain."
      >
        <SettingRow
          title="Database backup"
          description="Write a local copy you can archive or move."
          align="start"
        >
          <Button
            variant="secondary"
            className="gap-1.5 px-3 py-2 text-[13px]"
            disabled={busy === "backup"}
            onClick={() => void runBackup()}
          >
            <IconDatabaseExport size={15} stroke={1.75} aria-hidden />
            {busy === "backup" ? "Backing up…" : "Back up now"}
          </Button>
        </SettingRow>
      </SettingsGroup>
      {backupMsg && <StatusLine tone={backupMsg.tone}>{backupMsg.text}</StatusLine>}

      <SettingsGroup label="Updates">
        <SettingRow
          title="Allow update check"
          description="When off, the check reports that it was skipped."
        >
          <SettingsSwitch
            aria-label="Allow update check"
            checked={updateEnabled}
            onCheckedChange={setUpdateEnabled}
          />
        </SettingRow>
        <SettingRow
          title="Check for updates"
          description="Ask Core whether a newer build is available."
          align="start"
        >
          <Button
            variant="secondary"
            className="gap-1.5 px-3 py-2 text-[13px]"
            disabled={busy === "update"}
            onClick={() => void runUpdateCheck()}
          >
            <IconRefresh size={15} stroke={1.75} aria-hidden />
            {busy === "update" ? "Checking…" : "Check now"}
          </Button>
        </SettingRow>
      </SettingsGroup>
      {updateMsg && <StatusLine tone={updateMsg.tone}>{updateMsg.text}</StatusLine>}

      {(reliability.data?.labels?.length ?? 0) > 0 && (
        <SettingsGroup
          label="Reliability labels"
          hint="These categories surface on run errors so retries stay intentional."
        >
          <div className="flex flex-wrap gap-1.5 px-4 py-3.5">
            {reliability.data!.labels.map((l) => (
              <Chip key={l} quiet>
                {l}
              </Chip>
            ))}
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}
