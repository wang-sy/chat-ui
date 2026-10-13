import assert from "node:assert/strict";
import test from "node:test";

import {
  appendQueuedMessage,
  editQueuedMessage,
  maxQueueItems,
  removeQueuedMessage,
  reorderQueuedMessages,
} from "../src/conversationQueue.ts";
import { routeViewEvent } from "../src/hooks/conversationEvents.ts";

const view = {
  id: "architecture",
  title: "系统架构",
  threadId: "view-thread-1",
};
const settings = {
  modelId: "gpt-6.1-sol",
  effort: "high",
  speed: "standard",
  permission: "ask",
};

function queued(id, text = id) {
  return {
    id,
    text,
    targetViewId: null,
    targetTitle: null,
    settings,
    createdAt: Number(id.replace(/\D/g, "")) || 1,
  };
}

test("C04: View postMessage events route to the owning thread", async () => {
  const calls = [];
  let drafts = {};
  const setDrafts = (update) => {
    drafts = typeof update === "function" ? update(drafts) : update;
  };
  const openView = (next) => calls.push(["view", next.id]);
  const openMain = () => calls.push(["main"]);
  const dispatch = async (text) => {
    calls.push(["dispatch", text]);
    return { status: "sent" };
  };
  const enqueue = (text) => {
    calls.push(["queue", text]);
    return true;
  };

  await routeViewEvent({
    view,
    event: { name: "focus", payload: null },
    settings,
    cwd: "/tmp/chat-ui",
    openView,
    openMain,
    setDrafts,
    isWorking: () => false,
    enqueue,
    dispatch,
  });
  assert.deepEqual(calls, [], "local focus must not expand the conversation dock");

  await routeViewEvent({
    view,
    event: { name: "request-main", payload: { filter: "date" } },
    settings,
    cwd: "/tmp/chat-ui",
    openView,
    openMain,
    setDrafts,
    isWorking: () => false,
    enqueue,
    dispatch,
  });
  assert.equal(calls.at(-1)[0], "main");
  assert.match(drafts["/tmp/chat-ui:main"], /来自「系统架构」的页面请求/);

  await routeViewEvent({
    view,
    event: { name: "action", payload: { intent: "解释排序服务" } },
    settings,
    cwd: "/tmp/chat-ui",
    openView,
    openMain,
    setDrafts,
    isWorking: () => false,
    enqueue,
    dispatch,
  });
  assert.equal(calls.at(-1)[0], "dispatch");

  await routeViewEvent({
    view,
    event: { name: "action", payload: { intent: "更新数据" } },
    settings,
    cwd: "/tmp/chat-ui",
    openView,
    openMain,
    setDrafts,
    isWorking: () => true,
    enqueue,
    dispatch,
  });
  assert.equal(calls.at(-1)[0], "queue");
  const count = calls.length;
  await routeViewEvent({
    view, event: { name: "action", payload: { action: "toggle-explanation" } },
    settings, cwd: "/tmp/chat-ui", openView, openMain, setDrafts,
    isWorking: () => false, enqueue, dispatch,
  });
  assert.equal(calls.length, count, "local toggles must not start or queue agent work");
});

test("C07: queued messages preserve limit, edit, delete and drag order", () => {
  let queue = [];
  for (let index = 1; index <= maxQueueItems(); index += 1) {
    queue = appendQueuedMessage(queue, queued(`q${index}`));
  }
  assert.equal(queue.length, maxQueueItems());
  assert.equal(appendQueuedMessage(queue, queued("overflow")), null);

  queue = editQueuedMessage(queue, "q3", "edited");
  assert.equal(queue.find((item) => item.id === "q3").text, "edited");
  queue = reorderQueuedMessages(queue, "q3", "q1");
  assert.deepEqual(queue.slice(0, 3).map((item) => item.id), ["q3", "q1", "q2"]);
  queue = removeQueuedMessage(queue, "q1");
  assert.equal(queue.some((item) => item.id === "q1"), false);
});

test("C07: an active turn accepts a steer request with its request id", async () => {
  const originalFetch = globalThis.fetch;
  const calls = [];
  globalThis.fetch = async (url, options = {}) => {
    calls.push({ url: String(url), options });
    if (String(url).endsWith("/health")) {
      return new Response(JSON.stringify({ service: "chat-ui", protocol: 1, token: "case-token" }), { status: 200 });
    }
    return new Response(JSON.stringify({ id: "thread-1", steered: true }), { status: 200 });
  };
  try {
    const { steerIfActive } = await import(`../src/hooks/conversationSteer.ts?case=${Date.now()}`);
    const pending = steerIfActive({ id: "thread-1", activeTurn: "turn-1" }, "/tmp/chat-ui", "继续检查", async () => {});
    assert.equal(pending instanceof Promise, true);
    const result = await pending;
    assert.deepEqual(result, { status: "sent" });
    const body = JSON.parse(calls.at(-1).options.body);
    assert.deepEqual(body, {
      threadId: "thread-1",
      cwd: "/tmp/chat-ui",
      text: "继续检查",
      requestId: body.requestId,
    });
    assert.equal(typeof body.requestId, "string");
    assert.equal(steerIfActive({ id: "thread-1", activeTurn: "starting" }, "/tmp/chat-ui", "x", async () => {}), null);
  } finally {
    globalThis.fetch = originalFetch;
  }
});
