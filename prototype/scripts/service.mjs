import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export function startService() {
  const fallback = join(homedir(), ".cargo", "bin", process.platform === "win32" ? "cargo.exe" : "cargo");
  const cargo = existsSync(fallback) ? fallback : "cargo";
  const child = spawn(cargo, ["run", "--manifest-path", "src-tauri/Cargo.toml", "--bin", "chat-ui-server"], {
    stdio: "inherit", windowsHide: true,
  });
  child.on("error", error => { console.error(`无法启动本地服务：${error.message}。请安装 Rust。`); process.exitCode = 1; });
  return child;
}

export function stopService(child) {
  if (!child || child.killed) return;
  if (process.platform === "win32" && child.pid) {
    spawn("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore", windowsHide: true });
  } else {
    child.kill("SIGTERM");
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const child = startService();
  child.on("exit", code => { process.exitCode = code || 0; });
}
