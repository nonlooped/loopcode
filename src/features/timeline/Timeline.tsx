import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconAlertOctagon, IconChevronRight } from "@tabler/icons-react";
import { Button, ContextMenu, Markdown, Wordmark, cn } from "../../design";
import { copyText } from "../../lib/clipboard";
import { openExternal } from "../../ipc/commands";
import { useSession } from "../../store/session";
import type {
  TimelineItem,
  TimelineProjection,
} from "../../ipc/types";
import { ModeHandoffCard } from "./ModeHandoffCard";
import {
  buildToolCallRows,
  summarizeTools,
  type DiffLine,
  type ToolCallRow,
} from "./toolActivity";

type Role = "user" | "assistant" | "tool" | "error" | "system";

/** One visual row in the chat — tools nest under an assistant turn. */
type TimelineBlock =
  | { type: "user"; id: string; item: TimelineItem }
  | { type: "assistant"; id: string; messages: TimelineItem[]; tools: TimelineItem[] }
  | { type: "error"; id: string; item: TimelineItem }
  | { type: "system"; id: string; item: TimelineItem };

const HIDDEN_KINDS = new Set<string>(["lifecycle", "usage", "unknown"]);

const ACTIVE_RUN_STATUSES = new Set([
  "queued",
  "preparing",
  "model_active",
  "tool_active",
  "approval_waiting",
  "cancelling",
]);

/** Distance from bottom that still counts as "following" the live turn. */
const NEAR_BOTTOM_PX = 96;

function scrollParentOf(el: HTMLElement | null): HTMLElement | null {
  if (!el) return null;
  const marked = el.closest("[data-timeline-scroll]");
  if (marked instanceof HTMLElement) return marked;
  let node: HTMLElement | null = el.parentElement;
  while (node) {
    const { overflowY } = getComputedStyle(node);
    if (overflowY === "auto" || overflowY === "scroll" || overflowY === "overlay") {
      return node;
    }
    node = node.parentElement;
  }
  return null;
}

function isNearBottom(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight <= NEAR_BOTTOM_PX;
}

function scrollToBottom(el: HTMLElement): void {
  el.scrollTop = el.scrollHeight;
}

/** Internal Core telemetry / state — never shown as chat rows. */
function isUserFacing(item: TimelineItem): boolean {
  if (HIDDEN_KINDS.has(item.kind)) return false;
  // Pending approvals render on the composer; resolved ones are noise.
  if (item.kind === "approval" || item.needsApproval) return false;
  return true;
}

function roleOf(item: TimelineItem): Role {
  if (
    item.kind === "tool_proposal" ||
    item.kind === "tool_result" ||
    item.kind === "tool_rejected" ||
    item.kind.startsWith("tool")
  ) {
    return "tool";
  }
  if (item.kind === "error") return "error";
  if (item.kind === "system") return "system";
  if (
    item.kind === "user_message" ||
    item.kind === "user" ||
    item.title.trim().toLowerCase() === "you"
  ) {
    return "user";
  }
  if (
    item.kind === "assistant_message" ||
    item.kind === "assistant" ||
    item.kind === "assistantMessage"
  ) {
    return "assistant";
  }
  // Last-resort heuristics for older payloads — never treat lifecycle as chat.
  if (item.toolName) return "tool";
  if (/^(Error|Retry)/.test(item.title)) return "error";
  return "assistant";
}

function isRunActive(projection: TimelineProjection | null): boolean {
  if (!projection?.activeRunId) return false;
  const status = projection.activeRunStatus ?? "";
  if (!status) return true;
  return ACTIVE_RUN_STATUSES.has(status);
}

/**
 * Fold consecutive tool + assistant items into one assistant turn so tool
 * activity sits under the Assistant header, not under the preceding user turn.
 */
function groupIntoBlocks(items: TimelineItem[]): TimelineBlock[] {
  const blocks: TimelineBlock[] = [];
  let i = 0;
  while (i < items.length) {
    const item = items[i];
    const role = roleOf(item);

    if (role === "user") {
      blocks.push({ type: "user", id: item.id, item });
      i += 1;
      continue;
    }
    if (role === "error" || role === "system") {
      blocks.push({ type: role, id: item.id, item });
      i += 1;
      continue;
    }

    // tool | assistant → one assistant turn (tools may arrive before any reply text)
    const tools: TimelineItem[] = [];
    const messages: TimelineItem[] = [];
    const id = item.id;
    while (i < items.length) {
      const r = roleOf(items[i]);
      if (r === "tool") {
        tools.push(items[i]);
        i += 1;
      } else if (r === "assistant") {
        messages.push(items[i]);
        i += 1;
      } else {
        break;
      }
    }
    blocks.push({ type: "assistant", id, messages, tools });
  }
  return blocks;
}

