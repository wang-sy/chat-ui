import type { Dispatch, SetStateAction } from "react";
import type { ViewRecord } from "../bridge";
import { queueKey } from "../conversationQueue.ts";
import type { DispatchResult, Settings } from "./useConversation";

type EventPayload = { name: string; payload: unknown };

export async function routeViewEvent({
  view,
  event,
  settings,
  cwd,
  openView,
  openMain,
  setDrafts,
  isWorking,
  enqueue,
  dispatch,
}: {
  view: ViewRecord;
  event: EventPayload;
  settings: Settings;
  cwd: string;
  openView: (view: ViewRecord) => void;
  openMain: () => void;
  setDrafts: Dispatch<SetStateAction<Record<string, string>>>;
  isWorking: (view?: ViewRecord) => boolean;
  enqueue: (text: string, settings: Settings, view?: ViewRecord) => boolean;
  dispatch: (
    text: string,
    settings: Settings,
    view?: ViewRecord,
  ) => Promise<DispatchResult>;
}) {
  if (event.name === "focus") {
    // The source-checked iframe already belongs to the selected View.
    // Local focus must not expand the conversation over the page.
    return;
  }
  const payload = JSON.stringify(event.payload ?? null);
  if (event.name === "request-main") {
    openMain();
    const key = queueKey(cwd, null);
    setDrafts((current) => ({
      ...current,
      [key]: [current[key], `来自「${view.title}」的页面请求：${payload}`]
        .filter(Boolean)
        .join("\n\n"),
    }));
    return;
  }
  if (event.name !== "action") return;
  // Local filtering, toggles and selection must not start a paid agent turn.
  // An explicit request carries human-readable intent; ordinary UI telemetry does not.
  const intent = event.payload && typeof event.payload === "object"
    ? (event.payload as Record<string, unknown>).intent
    : undefined;
  if (typeof intent !== "string" || !intent.trim()) return;
  openView(view);
  const text = `页面交互事件：${event.name}\n事件数据：${payload}`;
  if (isWorking(view)) {
    enqueue(text, settings, view);
    return;
  }
  const result = await dispatch(text, settings, view);
  if (result.status === "busy") enqueue(text, settings, view);
}
