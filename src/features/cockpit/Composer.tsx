import {
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
	type ReactNode,
} from "react";
import {
	IconArrowUp,
	IconBug,
	IconHammer,
	IconListCheck,
	IconMessageCircle,
	IconPlayerStop,
	IconX,
} from "@tabler/icons-react";
import {
	ContextMenu,
	Button,
	ProviderMark,
	ReasoningSlider,
	SelectField,
	cn,
	pressable,
} from "../../design";
import { copyText } from "../../lib/clipboard";
import { clipboardReadText } from "../../ipc/commands";
import { ApprovalNotice } from "./ApprovalNotice";
import { ChangesPill } from "./ChangesPill";
import {
	useCancelRun,
	useCatalog,
	useCreateChat,
	useHeroes,
	useReadyProvider,
	useSend,
	useSetProviderSelection,
} from "../../ipc/hooks";
import { isTauri } from "../../ipc/client";
import { useSession } from "../../store/session";
import type {
	CatalogModel,
	CatalogProvider,
	Mode,
	ReasoningConfig,
	TimelineItem,
} from "../../ipc/types";

const MODE_META: Record<
	Mode,
	{ label: string; icon: typeof IconHammer; description: string }
> = {
	ask: {
		label: "Ask",
		icon: IconMessageCircle,
		description: "Answer questions — read-only, no changes",
	},
	plan: {
		label: "Plan",
		icon: IconListCheck,
		description: "Plan a task — outlines steps before building",
	},
	build: {
		label: "Build",
		icon: IconHammer,
		description: "Make changes — writes files and runs commands",
	},
	debug: {
		label: "Debug",
		icon: IconBug,
		description: "Investigate errors — diagnose and propose fixes",
	},
};

function modeLabel(id: Mode): ReactNode {
	const { label, icon: Icon } = MODE_META[id];
	return (
		<span className="inline-flex items-center gap-1.5">
			<Icon
				size={14}
				stroke={1.75}
				className="shrink-0 opacity-80"
				aria-hidden
			/>
			{label}
		</span>
	);
}

const MODE_ITEMS = (Object.keys(MODE_META) as Mode[]).map((id) => ({
	value: id,
	label: modeLabel(id),
	textValue: MODE_META[id].label,
	description: MODE_META[id].description,
}));

/** Display metadata for reasoning choices ("off"/"on" + models.dev efforts). */
const REASONING_META: Record<string, { label: string; description: string }> = {
	off: { label: "Off", description: "Answer directly, no thinking" },
	on: { label: "On", description: "Think before answering" },
	minimal: { label: "Minimal", description: "Barely thinks — fastest" },
	low: { label: "Low", description: "Quick thinking for simple tasks" },
	medium: { label: "Medium", description: "Balanced depth and speed" },
	high: { label: "High", description: "Thorough thinking for hard tasks" },
	xhigh: { label: "Extra high", description: "Very deep thinking — slower" },
	max: { label: "Max", description: "Deepest thinking the model allows" },
};

/**
 * Choices the composer offers for a model's reasoning, in menu order.
 * Empty means the control is hidden (no reasoning, or always-on with no knob).
 */
function reasoningChoices(model: CatalogModel | undefined): string[] {
	if (!model?.reasoning) return [];
	const efforts = (model.reasoningEfforts ?? []).filter(
		(e) => e !== "none" && REASONING_META[e],
	);
	const canDisable =
		Boolean(model.reasoningToggle) ||
		(model.reasoningEfforts ?? []).includes("none");
	if (efforts.length > 0) {
		return canDisable ? ["off", ...efforts] : efforts;
	}
	return canDisable ? ["off", "on"] : [];
}

/** Model default when switching models: medium when offered, else midpoint. */
function defaultReasoning(choices: string[]): string | null {
	if (choices.length === 0) return null;
	const efforts = choices.filter((c) => c !== "off");
	if (efforts.length === 0) return "off";
	if (efforts.includes("medium")) return "medium";
	return efforts[Math.floor((efforts.length - 1) / 2)];
}

/** Map the composer selection to the Core run payload. */
function reasoningPayload(selection: string | null): ReasoningConfig | null {
	if (!selection) return null;
	if (selection === "off") return { enabled: false };
	if (selection === "on") return { enabled: true };
	return { enabled: true, effort: selection };
}

const TEXTAREA_MIN = 52;
const TEXTAREA_MAX = 200;

/** Cancel hint under the composer while a run is in flight. */
function runStatusLabel(status?: string | null): string {
	switch (status) {
		case "approval_waiting":
			return "Waiting for your approval above";
		case "cancelling":
			return "Stopping…";
		default:
			// Thinking lives in the timeline; keep this line cancel-only.
			return "Press Stop or Esc to cancel";
	}
}

