import type { Space, ViewRecord } from "../bridge";

export type EmbeddedView = Pick<ViewRecord, "id" | "key" | "title" | "html" | "status" | "summary">;

export function viewCatalog(space: Space): EmbeddedView[] {
  return space.views.map(({ id, key, title, html, status, summary }) =>
    ({ id, key, title, html, status, summary }));
}

export function findView(views: ViewRecord[], reference: unknown) {
  if (typeof reference !== "string" || !reference) return undefined;
  return views.find((view) => view.id === reference)
    || views.find((view) => view.key === reference);
}

export function safePayload(payload: unknown) {
  try {
    const serialized = JSON.stringify(payload ?? null);
    if (serialized.length > 4096) return undefined;
    return JSON.parse(serialized) as unknown;
  } catch {
    return undefined;
  }
}

export function surfaceEvent(data: unknown, channel: string, views: ViewRecord[]) {
  if (!data || typeof data !== "object") return null;
  const message = data as Record<string, unknown>;
  if (message.type !== "chat-ui:surface" || message.channel !== channel) return null;
  const view = findView(views, message.viewId);
  if (!view) return null;
  if (message.event === "navigate") return { view, navigate: true } as const;
  if (message.event !== "view-event" || typeof message.name !== "string") return null;
  const payload = safePayload(message.payload);
  if (payload === undefined) return null;
  return { view, navigate: false, event: { name: message.name.slice(0, 80), payload } } as const;
}
