import { useEffect, useRef, useState } from "react";
import {
  bridge,
  type AgentRequest,
  type Chat,
  type Space,
  type ViewRecord,
} from "../bridge";
import {
  appendQueuedMessage,
  editQueuedMessage,
  loadQueuedMessages,
  maxQueueItems,
  queueKey,
  removeQueuedMessage,
  reorderQueuedMessages,
  saveQueuedMessages,
  type QueuedMessage,
  type QueuedMessages,
  type QueueSettings,
} from "../conversationQueue";
import type { ModelSpeed } from "../modelSettings";
import { routeViewEvent } from "./conversationEvents";
import { steerIfActive } from "./conversationSteer";
import { deliverQueuedMessage, queuedThreadBusy, queuedWorkspace } from "./queueDelivery";
import { errorText } from "./useWorkspace";

export type Settings = QueueSettings & {
  modelId: string;
  effort: string;
  speed: ModelSpeed;
};
type Props = {
  cwd: string;
  space: Space | null;
  chats: Chat[];
  refresh: () => Promise<unknown>;
  loadSpace: (path: string) => Promise<void>;
  onError: (message: string) => void;
  onChooseProject: () => void;
};
export type DispatchResult = {
  status: "sent" | "busy" | "failed";
  error?: string;
};

export function useConversation({
  cwd,
  space,
  chats,
  refresh,
  loadSpace,
  onError,
  onChooseProject,
}: Props) {
  const [open, setOpen] = useState(false);
  const [viewId, setViewId] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [queues, setQueues] = useState<QueuedMessages>(() =>
    loadQueuedMessages(),
  );
  const [busy, setBusy] = useState(false);
  const sending = useRef(false);
  const drainLock = useRef(false);
  const currentPath = useRef(cwd);
  currentPath.current = cwd;
  const retry = useRef<{ key: string; id: string } | null>(null);
  const target = viewId ? ("view" as const) : ("main" as const);
  const targetView = space?.views.find((view) => view.id === viewId);
  const mainChat =
    chats.find((item) => item.cwd === cwd && item.id === space?.mainThreadId) ||
    chats.find((item) => item.cwd === cwd && item.role !== "view");
  const chat = viewId
    ? chats.find((item) => item.id === targetView?.threadId && item.cwd === cwd)
    : mainChat;
  const draftKey = queueKey(cwd, viewId);
  const draft = drafts[draftKey] || "";
  const queue = queues[draftKey] || [];
  useEffect(() => {
    if (!open || !chat || chat.activeTurn) return;
    let cancelled = false;
    void bridge.openChat(chat.id).then(() => { if (!cancelled) void refresh(); })
      .catch((cause) => { if (!cancelled) onError(`已保留本地对话，历史同步失败：${errorText(cause)}`); });
    return () => { cancelled = true; };
    // A thread is restored once when selected, not on every streamed snapshot.
  }, [open, chat?.id]);
  useEffect(() => {
    saveQueuedMessages(queues);
  }, [queues]);
  useEffect(() => {
    setViewId(null);
    setOpen(false);
  }, [cwd]);
  useEffect(() => {
    if (space?.cwd === cwd && viewId && !targetView) {
      setViewId(null);
      onError("这个 View 已从 Space 移除，已回到 Main Thread。");
    }
  }, [space, cwd, viewId, targetView, onError]);
  const setDraft = (text: string) =>
    setDrafts((current) => ({ ...current, [draftKey]: text }));

  function openMain() {
    setViewId(null);
    setOpen(true);
  }
  function openView(view: ViewRecord) {
    setViewId(view.id);
    setOpen(true);
  }
  function threadFor(recipient?: ViewRecord) {
    if (!recipient) return mainChat;
    return chats.find((item) => item.id === recipient.threadId && item.cwd === cwd);
  }

  function isWorking(recipient?: ViewRecord) {
    const thread = threadFor(recipient);
    return Boolean(thread?.activeTurn || (thread?.activity && thread.activity !== "已停止"));
  }

  function updateQueue(
    key: string,
    updater: (items: QueuedMessage[]) => QueuedMessage[],
  ) {
    setQueues((current) => {
      const next = updater(current[key] || []);
      if (!next.length) {
        const { [key]: _removed, ...rest } = current;
        return rest;
      }
      return { ...current, [key]: next };
    });
  }

  function enqueue(text: string, settings: Settings, recipient?: ViewRecord) {
    if (!cwd) {
      onChooseProject();
      return false;
    }
    const key = queueKey(cwd, recipient?.id || null);
    const current = queues[key] || [];
    const item: QueuedMessage = {
      id: crypto.randomUUID(),
      text: text.trim(),
      targetViewId: recipient?.id || null,
      targetTitle: recipient?.title || null,
      settings: { ...settings },
      createdAt: Date.now(),
    };
    const next = appendQueuedMessage(current, item);
    if (!next) {
      onError(`排队消息已达到 ${maxQueueItems()} 条，请先处理已有消息。`);
      return false;
    }
    updateQueue(key, () => next);
    setOpen(true);
    onError("");
    return true;
  }

  async function dispatch(
    text: string,
    settings: Settings,
    recipient?: ViewRecord,
    requestIdOverride?: string,
  ): Promise<DispatchResult> {
    const trimmed = text.trim();
    if (!trimmed) return { status: "failed" };
    if (!cwd) {
      onChooseProject();
      return { status: "failed" };
    }
    if (!settings.modelId) {
      const message = "模型列表尚未准备好，请稍后再试。";
      onError(message);
      return { status: "failed", error: message };
    }
    const thread = threadFor(recipient);
    if (
      sending.current ||
      thread?.activeTurn ||
      (thread?.activity && thread.activity !== "已停止")
    ) {
      return { status: "busy" };
    }
    const key = JSON.stringify([cwd, recipient?.id || "main", trimmed]);
    const requestId =
      requestIdOverride ||
      (retry.current?.key === key ? retry.current.id : crypto.randomUUID());
    if (!requestIdOverride) retry.current = { key, id: requestId };
    sending.current = true;
    setBusy(true);
    onError("");
    let accepted = false;
    try {
      await bridge.send({
        threadId: recipient?.threadId || thread?.id,
        viewId: recipient?.id,
        cwd,
        model: settings.modelId,
        effort: settings.effort,
        speed: settings.speed,
        permission: settings.permission,
        text: trimmed,
        requestId,
      });
      accepted = true;
      if (!requestIdOverride) retry.current = null;
      try {
        await Promise.all([refresh(), loadSpace(cwd)]);
      } catch (cause) {
        onError(`消息已发送，状态同步稍后重试：${errorText(cause)}`);
      }
      return { status: "sent" };
    } catch (cause) {
      const message = errorText(cause);
      if (/正在工作|工作中|working|active|busy/i.test(message)) {
        return { status: "busy" };
      }
      if (!accepted) onError(message);
      return { status: "failed", error: message };
    } finally {
      sending.current = false;
      setBusy(false);
    }
  }

  async function send(settings: Settings) {
    if (viewId && !targetView) {
      onError("这个 View 已不存在。");
      return;
    }
    const text = draft.trim();
    if (!text) return;
    if (sending.current) {
      if (enqueue(text, settings, targetView)) setDraft("");
      return;
    }
    if (isWorking(targetView)) {
      if (enqueue(text, settings, targetView)) setDraft("");
      return;
    }
    const result = await dispatch(text, settings, targetView);
    if (result.status === "busy") {
      if (enqueue(text, settings, targetView)) setDraft("");
      return;
    }
    if (result.status === "sent") setDraft("");
  }

  const routeEvent = (
    view: ViewRecord,
    event: { name: string; payload: unknown },
    settings: Settings,
  ) =>
    routeViewEvent({
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
    });

  async function guideQueue(item: QueuedMessage) {
    const key = queueKey(cwd, item.targetViewId);
    const recipient = space?.views.find((view) => view.id === item.targetViewId);
    if (item.targetViewId && !recipient) {
      onError("这个 View 已不存在，请编辑或删除这条消息。");
      return;
    }
    const steered = await steerIfActive(threadFor(recipient), cwd, item.text, async () =>
      Promise.all([refresh(), loadSpace(cwd)]),
    );
    if (steered?.status === "failed") {
      onError(steered.error || "引导消息失败，消息仍保留在队列中。");
      return;
    }
    updateQueue(key, (items) => items.filter((queued) => queued.id !== item.id));
    setViewId(item.targetViewId);
    setOpen(true);
    if (steered?.status === "sent") return;
    setDrafts((current) => ({ ...current, [key]: item.text }));
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLTextAreaElement>('textarea[name="message"]')
        ?.focus();
    });
  }

  function editQueue(id: string, text: string) {
    updateQueue(draftKey, (items) => editQueuedMessage(items, id, text));
  }

  function deleteQueue(id: string) {
    updateQueue(draftKey, (items) => removeQueuedMessage(items, id));
  }

  function reorderQueue(fromId: string, toId: string) {
    updateQueue(draftKey, (items) =>
      reorderQueuedMessages(items, fromId, toId),
    );
  }

  useEffect(() => {
    if (sending.current || drainLock.current) return;
    const entries = Object.entries(queues);
    for (const [key, items] of entries) {
      const item = items[0];
      if (!item || item.error) continue;
      const targetCwd = queuedWorkspace(key);
      if (!targetCwd || queuedThreadBusy(chats, targetCwd, item)) continue;
      drainLock.current = true;
      void deliverQueuedMessage(targetCwd, item, async () => {
        await refresh();
        if (currentPath.current === targetCwd) await loadSpace(targetCwd);
      })
        .then((result) => {
          if (result.status === "sent") {
            updateQueue(key, (current) =>
              current.filter((queued) => queued.id !== item.id),
            );
          } else if (result.status === "failed") {
            updateQueue(key, (current) =>
              current.map((queued) =>
                queued.id === item.id
                  ? {
                      ...queued,
                      error: result.error || "发送失败，请编辑后重试。",
                    }
                  : queued,
              ),
            );
          }
        })
        .finally(() => {
          drainLock.current = false;
        });
      break;
    }
  }, [cwd, queues, space, chats, mainChat, refresh, loadSpace]);

  async function stop() {
    if (!chat) return;
    const pauseMessage = "任务已停止，队列已暂停。编辑后可继续排队，或点击引导。";
    updateQueue(draftKey, (items) => items.map((item, index) =>
      index === 0 ? { ...item, error: pauseMessage } : item));
    try {
      await bridge.interrupt(chat.id);
      await refresh();
    } catch (cause) {
      updateQueue(draftKey, (items) => items.map((item) =>
        item.error === pauseMessage ? { ...item, error: undefined } : item));
      onError(errorText(cause));
    }
  }

  async function reply(
    request: AgentRequest,
    decision: string,
    answers?: Record<string, { answers: string[] }>,
    content?: Record<string, unknown> | null,
  ) {
    await bridge.reply({ id: request.id, decision, answers, content });
    await refresh();
  }
  return {
    chat,
    target,
    targetView,
    open,
    setOpen,
    openMain,
    openView,
    draft,
    setDraft,
    busy,
    queue,
    send,
    stop,
    reply,
    routeEvent,
    guideQueue,
    editQueue,
    deleteQueue,
    reorderQueue,
  };
}
