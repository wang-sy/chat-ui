import { useCallback, useRef, useState } from "react";
import { bridge, type Directory } from "../bridge";
import { errorText } from "./useWorkspace";

export function useDirectoryBrowser(
  onSelected: (path: string) => Promise<void>,
) {
  const [open, setOpen] = useState(false);
  const [directory, setDirectory] = useState<Directory | null>(null);
  const [loading, setLoading] = useState(false);
  const [mutating, setMutating] = useState(false);
  const [error, setError] = useState("");
  const [history, setHistory] = useState({ paths: [] as string[], index: -1 });
  const historyRef = useRef(history);
  const sequence = useRef(0);
  const browsingPending = useRef(false);
  const mutationPending = useRef(false);

  function accept(next: Directory, index?: number) {
    const previous = historyRef.current;
    const unchanged = previous.paths[previous.index] === next.path;
    const paths = [...previous.paths.slice(0, previous.index + 1), next.path];
    const updated = index !== undefined
      ? { ...previous, index }
      : unchanged ? previous : { paths, index: paths.length - 1 };
    historyRef.current = updated;
    setHistory(updated);
    setDirectory(next);
  }

  async function browse(path?: string, index?: number) {
    if (mutationPending.current) return;
    const id = ++sequence.current;
    browsingPending.current = true;
    setLoading(true);
    setError("");
    try {
      const next = await bridge.directories(path);
      if (id === sequence.current) accept(next, index);
    } catch (cause) {
      if (id === sequence.current) setError(errorText(cause));
    } finally {
      if (id === sequence.current) {
        browsingPending.current = false;
        setLoading(false);
      }
    }
  }
  function show(path?: string) {
    if (mutationPending.current) return;
    historyRef.current = { paths: [], index: -1 };
    setHistory(historyRef.current);
    setDirectory(null);
    setOpen(true);
    void browse(path);
  }
  async function choose(path = directory?.path) {
    if (!path || browsingPending.current || mutationPending.current) return;
    mutationPending.current = true;
    setMutating(true);
    setError("");
    try {
      const added = await bridge.addProject(path);
      await onSelected(added.path);
      setOpen(false);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      mutationPending.current = false;
      setMutating(false);
    }
  }
  async function create(name: string) {
    if (!directory || browsingPending.current || mutationPending.current) return false;
    mutationPending.current = true;
    setMutating(true);
    setError("");
    try {
      accept(await bridge.createDirectory(directory.path, name));
      return true;
    } finally {
      mutationPending.current = false;
      setMutating(false);
    }
  }
  const close = useCallback(() => {
    if (mutationPending.current) return;
    ++sequence.current;
    browsingPending.current = false;
    setLoading(false);
    setOpen(false);
  }, []);

  return {
    open,
    directory,
    loading,
    mutating,
    error,
    browse,
    show,
    choose,
    create,
    canBack: history.index > 0,
    canForward: history.index < history.paths.length - 1,
    back: () => void browse(history.paths[history.index - 1], history.index - 1),
    forward: () => void browse(history.paths[history.index + 1], history.index + 1),
    close,
  };
}
