import { useLayoutEffect, useRef, useState } from "react";

export function useTranscriptScroll(threadId: string | undefined, revision: unknown) {
  const viewport = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  const [away, setAway] = useState(false);
  function toLatest() {
    pinned.current = true;
    setAway(false);
    if (viewport.current) viewport.current.scrollTop = viewport.current.scrollHeight;
  }
  function onScroll() {
    const node = viewport.current;
    if (!node) return;
    pinned.current = node.scrollHeight - node.clientHeight - node.scrollTop < 64;
    setAway(!pinned.current);
  }
  useLayoutEffect(toLatest, [threadId]);
  useLayoutEffect(() => { if (pinned.current) toLatest(); }, [revision]);
  useLayoutEffect(() => {
    if (!content.current) return;
    const observer = new ResizeObserver(() => { if (pinned.current) toLatest(); });
    observer.observe(content.current);
    return () => observer.disconnect();
  }, []);
  return { viewport, content, away, onScroll, toLatest };
}
