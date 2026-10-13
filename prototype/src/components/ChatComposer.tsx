import { useRef, useState } from "react";
import { ArrowUpIcon, PlusIcon, StopIcon } from "@phosphor-icons/react";
import type { Model, PermissionMode, ViewRecord } from "../bridge";
import {
  ModelPicker,
  PermissionPicker,
  type ModelSpeed,
} from "./ComposerPickers";

type Props = {
  models: Model[];
  model: Model | undefined;
  effort: string;
  speed: ModelSpeed;
  permission: PermissionMode;
  draft: string;
  busy: boolean;
  activeTurn: boolean;
  running: boolean;
  selectedView: ViewRecord | undefined;
  permissionOpen: boolean;
  onDraft: (value: string) => void;
  onModel: (value: string) => void;
  onEffort: (value: string) => void;
  onSpeed: (value: ModelSpeed) => void;
  onPermissionOpen: (open: boolean) => void;
  onPermission: (value: PermissionMode) => void;
  onSend: () => void;
  onStop: () => void;
};

export function ChatComposer({
  models,
  model,
  effort,
  speed,
  permission,
  draft,
  busy,
  activeTurn,
  running,
  selectedView,
  permissionOpen,
  onDraft,
  onModel,
  onEffort,
  onSpeed,
  onPermissionOpen,
  onPermission,
  onSend,
  onStop,
}: Props) {
  const fileInput = useRef<HTMLInputElement>(null);
  const draftRef = useRef(draft);
  const [attachmentError, setAttachmentError] = useState("");
  draftRef.current = draft;

  async function attach(file: File | undefined) {
    if (!file) return;
    if (file.size > 2_000_000) {
      setAttachmentError("附件过大，请选择 2 MB 以内的文本文件。");
      return;
    }
    try {
      const text = await file.text();
      if (text.includes("\u0000")) {
        setAttachmentError("这个附件不是文本文件，请选择可读的文本内容。");
        return;
      }
      setAttachmentError("");
      const currentDraft = draftRef.current;
      onDraft(
        `${currentDraft}${currentDraft ? "\n\n" : ""}【${file.name}】\n${text.slice(0, 30000)}`,
      );
    } catch {
      setAttachmentError("无法读取这个附件，请换一个文本文件。");
    }
  }

  return (
    <form
      className="composer-wrap"
      onSubmit={(event) => {
        event.preventDefault();
        onSend();
      }}
    >
      <div className="composer-card">
        <textarea
          name="message"
          value={draft}
          onChange={(event) => onDraft(event.target.value)}
          placeholder={
            selectedView ? `给 ${selectedView.title} 提交工作…` : "随心输入"
          }
          aria-label={
            selectedView
              ? `向 ${selectedView.title} View Thread 输入`
              : "向 Main Thread 输入"
          }
          rows={2}
          onKeyDown={(event) => {
            if (
              event.key === "Enter" &&
              !event.shiftKey &&
              !event.nativeEvent.isComposing &&
              event.keyCode !== 229
            ) {
              event.preventDefault();
              onSend();
            }
          }}
        />
        <div className="composer-toolbar">
          <input
            ref={fileInput}
            name="attachment"
            type="file"
            accept=".txt,.md,.json,.csv,.js,.jsx,.ts,.tsx,.css,.html,.rs,.py,.yaml,.yml"
            hidden
            onChange={(event) => {
              void attach(event.target.files?.[0]);
              event.currentTarget.value = "";
            }}
          />
          <button
            type="button"
            className="composer-icon-button"
            aria-label="添加文件"
            title="添加文件"
            onClick={() => fileInput.current?.click()}
          >
            <PlusIcon size={22} />
          </button>
          <PermissionPicker
            value={permission}
            open={permissionOpen}
            onOpenChange={onPermissionOpen}
            onChange={onPermission}
          />
          <span className="composer-spacer" />
          <ModelPicker
            models={models}
            model={model}
            effort={effort}
            speed={speed}
            onModel={onModel}
            onEffort={onEffort}
            onSpeed={onSpeed}
          />
          {activeTurn && (
            <button
              type="button"
              className="stop-button"
              aria-label="停止当前任务"
              title="停止当前任务"
              onClick={onStop}
            >
              <StopIcon size={15} weight="fill" />
            </button>
          )}
          <button
            type="submit"
            className="send-button"
            aria-label={activeTurn || running ? "加入队列" : "发送"}
            title={activeTurn || running ? "加入队列" : "发送"}
            disabled={!draft.trim() || busy}
          >
            <ArrowUpIcon size={20} weight="bold" />
          </button>
        </div>
      </div>
      {attachmentError && (
        <p className="inline-error composer-attachment-error" role="alert">
          {attachmentError}
        </p>
      )}
      {busy && !activeTurn && (
        <div className="composer-status">
          正在连接 {selectedView ? "View Thread" : "Main Thread"}…
        </div>
      )}
    </form>
  );
}
