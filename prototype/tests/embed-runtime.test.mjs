import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";
import { parseHTML } from "linkedom";
import { installCompositionRuntime } from "../src/pageRuntime/bootstrap.ts";
import { createEmbedSlot } from "../src/pageRuntime/embedElement.ts";
import { reportChildSize } from "../src/pageRuntime/childSize.ts";
import { embedStyles } from "../src/pageRuntime/embedStyles.ts";

const views = ["overview", "metrics", "detail"].map((key) => ({
  id: `owned-${key}`, key, title: key, status: "ready", summary: key,
  html: `<!doctype html><html><head></head><body><button>${key}</button></body></html>`,
}));

function runtime(body) {
  const { window } = parseHTML(`<!doctype html><html><head></head><body>${body}</body></html>`);
  const posts = [];
  const parent = { postMessage: (message) => posts.push(message) };
  const context = vm.createContext({
    window, document: window.document, HTMLElement: window.HTMLElement, Element: window.Element,
    customElements: window.customElements, DOMParser: window.DOMParser, crypto, parent,
  });
  vm.runInContext(`(${installCompositionRuntime})("current", ${createEmbedSlot}, ${JSON.stringify(embedStyles)}, ${reportChildSize})`, context);
  const send = (data, source = parent) => {
    const event = new window.Event("message");
    event.data = data;
    event.source = source;
    window.dispatchEvent(event);
  };
  const catalog = (items) => send({ type: "chat-ui:catalog", channel: "current", views: items });
  const slots = () => [...window.document.querySelectorAll("chat-view")];
  const frameSource = (frame) => {
    const received = [];
    const source = { postMessage: (message) => received.push(message) };
    Object.defineProperty(frame, "contentWindow", { value: source });
    return { source, received };
  };
  return { window, document: window.document, catalog, send, slots, posts, frameSource };
}

test("bare embeds expose actual results without host chrome or restructuring Main's content", () => {
  const r = runtime('<h1>入口</h1><nav><a href="#overview">总览</a></nav>');
  r.catalog(views);
  assert.deepEqual(r.slots().map((slot) => slot.dataset.viewId), views.map((view) => view.id));
  assert.ok(r.slots().every((slot) => slot.querySelector("iframe").srcdoc.includes("<button>")));
  assert.ok(r.slots().every((slot) => !slot.querySelector("button,.chat-view-bar,.chat-view-status")));
  assert.ok(r.slots().every((slot) => slot.querySelector(".chat-view-empty").hidden));
  assert.ok(r.document.querySelector("body>nav"));
  assert.ok(r.document.querySelector("body>h1"));
  assert.equal(r.document.querySelector("details[data-chat-legacy]"), null);
  assert.equal(r.document.body.getAttribute("style"), null);
  const frames = r.slots().map((slot) => slot.querySelector("iframe"));
  r.catalog(views);
  assert.deepEqual(r.slots().map((slot) => slot.querySelector("iframe")), frames);
});

test("partial compositions resolve keys and ids and backfill only missing results", () => {
  const r = runtime('<chat-view view-id="overview"></chat-view><chat-view view-id="owned-metrics"></chat-view>');
  r.catalog(views);
  assert.equal(r.slots().length, 3);
  assert.equal(r.document.querySelectorAll("chat-view[data-chat-automatic]").length, 1);
  assert.equal(r.document.querySelector("details[data-chat-legacy]"), null);
  r.catalog(views.slice(0, 2));
  assert.equal(r.slots().length, 2);
  r.catalog(views);
  assert.equal(r.slots().length, 3);
});

