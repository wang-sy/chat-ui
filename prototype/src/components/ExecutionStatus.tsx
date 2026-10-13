import {
  BrainIcon,
  CaretUpIcon,
  ChatCircleTextIcon,
  CircleNotchIcon,
  FileCodeIcon,
  TerminalWindowIcon,
} from "@phosphor-icons/react";
import type { Chat } from "../bridge";
import { currentActivityText } from "../conversationTranscript";

type Props = { chat: Chat | undefined };

type RunningProps = Props & { onOpen?: () => void };

export function isChatExecuting(chat: Chat | undefined) {
  return Boolean(
    chat?.activeTurn ||
      (chat?.activity && chat.activity !== "已停止"),
  );
}

function previewText(text: string) {
  const trimmed = text.trim();
  if (!trimmed) return "";
  return trimmed.length > 220 ? `${trimmed.slice(-220)}…` : trimmed;
}

function runningDetail(chat: Chat) {
  const detail = previewText(currentActivityText(chat));
  if (detail) return detail;
  if (chat.turnPlan?.explanation) return chat.turnPlan.explanation;
  return "Codex 正在处理当前任务…";
}

function activityIcon(activity: string) {
  if (activity.includes("命令")) return <TerminalWindowIcon size={14} />;
  if (activity.includes("文件")) return <FileCodeIcon size={14} />;
  if (activity.includes("思考") || activity.includes("规划") || activity.includes("推理")) {
    return <BrainIcon size={14} />;
  }
  if (activity.includes("回复")) return <ChatCircleTextIcon size={14} />;
  return <CircleNotchIcon className="spin" size={14} />;
}

/** A small Codex-style execution rail that stays attached to the page. */
export function RunningStatus({ chat, onOpen }: RunningProps) {
  if (!chat || !isChatExecuting(chat)) return null;
  const steps = chat.turnPlan?.plan || [];
  const completed = steps.filter((step) => step.status === "completed").length;
  const role = chat.role === "view" ? "View Thread" : "Main Thread";
  return (
    <button
      type="button"
      className="page-running-status"
      aria-label={`${role}：${chat.activity || "正在工作"}`}
      onClick={onOpen}
    >
      <span className="page-running-icon" aria-hidden="true">
        {activityIcon(chat.activity || "正在工作")}
      </span>
      <span className="page-running-copy">
        <strong>{chat.activity || "正在工作"}</strong>
        <span>{runningDetail(chat)}</span>
      </span>
      {steps.length > 0 && (
        <span className="page-running-progress">
          {completed}/{steps.length}
        </span>
      )}
      <span className="page-running-role">{role}</span>
      <CaretUpIcon size={14} aria-hidden="true" />
    </button>
  );
}
