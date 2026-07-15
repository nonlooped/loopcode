import { useState } from "react";
import { IconPlus, IconTrash } from "@tabler/icons-react";
import { Button, Chip, TextField, SelectField } from "../../../design";
import {
  useGrantMcpTrust,
  useMcp,
  useRegisterMcp,
  useRemoveMcp,
  useSetMcpEnabled,
} from "../../../ipc/hooks";
import { isTauri } from "../../../ipc/client";
import type { McpServer } from "../../../ipc/types";
import { useSession } from "../../../store/session";
import {
  EmptyState,
  MonoBadge,
  SectionLabel,
  SettingRow,
  SettingsCard,
  SettingsGroup,
  SettingsSwitch,
  StatusLine,
} from "../layout";

type TransportKind = "stdio" | "http";

function transportSummary(server: McpServer): string {
  const t = server.transport;
  if (t.type === "stdio") {
    const args = t.args?.length ? ` ${t.args.join(" ")}` : "";
    return `${t.command}${args}`;
  }
  return t.url;
}

function McpServerRow({ server }: { server: McpServer }) {
  const projectId = useSession((s) => s.activeProjectId);
  const setEnabled = useSetMcpEnabled();
  const remove = useRemoveMcp();
  const grant = useGrantMcpTrust();
  const [err, setErr] = useState<string | null>(null);

  return (
    <div className="border-b border-line last:border-b-0">
      <div className="flex items-start gap-3 px-4 py-3.5">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[14px] font-medium text-ink">{server.name}</span>
            <Chip tone={server.enabled ? "accent" : "neutral"}>
              {server.enabled ? "enabled" : "disabled"}
            </Chip>
            <MonoBadge>{server.transport.type}</MonoBadge>
          </div>
          <p className="mt-1 font-mono text-[12px] leading-snug break-all text-ink-3">
            {transportSummary(server)}
          </p>
        </div>
        <SettingsSwitch
          aria-label={`${server.enabled ? "Disable" : "Enable"} ${server.name}`}
          checked={server.enabled}
          disabled={setEnabled.isPending}
          onCheckedChange={(next) => {
            setErr(null);
            setEnabled.mutate(
              { serverId: server.id, enabled: next },
              { onError: (e) => setErr(String(e)) },
            );
          }}
        />
      </div>
      <div className="flex flex-wrap items-center gap-2 border-t border-line/80 bg-surface-2/30 px-4 py-2.5">
        <Button
          variant="secondary"
          className="px-2.5 py-1.5 text-[12.5px]"
          disabled={grant.isPending}
          onClick={() => {
            setErr(null);
            grant.mutate(
              { serverId: server.id, projectId },
              { onError: (e) => setErr(String(e)) },
            );
          }}
        >
          Grant trust
        </Button>
        <Button
          variant="ghost"
          className="gap-1 px-2.5 py-1.5 text-[12.5px] text-clay fine-hover:bg-clay-tint fine-hover:text-clay"
          disabled={remove.isPending}
          onClick={() => {
            if (!confirm(`Remove MCP server “${server.name}”?`)) return;
            setErr(null);
            remove.mutate(server.id, { onError: (e) => setErr(String(e)) });
          }}
        >
          <IconTrash size={14} stroke={1.75} aria-hidden />
          Remove
        </Button>
        <span className="ml-auto text-[11.5px] text-ink-3">
          Trust + enable are separate gates
        </span>
      </div>
      {err && (
        <div className="px-4 pb-2.5">
          <StatusLine tone="error">{err}</StatusLine>
        </div>
      )}
    </div>
  );
}