/** Inject a live Assistant row when the run is active but no assistant events yet. */
function shouldInjectThinking(blocks: TimelineBlock[], streaming: boolean): boolean {
  if (!streaming) return false;
  const last = blocks[blocks.length - 1];
  return !last || last.type !== "assistant";
}

export function Timeline({ projection }: { projection: TimelineProjection | null }) {
  const reduce = useReducedMotion();
  const chatId = useSession((s) => s.activeChatId);
  const allItems = projection?.items ?? [];
  const items = useMemo(() => allItems.filter(isUserFacing), [allItems]);
  const blocks = useMemo(() => groupIntoBlocks(items), [items]);
  const streaming = isRunActive(projection);
  const injectThinking = shouldInjectThinking(blocks, streaming);
  const empty = !projection || (blocks.length === 0 && !injectThinking);

  const rootRef = useRef<HTMLDivElement>(null);
  const stickToBottomRef = useRef(true);
  const pinnedChatRef = useRef<string | null>(null);
  const lastItemSigRef = useRef<string>("");

  const lastItem = items.length > 0 ? items[items.length - 1] : null;
  const contentSig = `${chatId ?? ""}:${items.length}:${lastItem?.id ?? ""}:${lastItem?.body?.length ?? 0}:${streaming}:${injectThinking}`;

  // Last assistant turn is the live stream target while a run is active.
  let lastAssistantId: string | null = null;
  for (let i = blocks.length - 1; i >= 0; i--) {
    if (blocks[i].type === "assistant") {
      lastAssistantId = blocks[i].id;
      break;
    }
  }

  // Only newly appended rows animate — never re-stagger history on poll or chat open.
  const seenRef = useRef(new Set<string>());
  const primedChatRef = useRef<string | null>(null);
  const primed = primedChatRef.current === chatId && chatId != null;

  useEffect(() => {
    if (primedChatRef.current !== chatId) {
      seenRef.current = new Set(blocks.map((b) => b.id));
      primedChatRef.current = chatId;
      return;
    }
    for (const block of blocks) seenRef.current.add(block.id);
  }, [blocks, chatId]);

  // Track whether the user is following the bottom (so we don't yank on history scroll).
  useEffect(() => {
    if (empty) return;
    const scroller = scrollParentOf(rootRef.current);
    if (!scroller) return;
    const onScroll = () => {
      stickToBottomRef.current = isNearBottom(scroller);
    };
    scroller.addEventListener("scroll", onScroll, { passive: true });
    return () => scroller.removeEventListener("scroll", onScroll);
  }, [empty, chatId]);

  // Keep the viewport pinned while new prompts / streamed content arrive.
  useLayoutEffect(() => {
    if (empty) return;
    const scroller = scrollParentOf(rootRef.current);
    if (!scroller) return;

    if (pinnedChatRef.current !== chatId) {
      pinnedChatRef.current = chatId;
      stickToBottomRef.current = true;
      scrollToBottom(scroller);
      lastItemSigRef.current = contentSig;
      return;
    }

    // A newly appended user prompt always re-pins to the bottom.
    const last = items[items.length - 1];
    const prevLastId = lastItemSigRef.current.split(":")[2] || "";
    if (last && roleOf(last) === "user" && last.id !== prevLastId) {
      stickToBottomRef.current = true;
    }

    if (stickToBottomRef.current) {
      scrollToBottom(scroller);
    }
    lastItemSigRef.current = contentSig;
  }, [contentSig, empty, chatId, items]);

  // Height can grow without item identity changing (Thinking…, markdown layout).
  useEffect(() => {
    if (empty) return;
    const root = rootRef.current;
    const scroller = scrollParentOf(root);
    if (!root || !scroller) return;
    const ro = new ResizeObserver(() => {
      if (stickToBottomRef.current) scrollToBottom(scroller);
    });
    ro.observe(root);
    return () => ro.disconnect();
  }, [empty, chatId]);

  if (empty) {
    return (
      <div className="grid h-full place-items-center px-6 text-center">
        <EmptyState statusLabel={projection?.statusLabel} reduce={Boolean(reduce)} />
      </div>
    );
  }

  return (
    <div ref={rootRef} className="mx-auto flex w-full max-w-[52.8rem] flex-col px-5 py-6">
      {blocks.map((block) => {
        const isNew = primed && !seenRef.current.has(block.id);
        const isStreamingAssistant =
          streaming && block.type === "assistant" && block.id === lastAssistantId;
        return (
          <motion.div
            key={block.id}
            // User prompts are chapter markers — the turn gap is the divider.
            className={block.type === "user" ? "mt-10 first:mt-0" : "mt-5 first:mt-0"}
            initial={
              isNew && !reduce
                ? { opacity: 0, transform: "translateY(8px)" }
                : false
            }
            animate={{ opacity: 1, transform: "translateY(0px)" }}
            transition={{
              duration: reduce ? 0.01 : 0.22,
              ease: [0.23, 1, 0.32, 1],
            }}
          >
            <TimelineBlockView block={block} isStreaming={isStreamingAssistant} />
          </motion.div>
        );
      })}
      {injectThinking && (
        <motion.div
          key="thinking-placeholder"
          className="mt-5 first:mt-0"
          initial={reduce ? false : { opacity: 0, transform: "translateY(8px)" }}
          animate={{ opacity: 1, transform: "translateY(0px)" }}
          transition={{ duration: reduce ? 0.01 : 0.22, ease: [0.23, 1, 0.32, 1] }}
        >
          <AssistantTurn messages={[]} tools={[]} isStreaming />
        </motion.div>
      )}
    </div>
  );
}