test("Main owns folding and sizing; catalog and sibling updates preserve its decisions and iframe state", () => {
  const r = runtime('<details><summary>按需查看关系图</summary><chat-view view-id="overview" style="height:480px;--chat-view-height:480px"></chat-view></details>');
  r.catalog(views);
  const [overview, metrics] = r.slots();
  const disclosure = r.document.querySelector("details");
  assert.equal(disclosure.hasAttribute("open"), false);
  const frame = overview.querySelector("iframe");
  const original = frame.srcdoc;
  frame.interactionMarker = "selected-node";
  disclosure.setAttribute("open", "");
  r.catalog(views);
  assert.equal(disclosure.hasAttribute("open"), true);
  disclosure.removeAttribute("open");
  const updated = views.map((view) => view.key === "metrics" ? { ...view, html: "<html><body>new metrics</body></html>" } : view);
  r.catalog(updated);
  assert.equal(overview.querySelector("iframe"), frame);
  assert.equal(frame.srcdoc, original);
  assert.equal(frame.interactionMarker, "selected-node");
  assert.equal(disclosure.hasAttribute("open"), false);
  assert.match(metrics.querySelector("iframe").srcdoc, /new metrics/);
  disclosure.setAttribute("open", "");
  assert.equal(overview.querySelector("iframe"), frame);
  assert.equal(overview.style.height, "480px");
  assert.equal(overview.style.getPropertyValue("--chat-view-height"), "480px");
});

test("auto-height only accepts finite dimensions from the owning iframe", () => {
  const r = runtime("");
  r.catalog(views);
  assert.equal(r.document.querySelector("[data-chat-legacy]"), null);
  const frame = r.slots()[0].querySelector("iframe");
  const measured = () => frame.style.getPropertyValue("--chat-view-content-height");
  const { source } = r.frameSource(frame);
  r.send({ type: "chat-ui:embed-size", height: 760 }, {});
  assert.ok(!measured());
  r.send({ type: "chat-ui:embed-size", height: 760 }, source);
  assert.equal(measured(), "760px");
  for (const height of [NaN, Infinity, -1, "900"]) r.send({ type: "chat-ui:embed-size", height }, source);
  assert.equal(measured(), "760px");
  r.send({ type: "chat-ui:embed-size", height: 90000 }, source);
  assert.equal(measured(), "12000px");
  r.send({ type: "chat-ui:embed-size", height: 320 }, source);
  assert.equal(measured(), "320px");
  r.slots()[0].style.setProperty("--chat-view-height", "500px");
  r.send({ type: "chat-ui:embed-size", height: 640 }, source);
  assert.equal(r.slots()[0].style.getPropertyValue("--chat-view-height"), "500px");
});

test("Main-hidden siblings retain linked entity context and restore it after a page update", () => {
  const r = runtime("");
  r.catalog(views);
  const [first, second] = r.slots();
  const origin = r.frameSource(first.querySelector("iframe"));
  const targetFrame = second.querySelector("iframe");
  const target = r.frameSource(targetFrame);
  second.hidden = true;
  r.send({ type: "chat-ui:view-event", event: "selection", payload: { entityId: "ranking" } }, origin.source);
  assert.equal(target.received[0].payload.entityId, "ranking");
  r.catalog(views.map((view) => view.key === "metrics" ? { ...view, html: "<html><body>v2</body></html>" } : view));
  targetFrame.onload();
  assert.equal(target.received.length, 2);
  assert.equal(target.received[1].payload.entityId, "ranking");
  assert.equal(second.hidden, true);
  assert.equal(r.posts.filter((post) => post.event === "view-event").length, 0);
});

test("unknown references cannot mask missing content or navigate to a foreign View", () => {
  const r = runtime('<chat-view view-id="foreign"></chat-view><button id="open" data-chat-open-view="overview">查看详情</button><button id="unknown" data-chat-open-view="foreign">未知</button>');
  r.send({ type: "chat-ui:catalog", channel: "stale", views });
  assert.equal(r.slots().length, 1);
  r.catalog(views);
  assert.equal(r.slots().filter((slot) => slot.dataset.viewId).length, 3);
  r.document.getElementById("unknown").click();
  assert.equal(r.posts.filter((post) => post.event === "navigate").length, 0);
  r.document.getElementById("open").click();
  assert.equal(r.posts.at(-1).viewId, views[0].id);
  assert.equal(r.posts.at(-1).event, "navigate");
});
