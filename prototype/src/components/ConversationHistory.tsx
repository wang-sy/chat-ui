import { useState, type ReactNode } from "react";
import { ArrowDownIcon, BrainIcon, CaretRightIcon, CheckIcon, CircleNotchIcon,
  TerminalWindowIcon, WrenchIcon, WarningCircleIcon } from "@phosphor-icons/react";
import type { Chat, TranscriptItem } from "../bridge";
import { transcriptFor } from "../conversationTranscript";
import { useTranscriptScroll } from "../hooks/useTranscriptScroll";
import { isChatExecuting } from "./ExecutionStatus";
import { MessageText } from "./MessageText";

function ProcessItem({ item }: { item: TranscriptItem }) {
  const [expanded, setExpanded] = useState(item.status === "running");
  const running = item.status === "running";
  const failed = item.status === "failed" || item.status === "interrupted";
  const Icon = item.kind === "command" ? TerminalWindowIcon : item.kind === "reasoning" ? BrainIcon : WrenchIcon;
  return <div className={`transcript-process${failed ? " is-failed" : ""}`}>
    <button type="button" className="transcript-process-toggle" onClick={() => setExpanded(!expanded)}
      aria-expanded={expanded} aria-label={`${expanded ? "收起" : "展开"}${item.title}`}>
      {running ? <CircleNotchIcon size={14} className="spin" /> : failed ? <WarningCircleIcon size={14} /> : <Icon size={14} />}
      <span className="transcript-process-title">{item.title}</span>
      <span className="transcript-process-state">{running ? "进行中" : failed ? (item.status === "interrupted" ? "已停止" : "失败") : <CheckIcon size={12} />}</span>
      <CaretRightIcon size={12} className={expanded ? "is-open" : ""} />
    </button>
    {!expanded && running && item.text && <p className="transcript-process-preview">{item.text.slice(-180)}</p>}
    {expanded && (item.text ? <pre className="transcript-process-output">{item.text}</pre> :
      <p className="transcript-process-preview">{running ? "等待输出…" : "这一步已结束，没有文本输出。"}</p>)}
  </div>;
}

export function ConversationHistory({ chat, children }: { chat: Chat | undefined; children: ReactNode }) {
  const items = transcriptFor(chat);
  const scroll = useTranscriptScroll(chat?.id, chat);
  const executing = isChatExecuting(chat);
  const hasRunning = items.some((item) => item.status === "running");
  return <div className="conversation-history">
    <div ref={scroll.viewport} onScroll={scroll.onScroll} className="conversation-scroll"
      role="region" aria-label="对话历史" tabIndex={0}>
      <div ref={scroll.content} className="transcript-content">
        {items.map((item) => item.kind === "user" || item.kind === "assistant" ? (
          item.text.trim() ? <article key={item.id} className={`transcript-message is-${item.kind}`}>
            <div className="transcript-message-author">{item.kind === "user" ? (item.text.startsWith("Main Thread 分派任务") ? "Main Thread 分派" : "你") : chat?.role === "view" ? "View Thread" : "Main Thread"}</div>
            <MessageText text={item.text} />
            {item.status === "running" && <span className="transcript-streaming" aria-label="正在流式输出" />}
          </article> : null
        ) : item.kind === "reasoning" && !item.text && item.status !== "running" ? null : <ProcessItem item={item} key={item.id} />)}
        {items.length === 0 && !executing && <p className="transcript-empty">这个 Thread 的对话会保留在这里。</p>}
        {executing && (!hasRunning || !items.some((item) => item.status === "running" && item.text.trim())) &&
          <p className="transcript-waiting" role="status"><CircleNotchIcon className="spin" size={14} />{chat?.activity || "正在处理…"}</p>}
        {children}
      </div>
    </div>
    {scroll.away && <button className="transcript-latest" type="button" onClick={scroll.toLatest}><ArrowDownIcon size={13} />回到最新</button>}
  </div>;
}