function EmptyState({
  statusLabel,
  reduce,
}: {
  statusLabel?: string | null;
  reduce: boolean;
}) {
  // Don't surface internal run status strings in the empty prompt.
  const hint =
    statusLabel &&
    !/^(completed|failed|cancelled|idle|running|queued)$/i.test(statusLabel.trim())
      ? statusLabel
      : null;

  return (
    <motion.div
      className="max-w-sm"
      initial={reduce ? false : { opacity: 0, transform: "translateY(6px)" }}
      animate={{ opacity: 1, transform: "translateY(0px)" }}
      transition={{ duration: 0.28, ease: [0.23, 1, 0.32, 1] }}
    >
      <Wordmark markOnly className="justify-center text-4xl opacity-90" />
      <h3 className="mt-4 font-serif text-xl tracking-tight">Start the conversation</h3>
      <p className="mt-2 text-[14.5px] leading-relaxed text-ink-2">
        {hint ?? "Ask a question about this project, or describe what you want to build."}
      </p>
    </motion.div>
  );
}

function TimelineBlockView({
  block,
  isStreaming = false,
}: {
  block: TimelineBlock;
  isStreaming?: boolean;
}) {
  if (block.type === "error") return <ErrorCard item={block.item} />;
  if (block.type === "system") return <SystemNote item={block.item} />;
  if (block.type === "user") return <UserMessage item={block.item} />;
  return (
    <AssistantTurn
      messages={block.messages}
      tools={block.tools}
      isStreaming={isStreaming}
    />
  );
}

function UserMessage({ item }: { item: TimelineItem }) {
  return (
    <ContextMenu
      items={[
        { label: "Copy message", onSelect: () => void copyText(item.body) },
        { label: "Copy as Markdown", onSelect: () => void copyText(item.body) },
        "separator",
        { label: "Quote / reply", disabled: true },
        { label: "Edit and resend", disabled: true },
      ]}
    >
      {/* Prompt as a section marker: a quiet surface card, not a chat bubble. */}
      <div className="rounded-card border border-line bg-surface px-4 py-3 shadow-soft-1">
        <div className="text-[15px] leading-relaxed whitespace-pre-wrap break-words text-ink">
          {item.body}
        </div>
      </div>
    </ContextMenu>
  );
}