function AddMcpForm({ onDone }: { onDone: () => void }) {
  const register = useRegisterMcp();
  const projectId = useSession((s) => s.activeProjectId);
  const [name, setName] = useState("");
  const [transportType, setTransportType] = useState<TransportKind>("stdio");
  const [command, setCommand] = useState("npx");
  const [argsText, setArgsText] = useState("-y @modelcontextprotocol/server-everything");
  const [url, setUrl] = useState("http://127.0.0.1:3000/mcp");
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    const trimmed = name.trim();
    if (!trimmed) {
      setError("Name is required.");
      return;
    }
    try {
      if (transportType === "stdio") {
        const args = argsText
          .trim()
          .split(/\s+/)
          .filter(Boolean);
        if (!command.trim()) {
          setError("Command is required for stdio transport.");
          return;
        }
        await register.mutateAsync({
          name: trimmed,
          transportType: "stdio",
          command: command.trim(),
          args,
          envKeys: [],
          projectId,
        });
      } else {
        if (!url.trim()) {
          setError("URL is required for HTTP transport.");
          return;
        }
        await register.mutateAsync({
          name: trimmed,
          transportType: "http",
          url: url.trim(),
          headerKeys: [],
          projectId,
        });
      }
      onDone();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-3 border-t border-line px-4 py-4">
      <TextField
        label="Display name"
        value={name}
        onChange={(e) => setName(e.currentTarget.value)}
        placeholder="Filesystem tools"
        autoFocus
      />
      <div className="flex flex-col gap-1.5">
        <span className="text-[13px] font-semibold tracking-wide text-ink-2">Transport</span>
        <SelectField
          aria-label="Transport"
          value={transportType}
          onValueChange={setTransportType}
          items={[
            { value: "stdio", label: "stdio (local command)" },
            { value: "http", label: "HTTP / SSE (remote)" },
          ]}
        />
      </div>
      {transportType === "stdio" ? (
        <>
          <TextField
            label="Command"
            value={command}
            onChange={(e) => setCommand(e.currentTarget.value)}
            placeholder="npx"
            hint="Executable only — never put secrets in the command string."
          />
          <TextField
            label="Arguments"
            value={argsText}
            onChange={(e) => setArgsText(e.currentTarget.value)}
            placeholder="-y package-name"
            spellCheck={false}
            hint="Space-separated. Env keys are configured later."
          />
        </>
      ) : (
        <TextField
          label="URL"
          value={url}
          onChange={(e) => setUrl(e.currentTarget.value)}
          placeholder="https://mcp.example/sse"
          hint="Header values are never stored in the fingerprint surface."
        />
      )}
      {error && <StatusLine tone="error">{error}</StatusLine>}
      <div className="flex flex-wrap gap-2 pt-1">
        <Button type="submit" disabled={register.isPending} className="px-3 py-2 text-[13px]">
          {register.isPending ? "Adding…" : "Add server"}
        </Button>
        <Button
          type="button"
          variant="ghost"
          className="px-3 py-2 text-[13px]"
          onClick={onDone}
        >
          Cancel
        </Button>
      </div>
    </form>
  );
}

export function McpSection() {
  const mcp = useMcp();
  const list = mcp.data ?? [];
  const [adding, setAdding] = useState(false);

  return (
    <div className="flex flex-col gap-7">
      <div>
        <div className="mb-2 flex items-center justify-between gap-3 px-0.5">
          <SectionLabel>Servers</SectionLabel>
          {!adding && (
            <Button
              variant="secondary"
              className="gap-1.5 px-2.5 py-1.5 text-[12.5px]"
              disabled={!isTauri()}
              onClick={() => setAdding(true)}
            >
              <IconPlus size={15} stroke={1.75} aria-hidden />
              Add server
            </Button>
          )}
        </div>
        <SettingsCard>
          {!isTauri() ? (
            <EmptyState
              title="Desktop app required"
              description="MCP servers are stored in the local database. Run LoopCode as a Tauri app to manage them."
            />
          ) : mcp.isLoading ? (
            <p className="px-4 py-6 text-[13.5px] text-ink-3">Loading servers…</p>
          ) : list.length === 0 && !adding ? (
            <EmptyState
              title="No MCP servers yet"
              description="Register a stdio or HTTP server. Servers start disabled — enable them when you want progressive disclosure, and grant trust before tools can run."
              action={
                <Button
                  className="gap-1.5 px-3 py-2 text-[13px]"
                  onClick={() => setAdding(true)}
                >
                  <IconPlus size={15} stroke={1.75} aria-hidden />
                  Add your first server
                </Button>
              }
            />
          ) : (
            list.map((s) => <McpServerRow key={s.id} server={s} />)
          )}
          {adding && <AddMcpForm onDone={() => setAdding(false)} />}
        </SettingsCard>
        <p className="mt-2 px-0.5 text-[12.5px] leading-snug text-ink-3">
          Dual gate: server trust + per-tool approval. Config changes reset trust via fingerprint.
        </p>
      </div>

      <SettingsGroup label="How it works">
        <SettingRow
          title="1. Register"
          description="Store command or URL. Nothing launches at registration."
        />
        <SettingRow
          title="2. Grant trust"
          description="Approve this server config for the current fingerprint."
        />
        <SettingRow
          title="3. Enable"
          description="Opt into progressive disclosure. Tools still need per-call approval."
        />
      </SettingsGroup>
    </div>
  );
}
