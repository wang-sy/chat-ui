import assert from "node:assert/strict";
import test from "node:test";
import { findView, surfaceEvent, viewCatalog } from "../src/pageRuntime/protocol.ts";

const views = [{ id: "view-owned", key: "architecture", title: "关系图", html: "<svg></svg>",
  status: "ready", summary: "召回 → 排序", threadId: "private-thread", brief: "private context" }];

test("live embeds use stable ids or semantic references, without leaking thread context", () => {
  assert.equal(findView(views, "architecture"), views[0]);
  assert.equal(findView(views, "view-owned"), views[0]);
  assert.equal(findView(views, "Architecture"), undefined);
  assert.equal(findView(views, "missing"), undefined);
  const catalog = viewCatalog({ views });
  assert.deepEqual(Object.keys(catalog[0]).sort(), ["html", "id", "key", "status", "summary", "title"]);
});

test("composition events require the current channel and an owned View", () => {
  const message = { type: "chat-ui:surface", channel: "current", event: "navigate", viewId: "architecture" };
  assert.equal(surfaceEvent(message, "current", views).view.id, "view-owned");
  assert.equal(surfaceEvent(message, "stale", views), null);
  assert.equal(surfaceEvent({ ...message, viewId: "foreign-view" }, "current", views), null);
  const action = { ...message, event: "view-event", name: "action", payload: { intent: "解释排序" } };
  assert.deepEqual(surfaceEvent(action, "current", views).event, { name: "action", payload: { intent: "解释排序" } });
  assert.equal(surfaceEvent({ ...action, payload: "x".repeat(4097) }, "current", views), null);
});
