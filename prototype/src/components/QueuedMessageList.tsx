import { useEffect, useRef, useState } from "react";
import {
  ArrowBendUpLeftIcon,
  CheckIcon,
  DotsSixVerticalIcon,
  DotsThreeIcon,
  PencilSimpleIcon,
  TrashIcon,
  WarningCircleIcon,
  XIcon,
} from "@phosphor-icons/react";
import type { QueuedMessage } from "../conversationQueue";

type Props = {
  queue: QueuedMessage[];
  onGuide: (item: QueuedMessage) => void;
  onEdit: (id: string, text: string) => void;
  onDelete: (id: string) => void;
  onReorder: (fromId: string, toId: string) => void;
};

export function QueuedMessageList({
  queue,
  onGuide,
  onEdit,
  onDelete,
  onReorder,
}: Props) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState("");
  const [moreId, setMoreId] = useState<string | null>(null);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const pointerDrag = useRef<{ id: string; x: number; y: number } | null>(null);

  useEffect(() => {
    if (!moreId) return;
    function closeMenu(event: PointerEvent) {
      const target = event.target;
      if (target instanceof Element && !target.closest(".queue-more-wrap")) {
        setMoreId(null);
      }
    }
    document.addEventListener("pointerdown", closeMenu);
    return () => document.removeEventListener("pointerdown", closeMenu);
  }, [moreId]);

  function beginEdit(item: QueuedMessage) {
    setEditingId(item.id);
    setEditingText(item.text);
    setMoreId(null);
  }

  function saveEdit() {
    if (!editingId || !editingText.trim()) return;
    onEdit(editingId, editingText.trim());
    setEditingId(null);
    setEditingText("");
  }

  function cancelEdit() {
    setEditingId(null);
    setEditingText("");
  }

  function moveBy(index: number, offset: number) {
    const target = queue[index + offset];
    if (target) onReorder(queue[index].id, target.id);
  }

  return (
    <section className="conversation-queue" aria-label="排队消息">
      <div className="conversation-queue-heading">
        <span>排队中</span>
        <span>{queue.length}</span>
        <span className="conversation-queue-hint">当前任务完成后自动发送</span>
      </div>
      <div className="conversation-queue-list">
        {queue.map((item, index) => {
          const editing = editingId === item.id;
          return (
            <article
              className={`queued-message${draggingId === item.id ? " is-dragging" : ""}`}
              key={item.id}
              data-queue-id={item.id}
              draggable={!editing}
              onDragStart={(event) => {
                if (editing) return;
                setDraggingId(item.id);
                event.dataTransfer.effectAllowed = "move";
                event.dataTransfer.setData("text/plain", item.id);
              }}
              onDragEnd={() => setDraggingId(null)}
              onDragOver={(event) => {
                if (draggingId && draggingId !== item.id) event.preventDefault();
              }}
              onDrop={(event) => {
                event.preventDefault();
                const fromId = event.dataTransfer.getData("text/plain") || draggingId;
                if (fromId && fromId !== item.id) onReorder(fromId, item.id);
                setDraggingId(null);
              }}
            >
              <button
                type="button"
                className="queue-drag-handle"
                aria-label="拖动排序"
                title="拖动排序"
                tabIndex={editing ? -1 : 0}
                style={{ touchAction: "none" }}
                onPointerDown={(event) => {
                  if (editing || event.button !== 0) return;
                  event.preventDefault();
                  event.currentTarget.setPointerCapture(event.pointerId);
                  pointerDrag.current = { id: item.id, x: event.clientX, y: event.clientY };
                }}
                onPointerMove={(event) => {
                  const drag = pointerDrag.current;
                  if (drag && Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > 4) {
                    setDraggingId(drag.id);
                  }
                }}
                onPointerUp={(event) => {
                  const drag = pointerDrag.current;
                  const moved = drag && Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > 4;
                  const target = document.elementFromPoint(event.clientX, event.clientY)
                    ?.closest<HTMLElement>("[data-queue-id]")?.dataset.queueId;
                  if (moved && target && target !== drag.id) onReorder(drag.id, target);
                  pointerDrag.current = null;
                  setDraggingId(null);
                }}
                onPointerCancel={() => {
                  pointerDrag.current = null;
                  setDraggingId(null);
                }}
                onKeyDown={(event) => {
                  if (event.key === "ArrowUp") {
                    event.preventDefault();
                    moveBy(index, -1);
                  }
                  if (event.key === "ArrowDown") {
                    event.preventDefault();
                    moveBy(index, 1);
                  }
                }}
              >
                <DotsSixVerticalIcon size={17} weight="bold" />
              </button>
              <div className="queued-message-body">
                {editing ? (
                  <div className="queue-edit-area">
                    <textarea
                      name={`queued-${item.id}`}
                      value={editingText}
                      rows={3}
                      autoFocus
                      onChange={(event) => setEditingText(event.target.value)}
                      onKeyDown={(event) => {
                        if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                          event.preventDefault();
                          saveEdit();
                        }
                        if (event.key === "Escape") cancelEdit();
                      }}
                    />
                    <div className="queue-edit-actions">
                      <button type="button" onClick={cancelEdit}>
                        <XIcon size={13} />
                        取消
                      </button>
                      <button type="button" className="is-primary" onClick={saveEdit}>
                        <CheckIcon size={13} />
                        保存
                      </button>
                    </div>
                  </div>
                ) : (
                  <>
                    <p className="queued-message-text">{item.text}</p>
                    <p className="queue-message-meta">
                      {item.targetTitle ? `发送给 ${item.targetTitle}` : "发送给 Main Thread"}
                    </p>
                    {item.error && (
                      <p className="queue-error" role="alert">
                        <WarningCircleIcon size={12} />
                        {item.error}
                      </p>
                    )}
                  </>
                )}
              </div>
              {!editing && (
                <div className="queue-message-actions">
                  <button
                    type="button"
                    className="queue-action queue-guide"
                    aria-label="引导这条消息"
                    title="引导这条消息"
                    onClick={() => onGuide(item)}
                  >
                    <ArrowBendUpLeftIcon size={15} />
                    <span>引导</span>
                  </button>
                  <button
                    type="button"
                    className="queue-action"
                    aria-label="删除这条消息"
                    title="删除这条消息"
                    onClick={() => onDelete(item.id)}
                  >
                    <TrashIcon size={15} />
                  </button>
                  <div className="queue-more-wrap">
                    <button
                      type="button"
                      className="queue-action"
                      aria-label="更多队列操作"
                      aria-expanded={moreId === item.id}
                      aria-haspopup="menu"
                      title="更多操作"
                      onClick={() => setMoreId((current) => (current === item.id ? null : item.id))}
                    >
                      <DotsThreeIcon size={16} weight="bold" />
                    </button>
                    {moreId === item.id && (
                      <div className="queue-more-menu" role="menu">
                        <button type="button" role="menuitem" onClick={() => beginEdit(item)}>
                          <PencilSimpleIcon size={14} />
                          编辑
                        </button>
                        {index > 0 && (
                          <button
                            type="button"
                            role="menuitem"
                            onClick={() => {
                              onReorder(item.id, queue[0].id);
                              setMoreId(null);
                            }}
                          >
                            移到最前
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </article>
          );
        })}
      </div>
    </section>
  );
}
