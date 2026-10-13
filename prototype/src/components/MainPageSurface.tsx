import { useEffect, useMemo, useRef, useState } from "react";
import { StackIcon } from "@phosphor-icons/react";
import type { Space, ViewRecord } from "../bridge";
import { compositionDocument } from "../pageRuntime/document";
import { safePayload, surfaceEvent, viewCatalog } from "../pageRuntime/protocol";

type Props = {
  space: Space;
  selectedViewId: string | null;
  onOpenView: (view: ViewRecord) => void;
  onViewEvent: (
    view: ViewRecord,
    event: { name: string; payload: unknown },
  ) => void;
};

/**
 * The Space surface is deliberately page-first. Main Thread owns the main
 * page manifest; selecting a View Thread swaps this one iframe to that View's
 * page. There is no second visual graph competing with the generated page.
 */
export function MainPageSurface({
  space,
  selectedViewId,
  onOpenView,
  onViewEvent,
}: Props) {
  const selectedView = space.views.find((view) => view.id === selectedViewId);
  const html = selectedView ? selectedView.html.trim() : space.mainPage.html.trim();
  const title = selectedView
    ? `${selectedView.title} View`
    : space.mainPage.title || "Main Page";
  const frame = useRef<HTMLIFrameElement>(null);
  const channel = useRef(crypto.randomUUID()).current;
  const viewsRef = useRef(space.views);
  viewsRef.current = space.views;
  const openRef = useRef(onOpenView);
  openRef.current = onOpenView;
  const catalog = JSON.stringify(viewCatalog(space));
  const catalogRef = useRef(catalog);
  catalogRef.current = catalog;
  const document = useMemo(() => selectedView ? html : compositionDocument(html, channel),
    [html, channel, selectedView?.id]);
  const viewRef = useRef(selectedView);
  const onViewEventRef = useRef(onViewEvent);
  const [loaded, setLoaded] = useState(false);
  viewRef.current = selectedView;
  onViewEventRef.current = onViewEvent;

  useEffect(() => setLoaded(false), [html, selectedView?.id]);

  const sendCatalog = () => frame.current?.contentWindow?.postMessage({
    type: "chat-ui:catalog", channel, views: JSON.parse(catalogRef.current),
  }, "*");

  useEffect(() => { if (!selectedView) sendCatalog(); }, [catalog, selectedView?.id]);

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      const view = viewRef.current;
      if (event.source !== frame.current?.contentWindow) return;
      const data = event.data;
      if (!view) {
        if (data?.type === "chat-ui:surface" && data.channel === channel && data.event === "ready") {
          sendCatalog();
          return;
        }
        const action = surfaceEvent(data, channel, viewsRef.current);
        if (action?.navigate) openRef.current(action.view);
        else if (action) onViewEventRef.current(action.view, action.event);
        return;
      }
      if (
        !data ||
        typeof data !== "object" ||
        data.type !== "chat-ui:view-event" ||
        typeof data.event !== "string"
      )
        return;
      const payload = safePayload(data.payload);
      if (payload === undefined) return;
      onViewEventRef.current(view, {
        name: data.event.slice(0, 80),
        payload,
      });
    };
    window.addEventListener("message", receive);
    return () => window.removeEventListener("message", receive);
  }, []);

  return (
    <section
      className="space-surface"
      aria-label={selectedView ? `${selectedView.title} 页面` : "Space 主页面"}
    >
      {html || (!selectedView && space.views.length > 0) ? (
        <div className="main-page-frame">
          <iframe
            ref={frame}
            className="main-page-surface"
            title={title}
            sandbox="allow-scripts allow-forms"
            srcDoc={document}
            onLoad={() => { setLoaded(true); if (!selectedView) sendCatalog(); }}
          />
          {!loaded && (
            <div className="main-page-loading" role="status">
              正在加载页面…
            </div>
          )}
        </div>
      ) : (
        <div className="page-empty-state" aria-label="Space 页面">
          <span className="page-empty-icon">
            <StackIcon size={22} />
          </span>
          <strong>{selectedView ? `正在等待「${selectedView.title}」` : "等待 Main Thread 页面"}</strong>
          <span>{selectedView ? "生成进度显示在下方，完成后自动展示" : "发送一个任务，让 Main Agent 直接生成 Space 页面"}</span>
        </div>
      )}
    </section>
  );
}
