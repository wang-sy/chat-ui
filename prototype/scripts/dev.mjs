import { spawn } from "node:child_process";
import { resolve } from "node:path";
import { startService, stopService } from "./service.mjs";

function runNode(path, args) {
  return spawn(process.execPath, [path, ...args], { stdio: "inherit", windowsHide: true });
}

async function health() {
  try {
    const response = await fetch("http://127.0.0.1:4317/health", { signal: AbortSignal.timeout(700) });
    const body = await response.json();
    return body.service === "chat-ui" && body.protocol === 1 ? body : null;
  } catch { return null; }
}

const service = await health() ? null : startService();
const vite = runNode(resolve("node_modules/vite/bin/vite.js"), process.argv.slice(2));
let closing = false;
async function close() {
  if (closing) return;
  closing = true;
  vite.kill();
  if (service) {
    stopService(service);
  }
}
process.once("SIGINT", () => void close());
process.once("SIGTERM", () => void close());
vite.once("exit", () => void close());
service?.once("exit", code => { if (code && !closing) { console.error("本地服务启动失败。"); vite.kill(); process.exitCode = code; } });
