import { useEffect, useRef } from "react";
import { motion, useReducedMotion } from "motion/react";
import { IconAlertOctagon, IconCheck } from "@tabler/icons-react";
import { Button, cn } from "../../design";
import { useGrantApproval } from "../../ipc/hooks";
import { useSession } from "../../store/session";
import type { ApprovalScope, TimelineItem } from "../../ipc/types";

const APPROVAL_SUMMARIES: Record<string, string> = {
  shell: "Run a terminal command on your machine.",
  file_write: "Create or modify a file in your project.",
  file_delete: "Delete a file from your project.",
  file_edit: "Edit a file in your project.",
  bash: "Run a shell command on your machine.",
  network: "Make a network request from your machine.",
};

function approvalSummary(item: TimelineItem): string {
  const cls = item.actionClass ?? "shell";
  return (
    APPROVAL_SUMMARIES[cls] ??
    "The agent wants to perform an action on your machine."
  );
}

/**
 * Pending tool-call approval — sits above the composer.
 * Same surface language as the composer, with amber iconography to signal
 * attention without turning the inline decision into a modal alert.
 */
export function ApprovalNotice({ item }: { item: TimelineItem }) {
  const reduce = useReducedMotion();
  const grant = useGrantApproval();
  const chatId = useSession((s) => s.activeChatId);
  const actionClass = item.actionClass ?? "shell";
  const runId = item.runId ?? null;
  const decided = grant.isSuccess;
  const lastScope = grant.variables?.scope;
  const grantError = grant.isError;

  const cardRef = useRef<HTMLDivElement>(null);
  const focusedRef = useRef(false);
  useEffect(() => {
    if (decided || focusedRef.current) return;
    focusedRef.current = true;
    cardRef.current?.focus();
  }, [decided]);

  function decide(scope: ApprovalScope) {
    if (!chatId) return;
    grant.mutate({ chatId, runId, actionClass, scope });
  }

  const outcomeLabel =
    lastScope === "deny"
      ? "Denied"
      : lastScope === "this_chat"
        ? "Allowed for this chat"
        : "Allowed once";

  const summaryId = `approval-summary-${item.id}`;

  if (decided) {
    const denied = lastScope === "deny";
    const Icon = denied ? IconAlertOctagon : IconCheck;
    return (
      <motion.div
        initial={reduce ? false : { opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.15, ease: [0.23, 1, 0.32, 1] }}
        className={cn(
          "mb-2 flex items-center gap-2 px-1 text-[12px] leading-snug",
          denied ? "text-clay" : "text-ink-2",
        )}
      >
        <Icon size={13} stroke={1.75} className="shrink-0 opacity-70" aria-hidden />
        <span className="font-medium">{outcomeLabel}</span>
        <span className="select-none text-ink-3" aria-hidden="true">
          ·
        </span>
        <span className="min-w-0 truncate text-ink-3">{approvalSummary(item)}</span>
      </motion.div>
    );
  }

  return (
    <motion.div
      ref={cardRef}
      role="region"
      aria-label="Approval required"
      aria-describedby={summaryId}
      tabIndex={-1}
      initial={reduce ? false : { opacity: 0, transform: "translateY(4px)" }}
      animate={{ opacity: 1, transform: "translateY(0px)" }}
      transition={{ duration: 0.18, ease: [0.23, 1, 0.32, 1] }}
      className={cn(
        "mb-2 overflow-hidden rounded-card border border-line bg-surface outline-none",
        "shadow-soft-1",
      )}
    >
      <div className="min-w-0 px-3 py-2.5">
        <div className="flex items-center gap-1.5">
          <IconAlertOctagon size={15} stroke={1.75} className="shrink-0 text-amber" aria-hidden />
          <div className="text-[13px] font-semibold leading-snug text-ink">
            {item.title || "Approval needed"}
          </div>
        </div>
          <p
            id={summaryId}
            className="mt-0.5 text-[12.5px] leading-snug text-ink-2"
          >
            {approvalSummary(item)}
          </p>
          {item.body && (
            <pre className="mt-2 max-h-20 overflow-auto rounded-md border border-line bg-surface-2 px-2.5 py-1.5 font-mono text-[12px] leading-relaxed text-ink-2">
              {item.body.slice(0, 400)}
              {item.body.length > 400 && <span className="text-ink-3"> …</span>}
            </pre>
          )}
          {grantError && (
            <p className="mt-1.5 text-[12px] text-clay" role="alert">
              Couldn't reach the agent. Try again.
            </p>
          )}
          <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
            <Button
              variant="secondary"
              onClick={() => decide("once")}
              disabled={grant.isPending}
              className="h-7 px-2.5 text-[12.5px]"
            >
              {grant.isPending && grant.variables?.scope === "once"
                ? "…"
                : "Allow once"}
            </Button>
            <Button
              variant="secondary"
              onClick={() => decide("this_chat")}
              disabled={grant.isPending}
              className="h-7 px-2.5 text-[12.5px]"
            >
              For this chat
            </Button>
            <Button
              variant="ghost"
              onClick={() => decide("deny")}
              disabled={grant.isPending}
              className="h-7 px-2.5 text-[12.5px] text-clay fine-hover:bg-clay-tint fine-hover:text-clay"
            >
              Deny
            </Button>
          </div>
      </div>
    </motion.div>
  );
}
