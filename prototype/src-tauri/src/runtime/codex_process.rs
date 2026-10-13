use super::{discover, hidden_command, Reply, Runtime};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
#[cfg(windows)]
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::time::Duration;

impl Runtime {
    pub fn connect(self: &Arc<Self>) -> Result<(), String> {
        let _gate = self.connect_gate.lock().unwrap();
        if self.ready.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.stop_process();
        let result = self.start_process();
        if let Err(error) = &result {
            self.stop_process();
            *self.issue.lock().unwrap() = Some(error.clone());
            self.publish_event("chat-ui/connection/error", &json!({"error": error}));
        } else {
            self.publish_event("chat-ui/connection/ready", &json!({}));
        }
        result
    }

    fn start_process(self: &Arc<Self>) -> Result<(), String> {
        let executable = discover()
            .ok_or("未在 PATH 中找到 Codex。请安装 Codex 或将它加入 PATH 后重启本地服务。")?;
        let mut command = hidden_command(&executable);
        #[cfg(windows)]
        if executable
            .extension()
            .is_some_and(|ext| ext == "cmd" || ext == "bat")
        {
            let path = executable.to_string_lossy();
            if path.contains(['"', '%', '\n', '\r']) {
                return Err("Codex 启动脚本路径包含不支持的字符。".into());
            }
            command = hidden_command(Path::new("cmd.exe"));
            command.args([
                "/d",
                "/s",
                "/c",
                &format!("\"\"{path}\" app-server --listen stdio://\""),
            ]);
        } else {
            command.args(["app-server", "--listen", "stdio://"]);
        }
        #[cfg(not(windows))]
        command.args(["app-server", "--listen", "stdio://"]);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("无法启动 Codex：{e}"))?;
        let output = child.stdout.take().ok_or("无法读取 Codex 输出")?;
        let errors = child.stderr.take().ok_or("无法读取 Codex 错误输出")?;
        *self.writer.lock().unwrap() = child.stdin.take();
        *self.child.lock().unwrap() = Some(child);
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let weak = Arc::downgrade(self);
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                let Some(runtime) = weak.upgrade() else {
                    return;
                };
                if runtime.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                if let Ok(message) = serde_json::from_str::<Value>(&line) {
                    runtime.receive(message);
                }
            }
            if let Some(runtime) = weak.upgrade() {
                if runtime.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                runtime.ready.store(false, Ordering::SeqCst);
                for (_, reply) in runtime.pending.lock().unwrap().drain() {
                    let _ = reply.send(Err("Codex 连接已关闭".into()));
                }
                for chat in &mut runtime.store.lock().unwrap().chats {
                    if chat.active_turn.take().is_some() {
                        chat.error = Some("Codex 连接已关闭，请重新连接后重试。".into());
                    }
                    chat.activity = None;
                    chat.requests.clear();
                }
                runtime.publish_event(
                    "chat-ui/connection/closed",
                    &json!({"reason":"Codex 连接已关闭"}),
                );
            }
        });
        std::thread::spawn(move || {
            // Drain diagnostics so a full stderr pipe never blocks app-server.
            for _ in BufReader::new(errors).lines().map_while(Result::ok) {}
        });
        self.rpc(
            "initialize",
            json!({"clientInfo":{"name":"chat_ui","title":"Chat UI","version":"0.1.0"}}),
        )?;
        self.write(&json!({"method":"initialized"}))?;
        let mut catalog = vec![];
        let mut cursor = Value::Null;
        for _ in 0..10 {
            let page = self.rpc("model/list", json!({"limit":100,"cursor":cursor}))?;
            catalog.extend(page["data"].as_array().cloned().unwrap_or_default());
            cursor = page["nextCursor"].clone();
            if cursor.is_null() {
                break;
            }
        }
        *self.models.lock().unwrap() = catalog;
        self.resumed.lock().unwrap().clear();
        self.ready.store(true, Ordering::SeqCst);
        *self.issue.lock().unwrap() = None;
        Ok(())
    }

    pub(crate) fn write(&self, value: &Value) -> Result<(), String> {
        let mut writer = self.writer.lock().unwrap();
        let writer = writer.as_mut().ok_or("本地 Codex 尚未连接")?;
        writeln!(writer, "{value}")
            .and_then(|_| writer.flush())
            .map_err(|e| e.to_string())
    }

    pub(crate) fn rpc(&self, method: &str, params: Value) -> Reply {
        let id = self.sequence.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, sender);
        if let Err(error) = self.write(&json!({"id":id,"method":method,"params":params})) {
            self.pending.lock().unwrap().remove(&id);
            return Err(error);
        }
        let result = receiver
            .recv_timeout(Duration::from_secs(40))
            .map_err(|_| format!("Codex 请求超时：{method}"));
        self.pending.lock().unwrap().remove(&id);
        result?
    }

    pub fn stop_process(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.ready.store(false, Ordering::SeqCst);
        *self.writer.lock().unwrap() = None;
        for (_, reply) in self.pending.lock().unwrap().drain() {
            let _ = reply.send(Err("Codex 连接已关闭".into()));
        }
        if let Some(mut child) = self.child.lock().unwrap().take() {
            #[cfg(windows)]
            {
                let _ = hidden_command(Path::new("taskkill.exe"))
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
