import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import {
  ArrowLeftIcon, ArrowRightIcon, ArrowUpIcon, CaretRightIcon,
  FolderIcon, FolderPlusIcon, HouseIcon, XIcon,
} from "@phosphor-icons/react";
import type { Directory } from "../bridge";
import { useDialogFocus } from "../hooks/useDialogFocus";
import { NewFolderDialog } from "./NewFolderDialog";

type Props = {
  open: boolean;
  directory: Directory | null;
  loading: boolean;
  mutating: boolean;
  error: string;
  canBack: boolean;
  canForward: boolean;
  onBack: () => void;
  onForward: () => void;
  onBrowse: (path?: string) => void;
  onCreate: (name: string) => Promise<boolean>;
  onChoose: (path?: string) => void;
  onClose: () => void;
};

export function DirectoryDialog({
  open, directory, loading, mutating, error, canBack, canForward,
  onBack, onForward, onBrowse, onCreate, onChoose, onClose,
}: Props) {
  const [path, setPath] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [newFolder, setNewFolder] = useState(false);
  const dialogRef = useRef<HTMLElement>(null);
  const pathRef = useRef<HTMLInputElement>(null);
  const createRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const busy = loading || mutating;
  const pathChanged = path !== (directory?.path || "");
  const validLocation = Boolean(directory) && !busy && !pathChanged && !error;
  useDialogFocus(dialogRef, open);
  useEffect(() => {
    setPath(directory?.path || "");
    setSelected(null);
  }, [directory]);
  useEffect(() => {
    if (!open) setNewFolder(false);
  }, [open]);
  useEffect(() => {
    if (open && !busy && !newFolder && document.activeElement === document.body) {
      (error ? pathRef.current : listRef.current)?.focus();
    }
  }, [open, busy, newFolder, error]);
  function closeNewFolder() {
    setNewFolder(false);
    requestAnimationFrame(() => createRef.current?.focus());
  }
  function openFolder(path?: string) {
    if (!busy) { setSelected(null); onBrowse(path); }
  }
  function moveSelection(event: KeyboardEvent<HTMLDivElement>) {
    const children = directory?.children || [];
    if (!children.length || busy) return;
    const index = children.findIndex((child) => child.path === selected);
    const next = event.key === "ArrowDown" ? Math.min(index + 1, children.length - 1)
      : event.key === "ArrowUp" ? Math.max(index - 1, 0)
      : event.key === "Home" ? 0
      : event.key === "End" ? children.length - 1 : null;
    if (next === null) return;
    event.preventDefault();
    setSelected(children[next].path);
    listRef.current?.querySelectorAll<HTMLButtonElement>('[role="option"]')[next]?.focus();
  }
  if (!open) return null;
  const chosen = selected || directory?.path;
  return (
    <div className="modal-backdrop" onMouseDown={(event) => {
      if (event.target === event.currentTarget && !mutating && !newFolder) onClose();
    }}>
      <section ref={dialogRef} className="directory-dialog" role="dialog" aria-modal="true"
        aria-labelledby="directory-dialog-title" tabIndex={-1}
        onKeyDown={(event) => {
          if (newFolder) return;
          if (event.key === "Escape" && !mutating) { event.preventDefault(); onClose(); }
          if (!(event.metaKey || event.ctrlKey)) return;
          if (event.shiftKey && event.key.toLowerCase() === "n") {
            event.preventDefault();
            if (validLocation) setNewFolder(true);
          } else if (event.shiftKey && event.key.toLowerCase() === "g") {
            event.preventDefault(); pathRef.current?.focus(); pathRef.current?.select();
          } else if (event.key === "ArrowUp" && directory?.parent) {
            event.preventDefault(); openFolder(directory.parent);
          } else if (event.key === "[" && canBack && !busy) {
            event.preventDefault(); onBack();
          } else if (event.key === "]" && canForward && !busy) {
            event.preventDefault(); onForward();
          }
        }}>
        <div inert={newFolder} aria-hidden={newFolder || undefined}>
          <header>
            <div>
              <h2 id="directory-dialog-title">选择 Space 文件夹</h2>
              <p>选择已有文件夹，或新建一个。</p>
            </div>
            <button type="button" aria-label="关闭" disabled={mutating} onClick={onClose}><XIcon size={18} /></button>
          </header>
          <form className="directory-path" onSubmit={(event) => { event.preventDefault(); openFolder(path.trim()); }}>
            <button type="button" aria-label="后退" title="后退（⌘[）" disabled={!canBack || busy} onClick={onBack}><ArrowLeftIcon size={16} /></button>
            <button type="button" aria-label="前进" title="前进（⌘]）" disabled={!canForward || busy} onClick={onForward}><ArrowRightIcon size={16} /></button>
            <button type="button" aria-label="上一级" title="上一级（⌘↑）" disabled={!directory?.parent || busy}
              onClick={() => openFolder(directory?.parent || undefined)}><ArrowUpIcon size={16} /></button>
            <input ref={pathRef} aria-label="文件夹路径" title="前往文件夹（⌘⇧G）" name="directory-path"
              value={path} disabled={mutating} autoComplete="off" spellCheck={false}
              onChange={(event) => setPath(event.target.value)} />
            <button type="submit" className="path-go" disabled={busy || !path.trim()}>前往</button>
          </form>
          <nav className="directory-roots" aria-label="常用位置">
            {directory?.roots.map((root) => (
              <button type="button" key={root.path} disabled={busy} aria-current={directory.path === root.path ? "location" : undefined}
                onClick={() => openFolder(root.path)}><HouseIcon size={15} />{root.name}</button>
            ))}
          </nav>
          <div ref={listRef} className="directory-list" role="listbox" aria-label="文件夹" aria-busy={busy}
            data-initial-focus tabIndex={0} onKeyDown={moveSelection}>
            {loading && <div className="directory-loading" role="status">正在读取文件夹…</div>}
            {!loading && directory?.children.map((child) => (
              <button type="button" role="option" aria-selected={selected === child.path}
                key={child.path} tabIndex={selected === child.path ? 0 : -1} disabled={mutating}
                onClick={() => setSelected(child.path)} onDoubleClick={() => openFolder(child.path)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === "ArrowRight") {
                    event.preventDefault(); openFolder(child.path);
                  }
                }}>
                <FolderIcon size={19} /><span>{child.name}</span><CaretRightIcon size={12} />
              </button>
            ))}
            {!loading && directory && directory.children.length === 0 && (
              <div className="directory-empty"><FolderIcon size={28} /><span>此文件夹中没有子文件夹</span><small>可以直接选择，或新建文件夹</small></div>
            )}
          </div>
          <div className="directory-selection" title={chosen} aria-live="polite">
            {error ? <span className="inline-error" role="alert">{error}</span>
              : <><span>{selected ? "已选择" : "当前位置"}</span><span>{chosen || "正在读取…"}</span></>}
          </div>
          <footer>
            <button ref={createRef} type="button" className="directory-secondary directory-new-folder"
              title="新建文件夹（⌘⇧N）" disabled={!validLocation} onClick={() => setNewFolder(true)}>
              <FolderPlusIcon size={16} />新建文件夹
            </button>
            <button type="button" className="directory-secondary" disabled={mutating} onClick={onClose}>取消</button>
            <button type="button" className="primary-button" disabled={!validLocation}
              onClick={() => onChoose(chosen)}>{mutating ? "正在添加…" : "选择文件夹"}</button>
          </footer>
        </div>
        {newFolder && directory && <NewFolderDialog parent={directory.path} busy={mutating} onCreate={onCreate} onClose={closeNewFolder} />}
      </section>
    </div>
  );
}
