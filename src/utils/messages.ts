import type { MessageImage, PromptPart, ThreadState, TimelineMessage } from "../types/index.ts";

export function nextTimestamp(thread: ThreadState) {
  return Math.max(Date.now(), thread.updatedAt + 1);
}

export function addMessage(
  thread: ThreadState,
  role: TimelineMessage["role"],
  text: string,
  images: MessageImage[] = [],
  content?: PromptPart[],
) {
  const createdAt = nextTimestamp(thread);
  const message: TimelineMessage = {
    id: crypto.randomUUID(),
    role,
    text,
    createdAt,
  };
  if (content) message.content = content.map(copyPromptPart);
  if (images.length > 0) message.images = images.map((image) => ({ ...image }));
  thread.messages.push(message);
  thread.updatedAt = createdAt;
}

export function copyPromptPart(part: PromptPart): PromptPart {
  return part.type === "text"
    ? { ...part }
    : { type: "reference", reference: { ...part.reference } };
}

export function appendMessage(
  thread: ThreadState,
  id: string,
  role: TimelineMessage["role"],
  text: string,
) {
  if (!id.trim()) throw new Error("Message id cannot be empty");
  const existing = thread.messages.find((message) => message.id === id);
  if (existing && existing.role !== role) throw new Error("Message role cannot change");
  const timestamp = nextTimestamp(thread);
  if (existing) existing.text += text;
  else thread.messages.push({ id, role, text, createdAt: timestamp });
  thread.updatedAt = timestamp;
}

export function titleFromPrompt(prompt: string) {
  const words = prompt.trim().match(/\S+/g) ?? [];
  if (words.length === 0) return "Untitled thread";
  return `${words.slice(0, 6).join(" ")}${words.length > 6 ? "..." : ""}`;
}
