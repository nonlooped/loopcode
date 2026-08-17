import type { TimelineMessage, ToolActivity } from "./index.ts";

export type TimelineActivityEntry =
  | { type: "message"; message: TimelineMessage; createdAt: number }
  | { type: "tool"; tool: ToolActivity; createdAt: number };

export type TimelineDisplayEntry =
  | Extract<TimelineActivityEntry, { type: "message" }>
  | {
      type: "work";
      id: string;
      entries: TimelineActivityEntry[];
      active: boolean;
      createdAt: number;
    };
