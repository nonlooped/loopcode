import type { MessageImage, ThreadState, TimelineMessage } from "../types/index.ts";

export function nextTimestamp(thread: ThreadState) {
  return Math.max(Date.now(), thread.updatedAt + 1);
}

export function addMessage(
  thread: ThreadState,
  role: TimelineMessage["role"],
  text: string,
  images: MessageImage[] = [],
) {
  const createdAt = nextTimestamp(thread);
  thread.messages.push({
    id: crypto.randomUUID(),
    role,
    text,
    ...(images.length > 0 ? { images: images.map((image) => ({ ...image })) } : {}),
    createdAt,
  });
  thread.updatedAt = createdAt;
}

export function appendMessage(
  thread: ThreadState,
  id: string,
  role: TimelineMessage["role"],
  text: string,
) {
  const existing = thread.messages.find((message) => message.id === id);
  if (existing) existing.text += text;
  else thread.messages.push({ id, role, text, createdAt: nextTimestamp(thread) });
  thread.updatedAt = nextTimestamp(thread);
}

export function titleFromPrompt(prompt: string) {
  const firstLine = prompt.split(/\r?\n/, 1)[0].trim();
  return firstLine.length > 42 ? `${firstLine.slice(0, 42)}...` : firstLine || "Untitled thread";
}
