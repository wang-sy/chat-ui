import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { execFileSync } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createInterface } from "node:readline";
import test from "node:test";

test("local Codex supports stdio initialization, requests, errors and shutdown", { timeout: 30000 }, async () => {
  const home = await mkdtemp(path.join(tmpdir(), "chat-ui-codex-test-"));
  const executable = process.env.CODEX_EXECUTABLE || (process.platform === "win32" ? "codex.exe" : "codex");
  const child = spawn(executable, ["app-server", "--listen", "stdio://"], {
    cwd: home,
    env: { ...process.env, CODEX_HOME: home },
    stdio: ["pipe", "pipe", "pipe"],
    windowsHide: true,
  });
  const pending = new Map();
  let nextId = 1;
  let stderr = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr = (stderr + chunk).slice(-8000); });
  const lines = createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    try {
      const message = JSON.parse(line);
      if (!pending.has(message.id)) return;
      const { resolve, timer } = pending.get(message.id);
      clearTimeout(timer);
      pending.delete(message.id);
      resolve(message);
    } catch (error) {
      for (const { reject, timer } of pending.values()) {
        clearTimeout(timer);
        reject(new Error(`Non-JSON protocol output: ${error.message}`));
      }
      pending.clear();
    }
  });
  child.on("error", (error) => {
    for (const { reject, timer } of pending.values()) {
      clearTimeout(timer);
      reject(error);
    }
    pending.clear();
  });
  function request(method, params) {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`Timed out: ${method}\n${stderr}`));
      }, 10000);
      pending.set(id, { resolve, reject, timer });
      child.stdin.write(`${JSON.stringify({ id, method, params })}\n`);
    });
  }
  let shutdownTimer;
  try {
    const initialized = await request("initialize", {
      clientInfo: { name: "chat-ui-integration-test", version: "0.1.0" },
    });
    assert.equal(initialized.error, undefined);
    assert.ok(initialized.result);
    child.stdin.write(`${JSON.stringify({ method: "initialized" })}\n`);
    const config = await request("config/read", { includeLayers: false });
    assert.equal(config.error, undefined);
    assert.ok(config.result.config);
    const unknown = await request("chat-ui/unsupported-test-method", {});
    assert.ok(unknown.error);
    assert.equal(typeof unknown.error.code, "number");
    const second = await request("config/read", { includeLayers: false });
    assert.equal(second.error, undefined, "server should still respond after a bad request");
    const exited = once(child, "exit");
    child.stdin.end();
    const [code] = await Promise.race([
      exited,
      new Promise((_, reject) => {
        shutdownTimer = setTimeout(() => reject(new Error("Codex did not stop after stdin closed")), 5000);
      }),
    ]);
    assert.equal(code, 0);
  } finally {
    clearTimeout(shutdownTimer);
    for (const { timer } of pending.values()) clearTimeout(timer);
    lines.close();
    if (child.exitCode === null && child.signalCode === null) {
      const closed = once(child, "close");
      child.kill();
      await Promise.race([
        closed,
        new Promise((resolve) => setTimeout(resolve, 2000)),
      ]);
    }
    if (child.exitCode === null && child.signalCode === null && process.platform === "win32") {
      try {
        execFileSync("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], {
          stdio: "ignore",
        });
      } catch {
        // The process may have exited between the state check and taskkill.
      }
      await Promise.race([
        once(child, "close"),
        new Promise((resolve) => setTimeout(resolve, 2000)),
      ]);
    }
    if (path.dirname(path.resolve(home)) !== path.resolve(tmpdir()) || !path.basename(home).startsWith("chat-ui-codex-test-")) {
      throw new Error("Refusing to remove an unexpected test directory");
    }
    await rm(home, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  }
});
