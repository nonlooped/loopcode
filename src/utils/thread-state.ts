import type { ThreadState } from "../types/index.ts";

type EmptyThreadCandidate = Pick<
  ThreadState,
  "id" | "cwd" | "projectId" | "messages" | "tools" | "draft" | "settled"
>;

interface ThreadTarget {
  cwd: string;
  projectId: string | null;
}

export function findReusableEmptyThread<T extends EmptyThreadCandidate>(
  threads: readonly T[],
  target: ThreadTarget,
  hasAttachments: (threadId: string) => boolean,
): T | undefined {
  return threads.find(
    (thread) =>
      !thread.settled &&
      thread.cwd === target.cwd &&
      (thread.projectId ?? null) === target.projectId &&
      thread.messages.length === 0 &&
      thread.tools.length === 0 &&
      thread.draft.trim().length === 0 &&
      !hasAttachments(thread.id),
  );
}
