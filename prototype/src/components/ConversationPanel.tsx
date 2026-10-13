import type { ReactNode } from "react";
import { CaretDownIcon, CaretUpIcon, GitBranchIcon } from "@phosphor-icons/react";
import type { AgentRequest, Chat } from "../bridge";
import type { QueuedMessage } from "../conversationQueue";
import { isChatExecuting } from "./ExecutionStatus";
import { ConversationHistory } from "./ConversationHistory";
import { QueuedMessageList } from "./QueuedMessageList";
import { RequestCard } from "./RequestCard";

type Props = {
  chat: Chat | undefined;
  target?: "main" | "view";
  targetTitle?: string;
  open: boolean;
  composer: ReactNode;
  queue: QueuedMessage[];
  onToggle: () => void;
  onGuideQueue: (item: QueuedMessage) => void;
  onEditQueue: (id: string, text: string) => void;
  onDeleteQueue: (id: string) => void;
  onReorderQueue: (fromId: string, toId: string) => void;
  onReply: (
    request: AgentRequest,
    decision: string,
    answers?: Record<string, { answers: string[] }>,
    content?: Record<string, unknown> | null,
  ) => Promise<void>;
};

function pendingLabel(
  chat: Chat | undefined,
  target: "main" | "view",
  queueLength: number,
) {
  if (queueLength) return `${queueLength} 条消息排队中`;
  if (isChatExecuting(chat)) return chat?.activity || "Agent 正在执行";
  if (chat?.requests.length) return "需要你的确认才能继续";
  if (chat?.error) return "需要处理";
  return target === "view" ? "View Thread" : "Main Thread";
}

function ConversationNotices({
  chat,
  onReply,
}: Pick<Props, "chat" | "onReply">) {
  return (
    <>
      {chat?.requests.map((request) => (
        <RequestCard
          key={String(request.id)}
          request={request}
          onReply={(decision, answers, content) =>
            onReply(request, decision, answers, content)
          }
        />
      ))}
      {chat?.error && <div className="thread-error">{chat.error}</div>}
    </>
  );
}

export function ConversationPanel({
  chat,
  target = "main",
  targetTitle,
  open,
  composer,
  queue,
  onToggle,
  onGuideQueue,
  onEditQueue,
  onDeleteQueue,
  onReorderQueue,
  onReply,
}: Props) {
  const isView = target === "view" || chat?.role === "view";
  const roleLabel = isView ? "VIEW THREAD" : "MAIN THREAD";
  const roleName = isView ? "View Thread" : "Main Thread";
  return (
    <aside
      className={`conversation-dock${open ? " is-expanded" : " is-collapsed"}`}
      aria-label={`${roleName} 对话`}
    >
      <header className="conversation-panel-header">
        <button
          type="button"
          className="conversation-toggle"
          aria-label={open ? "收起对话" : "展开对话"}
          aria-expanded={open}
          onClick={onToggle}
        >
          <span className="conversation-heading">
            <span className="eyebrow">
              <GitBranchIcon size={13} /> {roleLabel}
            </span>
            <strong>
              {chat?.title ||
                targetTitle ||
                (isView ? "View Thread" : "Main Thread")}
            </strong>
          </span>
          {open ? <CaretDownIcon size={17} /> : <CaretUpIcon size={17} />}
        </button>
      </header>
      {!open && (queue.length > 0 || chat?.requests.length || chat?.error) && (
        <button
          type="button"
          className="conversation-pending"
          onClick={onToggle}
        >
          <span
            className={`conversation-pending-dot${chat?.requests.length ? " is-alert" : ""}${queue.length ? " is-queued" : ""}`}
          />
          <span>{pendingLabel(chat, target, queue.length)}</span>
          <CaretUpIcon size={14} />
        </button>
      )}
      {open && (
        <ConversationHistory key={chat?.id || targetTitle || target} chat={chat}>
          {queue.length > 0 && (
            <QueuedMessageList
              queue={queue}
              onGuide={onGuideQueue}
              onEdit={onEditQueue}
              onDelete={onDeleteQueue}
              onReorder={onReorderQueue}
            />
          )}
          <ConversationNotices chat={chat} onReply={onReply} />
        </ConversationHistory>
      )}
      {composer}
    </aside>
  );
}