function AssistantTurn({
  messages,
  tools,
  isStreaming = false,
}: {
  messages: TimelineItem[];
  tools: TimelineItem[];
  isStreaming?: boolean;
}) {
  const toolSummary = summarizeTools(tools);
  const toolRows = buildToolCallRows(tools);
  const body = messages
    .map((m) => m.body)
    .filter((b) => b?.trim())
    .join("\n\n");
  const showThinking = isStreaming && !body && tools.length === 0;
  // Skip empty turns (e.g. history with no tools and no reply).
  if (!body && !isStreaming && toolRows.length === 0) return null;

  // The assistant speaks in the page's own voice — prose on paper, no chrome.
  return (
    <div className="min-w-0">
      {toolRows.length > 0 && (
        <div className="mb-3 border-l border-line pl-3">
          <ToolActivity summary={toolSummary} rows={toolRows} />
        </div>
      )}
      {tools.map((t) => (
        <ModeHandoffCard key={`handoff-${t.id}`} item={t} />
      ))}
      {showThinking && <ThinkingLabel />}
      {body && (
        <ContextMenu
          items={[
            { label: "Copy message", onSelect: () => void copyText(body) },
            { label: "Copy as Markdown", onSelect: () => void copyText(body) },
            "separator",
            { label: "Quote / reply", disabled: true },
            { label: "Regenerate", disabled: true },
          ]}
        >
          <Markdown
            mode={isStreaming ? "streaming" : "static"}
            isAnimating={isStreaming}
            caret="block"
          >
            {body}
          </Markdown>
        </ContextMenu>
      )}
    </div>
  );
}

function ThinkingLabel() {
  const reduce = useReducedMotion();
  return (
    <motion.p
      animate={reduce ? { opacity: 0.72 } : { opacity: [0.42, 1, 0.42] }}
      transition={reduce ? { duration: 0 } : { duration: 1.8, repeat: Infinity, ease: "easeInOut" }}
      className="py-0.5 text-[13px] leading-snug text-ink-3"
      aria-live="polite"
    >
      Thinking…
    </motion.p>
  );
}

/** Aggregated activity line with expandable per-call rows (or a single row inline). */
function ToolActivity({
  summary,
  rows,
}: {
  summary: string | null;
  rows: ToolCallRow[];
}) {
  const [open, setOpen] = useState(false);
  const reduce = useReducedMotion();

  // One call: show it directly — no nested "Used webfetch → Used" accordion.
  if (rows.length === 1) {
    return <ToolCallRowView row={rows[0]} />;
  }

  const canExpand = rows.length > 0;
  const label = summary ?? "Used tools";

  return (
    <div>
      <Button
        variant="ghost"
        disabled={!canExpand}
        aria-expanded={canExpand ? open : undefined}
        onClick={() => {
          if (canExpand) setOpen((v) => !v);
        }}
        className={cn(
          "group inline-flex max-w-full items-start gap-1 rounded-md py-0.5 text-left text-[12px] leading-snug text-ink-3",
          canExpand &&
            "fine-hover:text-ink-2 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
          !canExpand && "cursor-default",
        )}
      >
        <span className="min-w-0">{label}</span>
        {canExpand && (
          <IconChevronRight
            size={14}
            stroke={2}
            aria-hidden
            className={cn(
              "mt-0.5 shrink-0 text-ink-3 opacity-70 transition-transform duration-[var(--duration-fast)] ease-[var(--ease-out)]",
              open && "rotate-90",
            )}
          />
        )}
      </Button>

      <AnimatePresence initial={false}>
        {open && (
          <motion.ul
            key="tool-rows"
            initial={reduce ? false : { opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, height: 0 }}
            transition={{ duration: reduce ? 0.12 : 0.2, ease: [0.23, 1, 0.32, 1] }}
            className="mt-1 space-y-0.5 overflow-hidden pl-3"
          >
            {rows.map((row) => (
              <li key={row.callId}>
                <ToolCallRowView row={row} />
              </li>
            ))}
          </motion.ul>
        )}
      </AnimatePresence>
    </div>
  );
}

