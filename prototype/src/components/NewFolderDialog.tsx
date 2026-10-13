import { useEffect, useRef, useState } from "react";
import { FolderPlusIcon } from "@phosphor-icons/react";
import { useDialogFocus } from "../hooks/useDialogFocus";

type Props = {
  parent: string;
  busy: boolean;
  onCreate: (name: string) => Promise<boolean>;
  onClose: () => void;
};

export function NewFolderDialog({ parent, busy, onCreate, onClose }: Props) {
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const ref = useRef<HTMLElement>(null);
  useDialogFocus(ref, true);
  useEffect(() => {
    if (error) {
      const input = ref.current?.querySelector("input");
      input?.focus();
      input?.select();
    }
  }, [error]);
  async function submit() {
    if (!name.trim() || busy) return;
    setError("");
    try {
      if (await onCreate(name.trim())) onClose();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }
  return (
    <div className="folder-sheet-backdrop">
      <section ref={ref} className="folder-sheet" role="dialog" aria-modal="true"
        aria-labelledby="new-folder-title" tabIndex={-1}
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Escape" && !busy) onClose();
        }}>
        <FolderPlusIcon size={26} className="folder-sheet-icon" />
        <h3 id="new-folder-title">新建文件夹</h3>
        <p className="folder-sheet-location" title={parent}>位置：{parent}</p>
        <form onSubmit={(event) => { event.preventDefault(); void submit(); }}>
          <label htmlFor="new-folder-name">名称</label>
          <input id="new-folder-name" data-initial-focus autoComplete="off"
            value={name} disabled={busy} placeholder="未命名文件夹"
            aria-invalid={Boolean(error)} aria-describedby={error ? "new-folder-error" : undefined}
            onChange={(event) => { setName(event.target.value); setError(""); }} />
          {error && <p id="new-folder-error" className="inline-error" role="alert">{error}</p>}
          <div className="folder-sheet-actions">
            <button type="button" className="directory-secondary" disabled={busy} onClick={onClose}>取消</button>
            <button type="submit" className="primary-button" disabled={!name.trim() || busy}>
              {busy ? "正在创建…" : "创建"}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}
