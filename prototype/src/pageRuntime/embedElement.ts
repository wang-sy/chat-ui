import type { EmbeddedView } from "./protocol";

export type EmbedSlot = {
  frame: HTMLIFrameElement;
  update: () => void;
  resize: (height: unknown) => void;
};

/** Serialized with the parent runtime: all runtime dependencies are explicit arguments. */
export function createEmbedSlot(
  element: HTMLElement,
  resolve: (reference: string | null) => EmbeddedView | undefined,
  onLoad: (frame: HTMLIFrameElement) => void,
  sizeReporter: () => void,
): EmbedSlot {
  let html = "";
  const frame = document.createElement("iframe");
  frame.className = "chat-view-frame";
  frame.setAttribute("sandbox", "allow-scripts allow-forms");
  frame.onload = () => onLoad(frame);
  const empty = document.createElement("div");
  empty.className = "chat-view-empty";
  empty.setAttribute("role", "status");
  element.replaceChildren(frame, empty);
  const update = () => {
    const view = resolve(element.getAttribute("view-id"));
    const label = view?.title || element.getAttribute("view-id") || "View";
    element.dataset.viewId = view?.id || "";
    element.dataset.viewStatus = view?.status || "missing";
    element.setAttribute("aria-busy", String(view?.status === "planning" || view?.status === "working"));
    empty.hidden = Boolean(view?.html);
    empty.textContent = view?.status === "error" ? view.summary || "生成未完成，可单独打开查看原因" :
      view ? `正在等待「${view.title}」的真实结果…` : `找不到 View：${label}`;
    frame.hidden = !view?.html;
    frame.title = label;
    const nextHtml = view?.html || "";
    if (html !== nextHtml) {
      html = nextHtml;
      const child = new DOMParser().parseFromString(html, "text/html");
      const script = child.createElement("script");
      script.textContent = `(${sizeReporter.toString()})();`.replace(/<\/script/gi, "<\\/script");
      child.head.append(script);
      frame.srcdoc = `<!doctype html>\n${child.documentElement.outerHTML}`;
    }
  };
  update();
  return {
    frame, update,
    resize(height) {
      if (typeof height !== "number" || !Number.isFinite(height) || height <= 0) return;
      // Natural height is a fallback; Main owns sizing and presentation through CSS.
      frame.style.setProperty("--chat-view-content-height", `${Math.max(120, Math.min(12000, Math.ceil(height)))}px`);
    },
  };
}