function ToolCallRowView({ row }: { row: ToolCallRow }) {
  const [open, setOpen] = useState(false);
  const reduce = useReducedMotion();
  const activeProjectId = useSession((s) => s.activeProjectId);
  const hunkText = row.hunk
    ?.map((line) => `${line.kind === "add" ? "+" : line.kind === "del" ? "-" : " "}${line.text}`)
    .join("\n");
  const canOpenFile = Boolean(
    activeProjectId && row.detailFull && ["edit", "patch", "write", "read"].includes(row.name),
  );
  const menuItems = [
    { label: "Copy command / arguments", onSelect: () => void copyText(row.detailFull ?? row.name) },
    ...(hunkText ? [{ label: "Copy hunk", onSelect: () => void copyText(hunkText) }] : []),
    {
      label: "Open affected file",
      disabled: !canOpenFile,
      onSelect: () => {
        if (activeProjectId && row.detailFull) void openExternal(activeProjectId, row.detailFull);
      },
    },
  ];
  const stats =
    row.status === "used" && (row.added != null || row.removed != null) ? (
      <span className="ml-1.5 inline-flex items-center gap-1 font-mono text-[11.5px] tabular-nums">
        {row.added != null && row.added > 0 && (
          <span className="text-emerald-600 dark:text-emerald-400">+{row.added}</span>
        )}
        {row.removed != null && row.removed > 0 && (
          <span className="text-red-600/90 dark:text-red-400">-{row.removed}</span>
        )}
      </span>
    ) : null;

  const label = (
    <>
      {row.status === "using" && (
        <motion.span
          animate={reduce ? { opacity: 0.72 } : { opacity: [1, 0.45, 1] }}
          transition={reduce ? { duration: 0 } : { duration: 1.6, repeat: Infinity, ease: "easeInOut" }}
          className="mr-1.5 inline-block h-1.5 w-1.5 rounded-full bg-amber align-middle"
          aria-hidden="true"
        />
      )}
      <span className="font-semibold text-ink-2">{row.verb}</span>
      {row.detail && (
        <span className="ml-1.5 font-mono text-ink-3" title={row.detailFull ?? row.detail}>
          {row.detail}
        </span>
      )}
      {stats}
    </>
  );

  if (!row.expandable || !row.hunk) {
    return (
      <ContextMenu items={menuItems}>
        <div className="py-0.5 text-[12px] leading-snug text-ink-3">{label}</div>
      </ContextMenu>
    );
  }

  return (
    <ContextMenu items={menuItems}>
      <div>
      <Button
        variant="ghost"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className={cn(
          "rounded-md py-0.5 text-left text-[12px] leading-snug text-ink-3",
          "fine-hover:text-ink-2 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
        )}
      >
        {label}
        <IconChevronRight
          size={13}
          stroke={2}
          aria-hidden
          className={cn(
            "ml-1 inline-block align-text-bottom opacity-60 transition-transform duration-[var(--duration-fast)] ease-[var(--ease-out)]",
            open && "rotate-90",
          )}
        />
      </Button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            key="hunk"
            initial={reduce ? false : { opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, height: 0 }}
            transition={{ duration: reduce ? 0.12 : 0.18, ease: [0.23, 1, 0.32, 1] }}
            className="overflow-hidden"
          >
            <ToolHunk lines={row.hunk} />
          </motion.div>
        )}
      </AnimatePresence>
      </div>
    </ContextMenu>
  );
}

function ToolHunk({ lines }: { lines: DiffLine[] }) {
  return (
    <pre
      className={cn(
        "mt-1 mb-1 max-h-64 overflow-auto rounded-lg border border-line bg-surface-2 px-2.5 py-2",
        "font-mono text-[11.5px] leading-relaxed",
      )}
    >
      {lines.map((line, index) => (
        <div
          key={`${index}-${line.kind}`}
          className={cn(
            "whitespace-pre-wrap break-words",
            line.kind === "add" && "text-emerald-700 dark:text-emerald-400",
            line.kind === "del" && "text-red-700/90 dark:text-red-400",
            line.kind === "ctx" && "text-ink-3",
          )}
        >
          <span className="select-none opacity-70" aria-hidden="true">
            {line.kind === "add" ? "+" : line.kind === "del" ? "-" : " "}
          </span>
          {line.text || " "}
        </div>
      ))}
    </pre>
  );
}

function SystemNote({ item }: { item: TimelineItem }) {
  // Limit / retry notices — still user-relevant, but low emphasis.
  const text = [item.title, item.body].filter(Boolean).join(" — ").trim();
  if (!text) return null;
  return (
    <div className="py-0.5 text-[12px] leading-snug text-ink-3">
      {text.length > 200 ? `${text.slice(0, 197).trimEnd()}…` : text}
    </div>
  );
}

function ErrorCard({ item }: { item: TimelineItem }) {
  const next = /next_action=([^\n]+)/.exec(item.body)?.[1];
  const message = item.body
    .split("\n")
    .filter((line) => !line.startsWith("retryability=") && !line.startsWith("next_action="))
    .join("\n")
    .trim();

  return (
    <div className="rounded-card border border-clay/40 bg-clay-tint px-4 py-3">
      <div className="flex items-center gap-2 text-[14px] font-semibold text-clay">
        <IconAlertOctagon size={16} stroke={2} className="shrink-0" aria-hidden />
        {item.title}
      </div>
      {message && (
        <div className="mt-1 text-[13.5px] whitespace-pre-wrap break-words text-ink-2 line-clamp-6">{message}</div>
      )}
      {next && <div className="mt-2 text-[13px] font-medium text-ink">Next: {next}</div>}
    </div>
  );
}
