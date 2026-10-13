import type { PermissionMode } from "./bridge";
import type { ModelSpeed } from "./modelSettings";

export type QueueSettings = {
  modelId: string;
  effort: string;
  speed: ModelSpeed;
  permission: PermissionMode;
};

export type QueuedMessage = {
  id: string;
  text: string;
  targetViewId: string | null;
  targetTitle: string | null;
  settings: QueueSettings;
  createdAt: number;
  error?: string;
};

export type QueuedMessages = Record<string, QueuedMessage[]>;

const STORAGE_KEY = "chat-ui:queued-messages";
const MAX_QUEUE_ITEMS = 20;

function isQueueSettings(value: unknown): value is QueueSettings {
  if (!value || typeof value !== "object") return false;
  const settings = value as Record<string, unknown>;
  return (
    typeof settings.modelId === "string" &&
    typeof settings.effort === "string" &&
    (settings.speed === "standard" ||
      settings.speed === "fast" ||
      settings.speed === "turbo") &&
    (settings.permission === "ask" ||
      settings.permission === "auto" ||
      settings.permission === "full")
  );
}

function isQueuedMessage(value: unknown): value is QueuedMessage {
  if (!value || typeof value !== "object") return false;
  const item = value as Record<string, unknown>;
  return (
    typeof item.id === "string" &&
    typeof item.text === "string" &&
    item.text.trim().length > 0 &&
    item.text.length <= 100_000 &&
    (item.targetViewId === null || typeof item.targetViewId === "string") &&
    (item.targetTitle === null || typeof item.targetTitle === "string") &&
    isQueueSettings(item.settings) &&
    typeof item.createdAt === "number" &&
    Number.isFinite(item.createdAt)
  );
}

export function loadQueuedMessages(): QueuedMessages {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return {};
    return Object.fromEntries(
      Object.entries(parsed as Record<string, unknown>)
        .map(([key, value]) => [
          key,
          Array.isArray(value)
            ? value.filter(isQueuedMessage).slice(0, MAX_QUEUE_ITEMS)
            : [],
        ])
        .filter(([, value]) => (value as QueuedMessage[]).length > 0),
    );
  } catch {
    return {};
  }
}

export function saveQueuedMessages(queues: QueuedMessages) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(queues));
  } catch {
    // A full or disabled localStorage should not stop a conversation.
  }
}

export function queueKey(cwd: string, viewId: string | null) {
  return `${cwd}:${viewId || "main"}`;
}

export function maxQueueItems() {
  return MAX_QUEUE_ITEMS;
}

/**
 * Queue mutations live here instead of inside the React hook so ordering,
 * limits and the four user actions can be verified without mounting the UI.
 */
export function appendQueuedMessage(
  queue: QueuedMessage[],
  item: QueuedMessage,
): QueuedMessage[] | null {
  if (queue.length >= MAX_QUEUE_ITEMS) return null;
  return [...queue, item];
}

export function editQueuedMessage(
  queue: QueuedMessage[],
  id: string,
  text: string,
): QueuedMessage[] {
  return queue.map((item) =>
    item.id === id ? { ...item, text, error: undefined } : item,
  );
}

export function removeQueuedMessage(
  queue: QueuedMessage[],
  id: string,
): QueuedMessage[] {
  return queue.filter((item) => item.id !== id);
}

export function reorderQueuedMessages(
  queue: QueuedMessage[],
  fromId: string,
  toId: string,
): QueuedMessage[] {
  if (fromId === toId) return queue;
  const fromIndex = queue.findIndex((item) => item.id === fromId);
  const toIndex = queue.findIndex((item) => item.id === toId);
  if (fromIndex < 0 || toIndex < 0) return queue;
  const next = [...queue];
  const [moved] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, moved);
  return next;
}
