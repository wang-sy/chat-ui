import assert from "node:assert/strict";
import { afterEach, test } from "node:test";

let importId = 0;
const originalFetch = globalThis.fetch;

async function loadBridge() {
  return (await import(`../src/bridge.ts?test=${++importId}`)).bridge;
}

function jsonResponse(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

afterEach(() => {
  globalThis.fetch = originalFetch;
});

test("browser bridge discovers the local service and reads shared state", async () => {
  const calls = [];
  globalThis.fetch = async (url, options = {}) => {
    calls.push({ url: String(url), options });
    if (String(url).endsWith("/health")) {
      return jsonResponse({ service: "chat-ui", protocol: 1, token: "test-token" });
    }
    assert.equal(options.headers["X-Chat-UI-Token"], "test-token");
    return jsonResponse({ connected: true, available: true, issue: null, projects: [], chats: [], models: [], home: "C:/Users/test" });
  };

  const bridge = await loadBridge();
  assert.deepEqual(await bridge.state(), { connected: true, available: true, issue: null, projects: [], chats: [], models: [], home: "C:/Users/test" });
  assert.equal(calls[0].url, "http://127.0.0.1:4317/health");
  assert.equal(calls[1].url, "http://127.0.0.1:4317/api/state");
});

test("browser bridge sends directory, connection, and message requests", async () => {
  const calls = [];
  globalThis.fetch = async (url, options = {}) => {
    calls.push({ url: String(url), options });
    if (String(url).endsWith("/health")) return jsonResponse({ service: "chat-ui", protocol: 1, token: "token" });
    if (String(url).includes("/api/directories")) return jsonResponse({ path: "C:/work", parent: "C:/", children: [], roots: [] });
    if (String(url).endsWith("/api/connect")) return jsonResponse({ connected: true, available: true, issue: null, projects: [], chats: [], models: [], home: "C:/Users/test" });
    if (String(url).endsWith("/api/projects")) return jsonResponse({ path: "C:/work" });
    return jsonResponse({ id: "thread-1" });
  };

  const bridge = await loadBridge();
  await bridge.directories("C:/My Project");
  await bridge.connect();
  await bridge.addProject("C:/work");
  await bridge.send({ threadId: "thread-1", cwd: "C:/work", model: "gpt-6.1", effort: "high", text: "你好", requestId: "message-1" });

  assert.equal(calls[1].url, "http://127.0.0.1:4317/api/directories?path=C%3A%2FMy%20Project");
  assert.deepEqual(JSON.parse(calls[2].options.body), {});
  assert.deepEqual(JSON.parse(calls[4].options.body), { threadId: "thread-1", cwd: "C:/work", model: "gpt-6.1", effort: "high", text: "你好", requestId: "message-1" });
});

test("browser bridge refreshes its token once after a stale service token", async () => {
  let healthCalls = 0;
  globalThis.fetch = async (url, options = {}) => {
    if (String(url).endsWith("/health")) {
      healthCalls += 1;
      return jsonResponse({ service: "chat-ui", protocol: 1, token: healthCalls === 1 ? "old" : "new" });
    }
    if (options.headers["X-Chat-UI-Token"] === "old") return jsonResponse({ error: "请重新连接本地服务" }, 401);
    return jsonResponse({ connected: false, available: false, issue: null, projects: [], chats: [], models: [], home: "" });
  };

  const bridge = await loadBridge();
  await bridge.state();
  assert.equal(healthCalls, 2);
});

test("browser bridge reports local service errors", async () => {
  globalThis.fetch = async (url) => String(url).endsWith("/health")
    ? jsonResponse({ service: "chat-ui", protocol: 1, token: "token" })
    : jsonResponse({ error: "Codex 请求失败" }, 400);
  const bridge = await loadBridge();
  await assert.rejects(bridge.addProject("C:/missing"), /Codex 请求失败/);
});
