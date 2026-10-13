import type { EmbeddedView } from "./protocol";
import type { createEmbedSlot, EmbedSlot } from "./embedElement";

/** Runs inside Main's opaque-origin iframe. All serialized dependencies are arguments. */
export function installCompositionRuntime(
  channel: string, createSlot: typeof createEmbedSlot, styles: string, sizeReporter: () => void,
) {
  let catalog: EmbeddedView[] = [];
  let selection: { sourceViewId: string; payload: { entityId: string } } | null = null;
  const slots = new Map<HTMLElement, EmbedSlot>();
  const post = (data: Record<string, unknown>) =>
    parent.postMessage({ type: "chat-ui:surface", channel, ...data }, "*");
  const resolve = (reference: string | null) => catalog.find((v) => v.id === reference)
    || catalog.find((v) => v.key && v.key === reference);
  const bounded = (payload: unknown) => {
    try {
      const text = JSON.stringify(payload ?? null);
      return text.length <= 4096 ? JSON.parse(text) : undefined;
    } catch { return undefined; }
  };
  const style = document.createElement("style");
  style.textContent = styles;
  document.head.append(style);

  // Covers old directory pages and partial plans as well as new assignments.
  // This composes real owned results in the same Main document, never copies them.
  const ensureCoverage = () => {
    if (!document.body) return;
    document.querySelectorAll<HTMLElement>("chat-view[data-chat-automatic]").forEach((element) => {
      if (!resolve(element.getAttribute("view-id"))) element.remove();
    });
    const present = new Set([...slots.keys()].map((element) => resolve(element.getAttribute("view-id"))?.id));
    const missing = catalog.filter((view) => !present.has(view.id));
    if (!missing.length) return;
    let fallback = document.querySelector<HTMLElement>("[data-chat-fallback]");
    if (!fallback) {
      fallback = document.createElement("section");
      fallback.dataset.chatFallback = "";
      fallback.setAttribute("aria-label", "内嵌 View");
      // Backfill missing results without restructuring or restyling Main's document.
      document.body.append(fallback);
    }
    for (const view of missing) {
      const element = document.createElement("chat-view");
      element.setAttribute("view-id", view.id);
      element.dataset.chatAutomatic = "";
      fallback.append(element);
    }
  };
  const refresh = () => { ensureCoverage(); for (const slot of slots.values()) slot.update(); };

  class ChatView extends HTMLElement {
    connectedCallback() {
      if (slots.has(this)) return;
      slots.set(this, createSlot(this, resolve,
        (frame) => {
          if (selection) frame.contentWindow?.postMessage({ type: "chat-ui:context", ...selection }, "*");
        }, sizeReporter));
    }
    disconnectedCallback() { slots.delete(this); }
    static get observedAttributes() { return ["view-id"]; }
    attributeChangedCallback() { slots.get(this)?.update(); }
  }
  customElements.define("chat-view", ChatView);
  document.addEventListener("DOMContentLoaded", refresh, { once: true });

  document.addEventListener("click", (event) => {
    const target = event.target instanceof Element
      ? event.target.closest<HTMLElement>("[data-chat-open-view],a[href^='#']") : null;
    if (!target) return;
    const reference = target.dataset.chatOpenView ?? target.getAttribute("href")?.slice(1) ?? "";
    let decoded = reference;
    try { decoded = decodeURIComponent(reference); } catch { /* retain exact reference */ }
    const view = resolve(decoded);
    if (view) {
      event.preventDefault();
      post({ event: "navigate", viewId: view.id });
    }
  });

  window.addEventListener("message", (event) => {
    const data = event.data;
    if (!data || typeof data !== "object") return;
    if (event.source === parent) {
      if (data.type === "chat-ui:catalog" && data.channel === channel && Array.isArray(data.views)) {
        catalog = data.views;
        refresh();
      }
      return;
    }
    // Bind size and interaction messages to the actual mounted child WindowProxy.
    const owner = [...slots].find(([, slot]) => event.source === slot.frame.contentWindow);
    if (!owner) return;
    if (data.type === "chat-ui:embed-size") { owner[1].resize(data.height); return; }
    if (data.type !== "chat-ui:view-event" || typeof data.event !== "string") return;
    const view = resolve(owner[0].getAttribute("view-id"));
    const payload = bounded(data.payload);
    if (!view || payload === undefined) return;
    if (data.event === "selection") {
      if (!payload || typeof payload.entityId !== "string" || payload.entityId.length > 120) return;
      selection = { sourceViewId: view.id, payload };
      document.querySelectorAll<HTMLElement>("[data-entity-id]").forEach((element) => {
        element.dataset.chatSelected = String(element.dataset.entityId === payload.entityId);
      });
      for (const [, slot] of slots) {
        if (slot !== owner[1]) slot.frame.contentWindow?.postMessage({
          type: "chat-ui:context", sourceViewId: view.id, payload,
        }, "*");
      }
      return;
    }
    post({ event: "view-event", viewId: view.id, name: data.event, payload });
  });
  post({ event: "ready" });
}
