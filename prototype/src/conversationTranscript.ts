import type { Chat, TranscriptItem } from "./bridge.ts";

export function transcriptFor(chat: Chat | undefined): TranscriptItem[] {
  if (!chat) return [];
  if (chat.transcript?.length) return chat.transcript;
  return chat.messages.flatMap((message) => {
    let text = message.text;
    if (message.role === "assistant" && /^\s*(?:\{\s*"|```json)/.test(text)) {
      try {
        const value = JSON.parse(text.replace(/^\s*```json\s*/, "").replace(/\s*```\s*$/, ""));
        text = typeof value.response === "string" ? value.response : "";
      } catch { text = ""; }
    }
    if (!text.trim() || text.startsWith("Runtime integration pass:")) return [];
    return [{ id: message.id, turnId: "", kind: message.role, text,
      title: message.role === "user" ? "你" : "回复", status: "completed" }];
  });
}

export function currentActivityText(chat: Chat) {
  const items = transcriptFor(chat);
  const current = [...items].reverse().find((item) => item.kind !== "user" &&
    (item.status === "running" || (chat.activeTurn && item.turnId === chat.activeTurn)) && item.text.trim());
  if (current) return current.text;
  // Do not use a previous turn's final answer as this turn's progress.
  const detail = chat.activityDetail?.trim() || chat.turnPlan?.explanation || "";
  return detail.startsWith("{") || detail.startsWith("```json") ? "正在准备页面结果…" : detail;
}
