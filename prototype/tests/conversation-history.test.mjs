import assert from "node:assert/strict";
import test from "node:test";
import { transcriptFor, currentActivityText } from "../src/conversationTranscript.ts";
import { newestState } from "../src/workspaceState.ts";

test("legacy replies remain readable without showing HTML or incomplete orchestration JSON", () => {
  const chat = { messages: [
    { id: "u", role: "user", text: "解释这个过程" },
    { id: "a", role: "assistant", text: JSON.stringify({ response: "请求 → 响应", page: { html: "<script>private</script>" } }) },
    { id: "internal", role: "user", text: "Runtime integration pass: private" },
    { id: "partial", role: "assistant", text: '{"response":"' },
  ] };
  assert.deepEqual(transcriptFor(chat).map((item) => item.text), ["解释这个过程", "请求 → 响应"]);
});

test("selected thread returns only its owned saved transcript in original order", () => {
  const main = { messages: [], transcript: [{ id: "m", kind: "assistant", text: "主结果" }] };
  const view = { messages: [], transcript: [{ id: "u", kind: "user", text: "改图" },
    { id: "cmd", kind: "command", text: "checked" }, { id: "v", kind: "assistant", text: "图已更新" }] };
  assert.deepEqual(transcriptFor(main).map((item) => item.id), ["m"]);
  assert.deepEqual(transcriptFor(view).map((item) => item.id), ["u", "cmd", "v"]);
  assert.deepEqual(transcriptFor(undefined), []);
});

test("running rail uses accumulated current output, never a previous turn's answer", () => {
  const chat = { activeTurn: "current", activity: "正在思考", messages: [], transcript: [
    { id: "old", kind: "assistant", turnId: "previous", text: "旧答复", status: "completed" },
  ] };
  assert.equal(currentActivityText(chat), "");
  chat.transcript.push({ id: "cmd", kind: "command", turnId: "current", text: "第一步\n第二步", status: "running" });
  assert.equal(currentActivityText(chat), "第一步\n第二步");
  chat.transcript.push({ id: "a", kind: "assistant", turnId: "current", text: "已经检查", status: "running" });
  assert.equal(currentActivityText(chat), "已经检查");
});

test("delayed SSE and polls cannot erase newer View history; a restarted service is accepted", () => {
  const fresh = { instanceId: "service-a", revision: 42, chats: [{ id: "view", transcript: [{ text: "done" }] }] };
  const old = { instanceId: "service-a", revision: 41, chats: [] };
  assert.equal(newestState(fresh, old, true), fresh);
  assert.equal(newestState(fresh, old), fresh);
  assert.equal(newestState(fresh, { ...old, revision: 42 }, true), fresh);
  const updatedProjectList = { ...fresh, projects: [{ path: "/new" }] };
  assert.equal(newestState(fresh, updatedProjectList), updatedProjectList);
  const restarted = { ...fresh, instanceId: "service-b", revision: 1 };
  assert.equal(newestState(fresh, restarted, true), restarted);
});
