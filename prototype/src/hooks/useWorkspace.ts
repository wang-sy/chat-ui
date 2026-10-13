import { useCallback, useEffect, useRef, useState } from "react";
import { bridge, type AppState, type Space } from "../bridge";
import { newestState } from "../workspaceState";

const emptyState: AppState = {
  connected: false,
  available: false,
  issue: null,
  projects: [],
  chats: [],
  models: [],
  home: "",
};
export const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

export function useWorkspace() {
  const [state, setState] = useState<AppState>(emptyState);
  const [cwd, setCwd] = useState(
    () => localStorage.getItem("chat-ui:project") || "",
  );
  const [space, setSpace] = useState<Space | null>(null);
  const [error, setError] = useState("");
  const currentPath = useRef(cwd);
  currentPath.current = cwd;
  const connectAttempted = useRef(false);
  const mutationQueue = useRef(Promise.resolve());
  const streamVersion = useRef(0);
  const acceptState = useCallback((next: AppState, event = false) => {
    setState((current) => newestState(current, next, event));
  }, []);

  const acceptSpace = useCallback((next: Space) => {
    if (next.cwd !== currentPath.current) return;
    setSpace((current) =>
      current?.cwd === next.cwd && current.revision > next.revision
        ? current
        : next,
    );
  }, []);
  const refresh = useCallback(async () => {
    const version = streamVersion.current;
    const next = await bridge.state();
    if (version === streamVersion.current) acceptState(next);
    return next;
  }, [acceptState]);
  const loadSpace = useCallback(
    async (path: string) => {
      if (path) acceptSpace(await bridge.space(path));
    },
    [acceptSpace],
  );
  const reconnect = useCallback(async () => {
    connectAttempted.current = true;
    try {
      acceptState(await bridge.connect());
      setError("");
    } catch (cause) {
      setError(errorText(cause));
    }
  }, [acceptState]);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    async function tick() {
      try {
        const version = streamVersion.current;
        const next = await bridge.state();
        if (cancelled) return;
        if (version === streamVersion.current) acceptState(next);
        const projectExists = next.projects.some(
          (project) => project.path === cwd,
        );
        if (!projectExists) setCwd(next.projects[0]?.path || "");
        else await loadSpace(cwd);
        if (cancelled) return;
        if (next.available && !next.connected && !connectAttempted.current)
          await reconnect();
        if (!next.available) setError("未找到 Codex，请安装后重新连接。");
        else if (next.issue) setError(next.issue);
      } catch (cause) {
        if (!cancelled) setError(`无法连接 Chat UI 服务：${errorText(cause)}`);
      } finally {
        if (!cancelled)
          timer = setTimeout(() => void tick(), document.hidden ? 3000 : 900);
      }
    }
    void tick();
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [cwd, loadSpace, reconnect, acceptState]);

  useEffect(() => {
    let cancelled = false;
    let unsubscribe: (() => void) | undefined;
    let spaceTimer: ReturnType<typeof setTimeout> | undefined;
    const handleEvent = (event: {
      state: AppState;
      method: string;
    }) => {
      if (cancelled) return;
      streamVersion.current += 1;
      acceptState(event.state, true);
      clearTimeout(spaceTimer);
      spaceTimer = setTimeout(() => {
        const path = currentPath.current;
        if (path) void loadSpace(path);
      }, 80);
    };
    void bridge
      .subscribeEvents(handleEvent)
      .then((close) => {
        if (cancelled) close();
        else unsubscribe = close;
      })
      .catch(() => {
        // The regular state poll remains the compatibility fallback.
      });
    return () => {
      cancelled = true;
      clearTimeout(spaceTimer);
      unsubscribe?.();
    };
  }, [loadSpace, acceptState]);

  useEffect(() => {
    localStorage.setItem("chat-ui:project", cwd);
    setSpace((current) => (current?.cwd === cwd ? current : null));
  }, [cwd]);

  const mutateSpace = useCallback(
    (action: Record<string, unknown>) => {
      if (!cwd) return Promise.resolve();
      const operation = mutationQueue.current.then(async () => {
        try {
          acceptSpace(await bridge.mutateSpace({ cwd, ...action }));
        } catch (cause) {
          setError(errorText(cause));
        }
      });
      mutationQueue.current = operation;
      return operation;
    },
    [cwd, acceptSpace],
  );

  return {
    state,
    cwd,
    setCwd,
    space,
    error,
    setError,
    refresh,
    loadSpace,
    mutateSpace,
    reconnect,
  };
}
