import { bridge, type Chat } from "../bridge.ts";
import type { QueuedMessage } from "../conversationQueue";
import type { DispatchResult } from "./useConversation";

export function queuedWorkspace(key: string) {
  return key.slice(0, key.lastIndexOf(":"));
}

export function queuedThreadBusy(chats: Chat[], cwd: string, item: QueuedMessage) {
  const chat = chats.find((candidate) => candidate.cwd === cwd &&
    (item.targetViewId ? candidate.viewId === item.targetViewId : candidate.role !== "view"));
  return Boolean(chat?.activeTurn || (chat?.activity && chat.activity !== "已停止"));
}

/** Delivery follows the queued Space, even while another Space is on screen. */
export async function deliverQueuedMessage(
  cwd: string,
  item: QueuedMessage,
  sync: () => Promise<unknown>,
): Promise<DispatchResult> {
  try {
    const space = await bridge.space(cwd);
    const view = space.views.find((candidate) => candidate.id === item.targetViewId);
    if (item.targetViewId && !view) {
      return { status: "failed", error: "目标 View 已不存在，请编辑或删除这条消息。" };
    }
    await bridge.send({
      cwd, threadId: view?.threadId || space.mainThreadId || undefined,
      viewId: item.targetViewId || undefined, text: item.text, requestId: item.id,
      model: item.settings.modelId, effort: item.settings.effort,
      speed: item.settings.speed, permission: item.settings.permission,
    });
    try { await sync(); } catch { /* Accepted delivery is not retried on a read failure. */ }
    return { status: "sent" };
  } catch (cause) {
    const error = cause instanceof Error ? cause.message : String(cause);
    return /正在工作|工作中|working|active|busy/i.test(error)
      ? { status: "busy" } : { status: "failed", error };
  }
}