/** Default model for a catalog provider: explicit default → flagged model → first. */
function defaultModelId(provider: CatalogProvider | undefined): string | null {
	if (!provider) return null;
	if (
		provider.defaultModel &&
		provider.models.some((m) => m.id === provider.defaultModel)
	) {
		return provider.defaultModel;
	}
	const flagged = provider.models.find((m) => m.default);
	if (flagged) return flagged.id;
	return provider.models[0]?.id ?? null;
}

function resolveModel(
	provider: CatalogProvider | undefined,
	preferred: string | null | undefined,
): string | null {
	if (!provider) return preferred ?? null;
	if (preferred && provider.models.some((m) => m.id === preferred))
		return preferred;
	return defaultModelId(provider);
}

export function Composer({
	runStatus,
	pendingApproval = null,
}: {
	runStatus?: string | null;
	pendingApproval?: TimelineItem | null;
}) {
	const {
		activeProjectId,
		activeChatId,
		activeRunId,
		mode,
		providerId,
		model,
		reasoning,
	} = useSession();
	const setReasoning = useSession((s) => s.setReasoning);
	const setActiveChat = useSession((s) => s.setActiveChat);
	const setActiveRun = useSession((s) => s.setActiveRun);
	const setMode = useSession((s) => s.setMode);
	const setModel = useSession((s) => s.setModel);
	const setProviderAndModel = useSession((s) => s.setProviderAndModel);

	const { data: heroes = [], isSuccess: heroesReady } = useHeroes();
	const { data: catalog = [], isSuccess: catalogReady } = useCatalog();
	const { data: readyProvider, isSuccess: readyReady } = useReadyProvider();
	const persistSelection = useSetProviderSelection();

	const send = useSend();
	const cancel = useCancelRun();
	const createChat = useCreateChat();
	const [value, setValue] = useState("");
	const textareaRef = useRef<HTMLTextAreaElement>(null);
	const [showModeHint, setShowModeHint] = useState(false);

	useEffect(() => {
		if (typeof localStorage === "undefined") return;
		setShowModeHint(localStorage.getItem("loopcode:modeHintSeen") !== "1");
	}, []);

	function dismissModeHint() {
		setShowModeHint(false);
		try {
			localStorage.setItem("loopcode:modeHintSeen", "1");
		} catch {
			/* ignore */
		}
	}

	const running = Boolean(activeRunId);
	const canSend = Boolean(value.trim()) && !send.isPending && !running;

	const authenticatedIds = useMemo(
		() => new Set(heroes.filter((h) => h.hasSavedKey).map((h) => h.id)),
		[heroes],
	);

	const readyId =
		readyProvider && typeof readyProvider.providerId === "string"
			? readyProvider.providerId
			: null;
	const readyModel =
		readyProvider && typeof readyProvider.model === "string"
			? readyProvider.model
			: null;

	/** Update session + durable `provider.ready` so restarts keep the same pair. */
	const applyProviderAndModel = useCallback(
		(nextProvider: string | null, nextModel: string | null) => {
			setProviderAndModel(nextProvider, nextModel);
			if (!isTauri() || !nextProvider || !nextModel) return;
			if (!authenticatedIds.has(nextProvider)) return;
			if (nextProvider === readyId && nextModel === readyModel) return;
			persistSelection.mutate({ providerId: nextProvider, model: nextModel });
		},
		[
			setProviderAndModel,
			persistSelection,
			authenticatedIds,
			readyId,
			readyModel,
		],
	);

	const applyModel = useCallback(
		(nextModel: string) => {
			setModel(nextModel);
			if (!isTauri() || !providerId) return;
			if (!authenticatedIds.has(providerId)) return;
			if (providerId === readyId && nextModel === readyModel) return;
			persistSelection.mutate({ providerId, model: nextModel });
		},
		[
			setModel,
			providerId,
			persistSelection,
			authenticatedIds,
			readyId,
			readyModel,
		],
	);

	/**
	 * Keep provider/model aligned with auth + catalog:
	 * 1) Prefer an authenticated provider (never stick on a greyed-out one).
	 * 2) Prefer Core's ready provider when it is authenticated.
	 * 3) Snap model to a valid catalog id for the active provider.
	 */
	useEffect(() => {
		if (!heroesReady || !catalogReady || !readyReady) return;
		if (heroes.length === 0) return;

		const isAuth = (id: string | null | undefined) =>
			Boolean(id && authenticatedIds.has(id));

		const preferredProvider =
			(isAuth(providerId) ? providerId : null) ??
			(isAuth(readyId) ? readyId : null) ??
			[...authenticatedIds][0] ??
			// No keys yet — stay on current selection or first hero (browse-only).
			(providerId && heroes.some((h) => h.id === providerId)
				? providerId
				: null) ??
			readyId ??
			heroes[0]?.id ??
			null;

		if (!preferredProvider) return;

		const entry = catalog.find((p) => p.id === preferredProvider);
		// Prefer session model when already on this provider; otherwise restore
		// persisted ready model (survives restarts) before falling back to catalog default.
		const preferredModel = resolveModel(
			entry,
			preferredProvider === providerId
				? model
				: preferredProvider === readyId
					? readyModel
					: null,
		);

		if (preferredProvider !== providerId || preferredModel !== model) {
			applyProviderAndModel(preferredProvider, preferredModel);
		}
	}, [
		heroes,
		catalog,
		readyId,
		readyModel,
		heroesReady,
		catalogReady,
		readyReady,
		authenticatedIds,
		providerId,
		model,
		applyProviderAndModel,
	]);

	const providerItems = useMemo(() => {
		// Authenticated first so the active provider is easy to find.
		const ordered = [...heroes].sort((a, b) => {
			const ac = a.hasSavedKey ? 0 : 1;
			const bc = b.hasSavedKey ? 0 : 1;
			if (ac !== bc) return ac - bc;
			return a.name.localeCompare(b.name);
		});
		return ordered.map((h) => {
			const connected = Boolean(h.hasSavedKey);
			return {
				value: h.id,
				textValue: connected ? h.name : `${h.name} not connected`,
				disabled: !connected,
				label: (
					<span
						className={cn(
							"inline-flex min-w-0 items-center gap-2",
							!connected && "text-ink-3",
						)}
					>
						<ProviderMark
							providerId={h.id}
							name={h.name}
							size={14}
							className={cn(connected ? "text-ink" : "text-ink-3")}
						/>
						<span className="truncate">{h.name}</span>
					</span>
				),
			};
		});
	}, [heroes]);

	const catalogEntry = catalog.find((p) => p.id === providerId);

	const modelItems = useMemo(() => {
		const models = catalogEntry?.models ?? [];
		if (models.length === 0) {
			return [
				{
					value: "__none__",
					textValue: "No models",
					disabled: true as boolean,
					label: <span className="text-ink-3">No models</span>,
				},
			];
		}

		// Surface the catalog default first; keep newest-first order for the rest.
		const defaultId = defaultModelId(catalogEntry);
		const sorted = [...models].sort((a, b) => {
			if (a.id === defaultId) return -1;
			if (b.id === defaultId) return 1;
			return 0;
		});

		return sorted.map((m) => {
			const name = m.name?.trim() || m.id;
			return {
				value: m.id,
				textValue: name,
				disabled: false as boolean,
				label: <span className="truncate">{name}</span>,
			};
		});
	}, [catalogEntry]);

	const providerConnected = Boolean(
		providerId && authenticatedIds.has(providerId),
	);

	const modelValue =
		model && catalogEntry?.models.some((m) => m.id === model)
			? model
			: (modelItems.find((m) => !m.disabled)?.value ??
				modelItems[0]?.value ??
				"__none__");

	const selectedModel = catalogEntry?.models.find((m) => m.id === modelValue);
	const thinkingChoices = useMemo(
		() => reasoningChoices(selectedModel),
		[selectedModel],
	);

	// Snap the reasoning selection when the model changes: keep it when the new
	// model offers the same choice, otherwise fall back to the model default.
	useEffect(() => {
		const next =
			reasoning && thinkingChoices.includes(reasoning)
				? reasoning
				: defaultReasoning(thinkingChoices);
		if (next !== reasoning) setReasoning(next);
	}, [thinkingChoices, reasoning, setReasoning]);

	const thinkingSliderItems = useMemo(
		() =>
			thinkingChoices.map((c) => ({
				value: c,
				label: REASONING_META[c]?.label ?? c,
				description: REASONING_META[c]?.description ?? "",
			})),
		[thinkingChoices],
	);

	const modelTriggerLabel = useMemo(() => {
		const m = catalogEntry?.models.find((x) => x.id === modelValue);
		return m?.name ?? modelValue;
	}, [catalogEntry, modelValue]);

	function handleProviderChange(nextId: string) {
		if (!authenticatedIds.has(nextId)) return;
		const entry = catalog.find((p) => p.id === nextId);
		const hero = heroes.find((h) => h.id === nextId);
		// Prefer last saved model for this provider (Core), then catalog default.
		const preferred = resolveModel(entry, hero?.defaultModel ?? null);
		applyProviderAndModel(nextId, preferred);
	}

	const resizeTextarea = useCallback(() => {
		const el = textareaRef.current;
		if (!el) return;
		el.style.height = "0px";
		const next = Math.min(
			TEXTAREA_MAX,
			Math.max(TEXTAREA_MIN, el.scrollHeight),
		);
		el.style.height = `${next}px`;
	}, []);

	useEffect(() => {
		resizeTextarea();
	}, [value, resizeTextarea]);

	async function handleSend() {
		const text = value.trim();
		if (!text || send.isPending || running) return;
		let chatId = activeChatId;
		if (!chatId && activeProjectId) {
			const chat = await createChat.mutateAsync({ projectId: activeProjectId });
			chatId = chat.id;
			setActiveChat(chatId);
		}
		if (!chatId) return;
		const run = await send.mutateAsync({
			chatId,
			text,
			mode,
			model,
			reasoning: reasoningPayload(reasoning),
		});
		setActiveRun(run.id);
		setValue("");
	}

	function handleStop() {
		if (activeRunId)
			cancel.mutate({ runId: activeRunId, chatId: activeChatId });
	}

	function selectedComposerText(): string {
		const input = textareaRef.current;
		if (!input) return "";
		return input.value.slice(input.selectionStart, input.selectionEnd);
	}

	function copyComposerSelection() {
		const text = selectedComposerText();
		if (text) void copyText(text);
	}

	function cutComposerSelection() {
		const input = textareaRef.current;
		const text = selectedComposerText();
		if (!input || !text) return;
		void copyText(text);
		setValue(
			(current) =>
				current.slice(0, input.selectionStart) +
				current.slice(input.selectionEnd),
		);
	}

	async function pasteIntoComposer() {
		const input = textareaRef.current;
		if (!input || !isTauri()) return;
		let pasted: string;
		try {
			pasted = await clipboardReadText();
		} catch {
			return;
		}
		const start = input.selectionStart;
		const end = input.selectionEnd;
		setValue(
			(current) => current.slice(0, start) + pasted + current.slice(end),
		);
		requestAnimationFrame(() => {
			input.focus();
			input.setSelectionRange(start + pasted.length, start + pasted.length);
		});
	}

	const providerSelectValue =
		providerId && heroes.some((h) => h.id === providerId)
			? providerId
			: (heroes.find((h) => h.hasSavedKey)?.id ?? heroes[0]?.id ?? "");

	const dataPending = !heroesReady || !catalogReady;

	return (
		<div className="relative pb-3 pt-1">
			{/* Soft scroll-edge fade — hierarchy without a hard divider (Apple materials). */}
			<div
				className="pointer-events-none absolute inset-x-0 -top-8 h-8 bg-gradient-to-b from-transparent to-paper"
				aria-hidden
			/>

			<div className="mx-auto w-full max-w-[52.8rem] px-5">
				<ChangesPill />

				{pendingApproval && <ApprovalNotice item={pendingApproval} />}

				<div
					className={cn(
						"rounded-panel border border-line bg-surface",
						"transition-[border-color] duration-[var(--duration-fast)] ease-[ease]",
						"focus-within:border-accent",
					)}
				>
				<ContextMenu
					items={[
						{
							label: "Cut",
							onSelect: cutComposerSelection,
							disabled: !selectedComposerText(),
						},
						{
							label: "Copy",
							onSelect: copyComposerSelection,
							disabled: !selectedComposerText(),
						},
						{ label: "Paste", onSelect: () => void pasteIntoComposer() },
						{
							label: "Paste as plain text",
							onSelect: () => void pasteIntoComposer(),
						},
						"separator",
						{
							label: "Select all",
							onSelect: () => textareaRef.current?.select(),
						},
					]}
				>
					<textarea
						ref={textareaRef}
						id="composer-input"
						rows={1}
						value={value}
						onChange={(e) => setValue(e.currentTarget.value)}
						onKeyDown={(e) => {
							if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
								e.preventDefault();
								void handleSend();
							}
						}}
						placeholder="Ask a question about this project, or describe what you want to build…"
						aria-label="Message composer"
						className={cn(
							"composer-input block w-full resize-none bg-transparent px-3.5 pt-3.5 pb-2",
							"text-[15px] leading-relaxed text-ink placeholder:text-ink-3",
							// Focus chrome lives on the card (focus-within), not the field.
							"outline-none ring-0 shadow-none",
							"focus:outline-none focus:ring-0 focus:shadow-none",
							"focus-visible:outline-none focus-visible:ring-0 focus-visible:shadow-none",
						)}
						style={{ height: TEXTAREA_MIN, maxHeight: TEXTAREA_MAX }}
					/>
				</ContextMenu>

				<div className="flex flex-wrap items-center gap-0.5 px-2 pb-2 pt-0.5">
					<SelectField
						value={mode}
						onValueChange={(v) => setMode(v)}
						items={MODE_ITEMS}
						aria-label="Agent mode"
						variant="ghost"
					/>

					<span className="mx-0.5 h-3.5 w-px shrink-0 bg-line" aria-hidden />

					{providerSelectValue ? (
						<SelectField
							value={providerSelectValue}
							onValueChange={handleProviderChange}
							items={providerItems}
							aria-label="Provider"
							variant="ghost"
							disabled={dataPending}
							popupClassName="min-w-[14rem]"
						/>
					) : (
						<span className="px-2 text-[13px] text-ink-3">
							{dataPending ? "Loading providers…" : "No providers"}
						</span>
					)}

					<span className="mx-0.5 h-3.5 w-px shrink-0 bg-line" aria-hidden />

					<SelectField
						value={modelValue}
						onValueChange={(v) => {
							if (v === "__none__") return;
							applyModel(v);
						}}
						items={modelItems}
						aria-label="Model"
						variant="ghost"
						disabled={dataPending || !catalogEntry?.models.length}
						searchable
						popupClassName="min-w-[18rem]"
						triggerClassName="max-w-[14rem] truncate"
					/>
					{/* Keep accessible name in trigger when rich labels are long */}
					<span className="sr-only">{modelTriggerLabel}</span>

					{thinkingSliderItems.length > 0 && reasoning && (
						<>
							<span
								className="mx-0.5 h-3.5 w-px shrink-0 bg-line"
								aria-hidden
							/>
							<ReasoningSlider
								value={reasoning}
								onValueChange={(v) => setReasoning(v)}
								items={thinkingSliderItems}
								aria-label="Thinking effort"
								disabled={dataPending}
							/>
						</>
					)}

					<div className="min-w-[0.5rem] flex-1" />

					{running ? (
						<Button
							variant="ghost"
							onClick={handleStop}
							aria-label="Stop run"
							title="Stop"
							className={cn(
								pressable,
								"flex h-8 w-8 shrink-0 items-center justify-center rounded-full",
								"bg-clay-tint text-clay",
								"focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
							)}
						>
							<IconPlayerStop size={15} stroke={2} aria-hidden />
						</Button>
					) : (
						<Button
							variant="ghost"
							onClick={() => void handleSend()}
							disabled={!canSend || !providerConnected}
							aria-label={send.isPending ? "Sending" : "Send message"}
							title={
								!providerConnected
									? "Connect a provider first"
									: "Send (⌘/Ctrl + Enter)"
							}
							className={cn(
								pressable,
								"flex h-8 w-8 shrink-0 items-center justify-center rounded-full",
								"focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
								canSend && providerConnected
									? "bg-accent text-accent-foreground fine-hover:bg-accent-strong"
									: "bg-surface-2 text-ink-3 disabled:opacity-100",
							)}
						>
							<IconArrowUp size={16} stroke={2.25} aria-hidden />
						</Button>
					)}
				</div>
				</div>

			{showModeHint && providerConnected && !running && (
				<div className="mt-1.5 flex items-start gap-2 rounded-card border border-line bg-surface-2 px-3 py-2">
					<p className="flex-1 text-[12px] leading-snug text-ink-2">
						<span className="font-semibold text-ink">Modes:</span>{" "}
						<span className="text-ink-2">
							Ask (read-only), Plan (outline before building), Build (make
							changes), Debug (investigate errors). Switch anytime above.
						</span>
					</p>
					<Button
						variant="ghost"
						onClick={dismissModeHint}
						aria-label="Dismiss mode hint"
						className="shrink-0 rounded p-0.5 text-ink-3 fine-hover:text-ink"
					>
						<IconX size={14} stroke={2} aria-hidden />
					</Button>
				</div>
			)}

			{(running || providerConnected) && (
				<p
					className={cn(
						"mt-1.5 px-1 text-[13px] tracking-[0.01em] text-ink-3",
						running && "text-accent",
					)}
				>
					{running
						? runStatusLabel(runStatus)
						: catalogEntry
							? `${catalogEntry.models.length} models · ⌘/Ctrl + Enter to send`
							: "⌘/Ctrl + Enter to send"}
				</p>
			)}
			</div>
		</div>
	);
}
