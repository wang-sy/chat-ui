import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

let importId = 0;
async function loadBridge() {
  return (await import(`../src/bridge.ts?test=${++importId}`)).bridge;
}

function nativeIPC(handler, options) {
  globalThis.window = { crypto: globalThis.crypto };
  mockIPC(handler, options);
}

afterEach(() => {
  if (globalThis.window?.__TAURI_INTERNALS__) clearMocks();
  delete globalThis.window;
});

test("works without a window or Tauri runtime", async () => {
  delete globalThis.window;
  const bridge = await loadBridge();
  assert.equal(bridge.isNative, false);
  assert.deepEqual(await bridge.start(), { running: false, pid: null, executable: null });
  assert.deepEqual(await bridge.stop(), { running: false, pid: null, executable: null });
  const dispose = await bridge.subscribe(() => assert.fail("unexpected browser event"));
  dispose();
  await assert.rejects(bridge.send("{}"), /browser preview cannot send/);
});

test("a plain browser window does not enable native commands", async () => {
  globalThis.window = {};
  const bridge = await loadBridge();
  assert.equal(bridge.isNative, false);
  assert.equal((await bridge.start("not-a-real-executable")).running, false);
  await assert.rejects(bridge.send("{}"), /browser preview cannot send/);
});

test("forwards the selected executable and the native process status", async () => {
  const calls = [];
  const status = { running: true, pid: 123, executable: "C:/Tools/Codex/codex.exe" };
  nativeIPC((command, args) => {
    calls.push({ command, args });
    return status;
  });
  const bridge = await loadBridge();
  assert.equal(bridge.isNative, true);
  assert.deepEqual(await bridge.start(status.executable), status);
  assert.deepEqual(calls, [{ command: "codex_start", args: { executable: status.executable } }]);
});

test("supports default executable selection and stop", async () => {
  const calls = [];
  nativeIPC((command, args) => {
    calls.push({ command, args });
    return { running: false, pid: null, executable: null };
  });
  const bridge = await loadBridge();
  await bridge.start();
  assert.equal((await bridge.stop()).running, false);
  assert.equal(calls[0].command, "codex_start");
  assert.equal(calls[0].args.executable, undefined);
  assert.equal(calls[1].command, "codex_stop");
});

test("preserves JSON-RPC and Unicode payloads", async () => {
  const calls = [];
  nativeIPC((command, args) => calls.push({ command, args }));
  const bridge = await loadBridge();
  const message = JSON.stringify({ id: 1, method: "example", params: { text: "中文 ✓" } });
  await bridge.send(message);
  assert.deepEqual(calls, [{ command: "codex_send", args: { message } }]);
});

test("propagates native start, stop and send errors", async () => {
  nativeIPC((command) => Promise.reject(new Error(`failed: ${command}`)));
  const bridge = await loadBridge();
  await assert.rejects(bridge.start(), /failed: codex_start/);
  await assert.rejects(bridge.stop(), /failed: codex_stop/);
  await assert.rejects(bridge.send("{}"), /failed: codex_send/);
});

test("subscribes only to Codex messages and releases its callback", async () => {
  nativeIPC(() => undefined, { shouldMockEvents: true });
  const bridge = await loadBridge();
  const received = [];
  const dispose = await bridge.subscribe((message) => received.push(message));
  await emit("other-event", "ignored");
  await emit("codex://message", '{"id":1,"result":{}}');
  await emit("codex://message", "[stderr] diagnostic");
  assert.deepEqual(received, ['{"id":1,"result":{}}', "[stderr] diagnostic"]);
  assert.equal(window.__TAURI_INTERNALS__.callbacks.size, 1);
  await dispose();
  assert.equal(window.__TAURI_INTERNALS__.callbacks.size, 0);
});
