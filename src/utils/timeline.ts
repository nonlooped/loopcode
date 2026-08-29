import type { ThreadState, TimelineMessage } from "../types/index.ts";
import type { TimelineActivityEntry, TimelineDisplayEntry } from "../types/timeline.ts";
import { threadStatus } from "./threads.ts";

export function timelineEntries(thread: ThreadState): TimelineDisplayEntry[] {
  const entries: TimelineActivityEntry[] = [
    ...thread.messages.map((message) => ({
      type: "message" as const,
      message,
      createdAt: message.createdAt,
    })),
    ...thread.tools.map((tool) => ({ type: "tool" as const, tool, createdAt: tool.createdAt })),
  ].sort((left, right) => left.createdAt - right.createdAt);

  const displayEntries: TimelineDisplayEntry[] = [];
  let segmentStart = 0;

  for (let index = 0; index <= entries.length; index += 1) {
    const entry = entries[index];
    const startsNextTurn =
      entry?.type === "message" && entry.message.role === "user" && !entry.message.followUp;
    if (index > segmentStart && (startsNextTurn || index === entries.length)) {
      displayEntries.push(
        ...collapseTurnActivity(
          entries.slice(segmentStart, index),
          threadStatus(thread) === "running" && index === entries.length,
        ),
      );
      segmentStart = index;
    }
  }

  if (segmentStart < entries.length) {
    displayEntries.push(
      ...collapseTurnActivity(entries.slice(segmentStart), threadStatus(thread) === "running"),
    );
  }

  return displayEntries;
}

export function collapseTurnActivity(
  entries: TimelineActivityEntry[],
  active: boolean,
): TimelineDisplayEntry[] {
  const terminalAgentEntry = active
    ? undefined
    : [...entries]
        .reverse()
        .find(
          (entry): entry is Extract<TimelineActivityEntry, { type: "message" }> =>
            entry.type === "message" &&
            entry.message.role === "agent" &&
            entry.message.text.trim().length > 0,
        );
  const terminalAgentId = terminalAgentEntry?.message.id;
  const workEntries = entries.filter(
    (entry) =>
      entry.type === "tool" ||
      entry.message.role === "thought" ||
      (entry.message.role === "agent" && entry.message.id !== terminalAgentId),
  );

  if (workEntries.length === 0) {
    return entries.filter(
      (entry): entry is Extract<TimelineActivityEntry, { type: "message" }> =>
        entry.type === "message",
    );
  }

  const firstWorkEntry = workEntries[0];
  const workIds = new Set(
    workEntries.map((entry) =>
      entry.type === "tool" ? `tool-${entry.tool.id}` : `message-${entry.message.id}`,
    ),
  );
  const collapsed: TimelineDisplayEntry[] = [];

  for (const entry of entries) {
    const key = entry.type === "tool" ? `tool-${entry.tool.id}` : `message-${entry.message.id}`;
    if (!workIds.has(key)) {
      if (entry.type === "message") collapsed.push(entry);
      continue;
    }
    if (entry !== firstWorkEntry) continue;
    collapsed.push({
      type: "work",
      id: `work-${key}`,
      entries: workEntries,
      active,
      createdAt: entry.createdAt,
      startedAt: entries[0]?.createdAt ?? entry.createdAt,
      durationMs: active
        ? null
        : Math.max(
            0,
            (entries.at(-1)?.createdAt ?? entry.createdAt) -
              (entries[0]?.createdAt ?? entry.createdAt),
          ),
    });
  }

  return collapsed;
}

/**
 * Only the newest message in a running work group is still streaming. Marking them all
 * leaves a caret stranded after every finished thought.
 */
export function streamingMessageId(entry: Extract<TimelineDisplayEntry, { type: "work" }>) {
  if (!entry.active) return undefined;
  return entry.entries
    .filter(
      (item): item is Extract<TimelineActivityEntry, { type: "message" }> =>
        item.type === "message",
    )
    .at(-1)?.message.id;
}

const compactFormat = new Intl.NumberFormat(undefined, {
  notation: "compact",
  maximumFractionDigits: 1,
});

export function compactNumber(value: number) {
  return compactFormat.format(value);
}

export function formatElapsedDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.round(durationMs / 1_000));
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return seconds === 0 ? `${minutes}m` : `${minutes}m ${seconds}s`;
}

export function isStreamingMessage(thread: ThreadState, message: TimelineMessage) {
  if (message.role !== "agent" || threadStatus(thread) !== "running") return false;
  for (let index = thread.messages.length - 1; index >= 0; index -= 1) {
    const item = thread.messages[index];
    if (item.role === "user") return false;
    if (item.role === "agent") return item.id === message.id;
  }
  return false;
}
