import { useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { Button, Chip, ProviderMark, TextField, cn } from "../../../design";
import { pressable } from "../../../design/interactive";
import {
	useConnectProvider,
	useHeroes,
	useReadyProvider,
} from "../../../ipc/hooks";
import { isTauri } from "../../../ipc/client";
import type { ConnectResult, HeroCard } from "../../../ipc/types";
import { EmptyState, MonoBadge, SettingsCard, StatusLine } from "../layout";
import { CatalogBrowsePanel } from "./CatalogBrowsePanel";
import { CustomProviderForm } from "./CustomProviderForm";

/** Mask shown when a provider already has a key on file (real secret never leaves Core). */
const SAVED_KEY_MASK = "•".repeat(28);

/** Catalog taglines per first-party provider. */
const PROVIDER_TAGLINES: Record<string, string> = {
	openai: "GPT models, direct from OpenAI.",
	anthropic: "Claude models, direct from Anthropic.",
	openrouter: "One key for hundreds of models across labs.",
	opencode: "Curated coding models via OpenCode Zen.",
};

/** Brand mark in a quiet tile. */
function ProviderTile({ hero, size = 22 }: { hero: HeroCard; size?: number }) {
	return (
		<span
			aria-hidden="true"
			className="flex size-10 shrink-0 items-center justify-center rounded-[10px] border border-line bg-surface-2 text-ink"
		>
			<ProviderMark providerId={hero.id} name={hero.name} size={size} />
		</span>
	);
}

type Status = { tone: "neutral" | "ok" | "error"; text: string };

/** Branchy status derivation kept out of the component so the form stays simple. */
function computeProviderStatus(args: {
	pending: boolean;
	ready: boolean;
	error: unknown;
	result: ConnectResult | undefined;
	canTest: boolean;
	connected: boolean;
}): Status | null {
	const { pending, ready, error, result, canTest, connected } = args;
	if (pending) return null;
	if (ready) {
		return {
			tone: "ok",
			text: `Connected — ${result?.connection.defaultModel ?? "ready"}`,
		};
	}
	if (error) {
		return {
			tone: "error",
			text:
				error instanceof Error ? error.message : "Couldn't reach the provider.",
		};
	}
	if (result && !ready) {
		return {
			tone: "error",
			text: result.probe.error?.message ?? "Not ready — check the key.",
		};
	}
	if (!canTest) {
		return {
			tone: "neutral",
			text: connected
				? "Key on file. Clear and paste a new one to replace it."
				: "Paste an API key, then test the connection.",
		};
	}
	return { tone: "neutral", text: "Test the connection to save." };
}

/** Connect / reconnect form shown in the detail panel under the catalog grid. */
function ProviderConnectForm({
	hero,
	connected,
	onClose,
}: {
	hero: HeroCard;
	connected: boolean;
	onClose: () => void;
}) {
	const connect = useConnectProvider();
	const [apiKey, setApiKey] = useState(connected ? SAVED_KEY_MASK : "");
	const usingSavedKey = connected && apiKey === SAVED_KEY_MASK;
	const canTest = usingSavedKey || apiKey.trim().length > 0;
	const result = connect.data;
	const ready = result?.probe.ready ?? false;

	function onApiKeyChange(value: string) {
		setApiKey(value);
		// Key changed — require a fresh test before it counts as connected.
		if (connect.data || connect.isError) connect.reset();
	}

	function test() {
		if (!canTest) return;
		connect.mutate({
			providerId: hero.id,
			// Empty string tells Core to reuse the stored secret.
			apiKey: usingSavedKey ? "" : apiKey.trim(),
		});
	}

	const status = computeProviderStatus({
		pending: connect.isPending,
		ready,
		error: connect.error,
		result,
		canTest,
		connected,
	});

	return (
		<SettingsCard>
			<div className="flex items-center gap-3 border-b border-line px-4 py-3">
				<ProviderTile hero={hero} size={18} />
				<div className="min-w-0 flex-1">
					<p className="text-[14px] font-medium text-ink">
						{connected ? `Reconnect ${hero.name}` : `Connect ${hero.name}`}
					</p>
					<p className="text-[12.5px] leading-snug text-ink-3">
						Keys stay on this computer. You pay the provider directly.
					</p>
				</div>
				<Button
					variant="ghost"
					className="px-2.5 py-1.5 text-[12.5px]"
					onClick={onClose}
					disabled={connect.isPending}
				>
					Close
				</Button>
			</div>
			<div className="px-4 py-4">
				<TextField
					label={`${hero.name} API key`}
					type="password"
					autoComplete="off"
					placeholder="Paste your API key"
					value={apiKey}
					autoFocus
					onChange={(e) => onApiKeyChange(e.currentTarget.value)}
					onFocus={(e) => {
						// Selecting the mask makes it easy to replace the whole value.
						if (usingSavedKey) e.currentTarget.select();
					}}
					hint={
						connected
							? "Key on file — clear and paste a new one to replace it. An empty test reuses the stored key."
							: "Stored in the OS keychain, never in the database."
					}
				/>
				<div className="mt-3 flex flex-wrap items-center gap-2">
					{ready ? (
						<Button className="px-3 py-2 text-[13px]" onClick={onClose}>
							Done
						</Button>
					) : (
						<Button
							className="px-3 py-2 text-[13px]"
							disabled={!canTest || connect.isPending}
							onClick={test}
						>
							{connect.isPending
								? "Testing…"
								: connected
									? "Test & save"
									: "Test connection"}
						</Button>
					)}
				</div>
				{status && <StatusLine tone={status.tone}>{status.text}</StatusLine>}
			</div>
		</SettingsCard>
	);
}

/** One catalog tile: brand mark, tagline, connection state. Click opens the connect panel. */
function ProviderCard({
	hero,
	active,
	selected,
	onSelect,
}: {
	hero: HeroCard;
	active: boolean;
	selected: boolean;
	onSelect: () => void;
}) {
	const connected = Boolean(hero.hasSavedKey);
	const tagline = PROVIDER_TAGLINES[hero.id] ?? "First-party provider.";

	return (
		<Button
			variant="ghost"
			onClick={onSelect}
			aria-expanded={selected}
			className={cn(
				pressable,
				"flex flex-col items-start gap-3 rounded-card border p-4 text-left",
				"focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
				selected
					? "border-accent bg-surface shadow-[0_0_0_3px_var(--accent-tint)]"
					: "border-line bg-surface shadow-soft-1 fine-hover:border-accent-line",
			)}
		>
			<div className="flex w-full items-start justify-between gap-2">
				<ProviderTile hero={hero} />
				{active ? (
					<Chip tone="accent" dot>
						Active
					</Chip>
				) : connected ? (
					<Chip tone="accent" dot>
						Connected
					</Chip>
				) : (
					<Chip quiet>Not connected</Chip>
				)}
			</div>
			<div className="min-w-0">
				<p className="text-[14px] font-medium text-ink">{hero.name}</p>
				<p className="mt-0.5 text-[12.5px] leading-snug text-ink-3">
					{tagline}
				</p>
			</div>
			{hero.defaultModel && <MonoBadge>{hero.defaultModel}</MonoBadge>}
		</Button>
	);
}

/** Skeleton tiles while the catalog loads (shape matches the real cards). */
function CatalogSkeleton() {
	return (
		<div className="grid gap-3 sm:grid-cols-2" aria-hidden="true">
			{Array.from({ length: 4 }, (_, i) => (
				<div
					key={i}
					className="flex animate-pulse flex-col gap-3 rounded-card border border-line bg-surface p-4"
				>
					<div className="size-10 rounded-[10px] bg-surface-2" />
					<div className="h-3.5 w-24 rounded bg-surface-2" />
					<div className="h-3 w-full rounded bg-surface-2" />
				</div>
			))}
		</div>
	);
}

export function ProvidersSection() {
	const { data: heroes = [], isLoading } = useHeroes();
	const { data: readyProvider } = useReadyProvider();
	const activeId = readyProvider?.providerId ?? null;
	const [selectedId, setSelectedId] = useState<string | null>(null);
	const reduce = useReducedMotion();

	const selected = heroes.find((h) => h.id === selectedId) ?? null;
	const connectedCount = heroes.filter((h) => h.hasSavedKey).length;

	if (!isTauri()) {
		return (
			<SettingsCard>
				<EmptyState
					title="Desktop app required"
					description="Provider keys are stored in the local OS keychain. Run LoopCode as a Tauri app to connect."
				/>
			</SettingsCard>
		);
	}

	return (
		<div className="flex flex-col gap-4">
			{heroes.length > 0 && (
				<p className="px-0.5 text-[12.5px] text-ink-3">
					{connectedCount === 0
						? "Pick a provider to connect your first model."
						: `${connectedCount} of ${heroes.length} connected. Pick the active model from the composer.`}
				</p>
			)}

			{isLoading ? (
				<CatalogSkeleton />
			) : heroes.length === 0 ? (
				<SettingsCard>
					<EmptyState
						title="No providers available"
						description="The bundled provider catalog is empty. Reloading the catalog may restore the first-party set."
					/>
				</SettingsCard>
			) : (
				<div className="grid gap-3 sm:grid-cols-2">
					{heroes.map((h) => (
						<ProviderCard
							key={h.id}
							hero={h}
							active={activeId === h.id}
							selected={selectedId === h.id}
							onSelect={() =>
								setSelectedId((cur) => (cur === h.id ? null : h.id))
							}
						/>
					))}
				</div>
			)}

			<AnimatePresence initial={false} mode="wait">
				{selected && (
					<motion.div
						key={selected.id}
						initial={reduce ? { opacity: 0 } : { opacity: 0, height: 0 }}
						animate={reduce ? { opacity: 1 } : { opacity: 1, height: "auto" }}
						exit={reduce ? { opacity: 0 } : { opacity: 0, height: 0 }}
						transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
						className="overflow-hidden"
					>
						<ProviderConnectForm
							key={selected.id}
							hero={selected}
							connected={Boolean(selected.hasSavedKey)}
							onClose={() => setSelectedId(null)}
						/>
					</motion.div>
				)}
			</AnimatePresence>

			<CatalogBrowsePanel />
			<CustomProviderForm />
		</div>
	);
}
