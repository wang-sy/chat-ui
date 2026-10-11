use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

type Reply = Result<Value, String>;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub role: String,
    pub text: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub model: String,
    pub messages: Vec<Message>,
    pub active_turn: Option<String>,
    pub activity: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub requests: Vec<Value>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Project {
    pub path: String,
    pub name: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Store {
    pub projects: Vec<Project>,
    pub chats: Vec<Chat>,
}

pub struct Runtime {
    pub store: Mutex<Store>,
    child: Mutex<Option<Child>>,
    writer: Mutex<Option<ChildStdin>>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
    models: Mutex<Vec<Value>>,
    issue: Mutex<Option<String>>,
    ready: AtomicBool,
    sequence: AtomicU64,
    generation: AtomicU64,
    connect_gate: Mutex<()>,
    mutation_gate: Mutex<()>,
    resumed: Mutex<HashSet<String>>,
    completed_turns: Mutex<HashSet<String>>,
    state_dir: PathBuf,
}

pub fn home_dir() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn discover_in(path: &std::ffi::OsStr) -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["codex.exe", "codex.cmd", "codex.bat"]
    } else {
        &["codex"]
    };
    for directory in std::env::split_paths(path) {
        if directory.as_os_str().is_empty() || !directory.is_absolute() {
            continue;
        }
        for name in names {
            let candidate = directory.join(name);
            if !candidate.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = candidate.metadata() {
                    if metadata.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
            }
            return Some(candidate);
        }
    }
    None
}

pub fn discover() -> Option<PathBuf> {
    discover_in(&std::env::var_os("PATH")?)
}

pub fn hidden_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

impl Runtime {
    pub fn new(state_dir: PathBuf) -> Arc<Self> {
        let mut store: Store = std::fs::read(state_dir.join("projects.json"))
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        store.chats.clear();
        for project in &store.projects {
            if let Ok(data) = std::fs::read(Path::new(&project.path).join(".space/chat-ui.json")) {
                if let Ok(chats) = serde_json::from_slice::<Vec<Chat>>(&data) {
                    store.chats.extend(
                        chats
                            .into_iter()
                            .filter(|chat| chat.cwd == project.path)
                            .map(|mut chat| {
                                chat.active_turn = None;
                                chat.activity = None;
                                chat.requests.clear();
                                chat
                            }),
                    );
                }
            }
        }
        Arc::new(Self {
            store: Mutex::new(store),
            child: Mutex::new(None),
            writer: Mutex::new(None),
            pending: Mutex::new(HashMap::new()),
            models: Mutex::new(vec![]),
            issue: Mutex::new(None),
            ready: AtomicBool::new(false),
            sequence: AtomicU64::new(1),
            generation: AtomicU64::new(0),
            connect_gate: Mutex::new(()),
            mutation_gate: Mutex::new(()),
            resumed: Mutex::new(HashSet::new()),
            completed_turns: Mutex::new(HashSet::new()),
            state_dir,
        })
    }

    pub fn snapshot(&self) -> Value {
        let store = self.store.lock().unwrap();
        json!({
            "connected": self.ready.load(Ordering::SeqCst),
            "available": discover().is_some(), "issue": *self.issue.lock().unwrap(),
            "projects": store.projects, "chats": store.chats,
            "models": *self.models.lock().unwrap(), "home": home_dir(),
        })
    }

    pub fn persist(&self) -> Result<(), String> {
        let store = self.store.lock().unwrap();
        std::fs::create_dir_all(&self.state_dir).map_err(|e| e.to_string())?;
        atomic_json(
            &self.state_dir.join("projects.json"),
            &json!({"projects": store.projects, "chats": []}),
        )?;
        for project in &store.projects {
            let chats: Vec<&Chat> = store
                .chats
                .iter()
                .filter(|chat| chat.cwd == project.path)
                .collect();
            let directory = Path::new(&project.path).join(".space");
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            atomic_json(&directory.join("chat-ui.json"), &json!(chats))?;
        }
        Ok(())
    }

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

    fn write(&self, value: &Value) -> Result<(), String> {
        let mut writer = self.writer.lock().unwrap();
        let writer = writer.as_mut().ok_or("本地 Codex 尚未连接")?;
        writeln!(writer, "{value}")
            .and_then(|_| writer.flush())
            .map_err(|e| e.to_string())
    }

    fn rpc(&self, method: &str, params: Value) -> Reply {
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

    pub fn add_project(&self, path: &str) -> Reply {
        let path = std::fs::canonicalize(path).map_err(|_| "目录不存在或无法访问".to_string())?;
        if !path.is_dir() {
            return Err("请选择文件夹".into());
        }
        let path = display_path(&path);
        let name = Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        {
            let mut store = self.store.lock().unwrap();
            if !store.projects.iter().any(|p| p.path == path) {
                store.projects.push(Project {
                    path: path.clone(),
                    name,
                });
            }
        }
        self.persist()?;
        Ok(json!({"path":path}))
    }

    pub fn open_chat(self: &Arc<Self>, id: &str) -> Reply {
        self.connect()?;
        let _gate = self.mutation_gate.lock().unwrap();
        self.resume(id)?;
        Ok(json!({"id":id}))
    }

    fn resume(&self, id: &str) -> Result<(), String> {
        if self.resumed.lock().unwrap().contains(id) {
            return Ok(());
        }
        let chat = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or("找不到聊天")?;
        self.rpc("thread/resume", json!({"threadId":id,"cwd":chat.cwd,"approvalPolicy":"on-request","sandbox":"workspace-write","excludeTurns":true}))?;
        let response = self.rpc("thread/read", json!({"threadId":id,"includeTurns":true}))?;
        if let Some(turns) = response["thread"]["turns"].as_array() {
            let messages = messages_from_turns(turns);
            if !messages.is_empty() {
                if let Some(chat) = self
                    .store
                    .lock()
                    .unwrap()
                    .chats
                    .iter_mut()
                    .find(|c| c.id == id)
                {
                    chat.messages = messages;
                }
            }
        }
        self.resumed.lock().unwrap().insert(id.to_string());
        Ok(())
    }

    pub fn send(self: &Arc<Self>, body: &Value) -> Reply {
        let text = body["text"].as_str().unwrap_or("").trim();
        if text.is_empty() || text.len() > 100_000 {
            return Err("请输入 1 至 100000 字节的消息".into());
        }
        self.connect()?;
        let _gate = self.mutation_gate.lock().unwrap();
        let model = body["model"].as_str().ok_or("请选择模型")?;
        if !self
            .models
            .lock()
            .unwrap()
            .iter()
            .any(|m| m["model"] == model)
        {
            return Err("所选模型不在 Codex 模型列表中".into());
        }
        let id = if let Some(id) = body["threadId"].as_str() {
            self.resume(id)?;
            id.to_string()
        } else {
            let cwd = body["cwd"].as_str().ok_or("请先选择项目目录")?;
            if !self
                .store
                .lock()
                .unwrap()
                .projects
                .iter()
                .any(|p| p.path == cwd)
            {
                return Err("请先选择项目目录".into());
            }
            let result = self.rpc("thread/start", json!({"cwd":cwd,"model":model,"approvalPolicy":"on-request","sandbox":"workspace-write"}))?;
            let id = result["thread"]["id"]
                .as_str()
                .ok_or("Codex 没有返回线程 ID")?
                .to_string();
            self.store.lock().unwrap().chats.insert(
                0,
                Chat {
                    id: id.clone(),
                    cwd: cwd.to_string(),
                    title: text.chars().take(40).collect(),
                    model: model.to_string(),
                    ..Chat::default()
                },
            );
            self.resumed.lock().unwrap().insert(id.clone());
            id
        };
        let user_id = body["requestId"].as_str().ok_or("缺少消息 ID")?.to_string();
        {
            let mut store = self.store.lock().unwrap();
            let chat = store
                .chats
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or("找不到聊天")?;
            if chat.messages.iter().any(|m| m.id == user_id) {
                return Ok(json!({"id":id}));
            }
            if chat.active_turn.is_some() {
                return Err("这个线程正在工作，请停止后再发送。".into());
            }
            chat.active_turn = Some("starting".into());
            chat.activity = Some("正在思考".into());
            chat.error = None;
            chat.model = model.to_string();
            chat.messages.push(Message {
                id: user_id.clone(),
                role: "user".into(),
                text: text.into(),
            });
        }
        let mut params = json!({"threadId":id,"model":model,"input":[{"type":"text","text":text}]});
        if let Some(effort) = body["effort"].as_str().filter(|v| !v.is_empty()) {
            params["effort"] = json!(effort);
        }
        let result = self.rpc("turn/start", params);
        {
            let mut store = self.store.lock().unwrap();
            let chat = store.chats.iter_mut().find(|c| c.id == id).unwrap();
            match &result {
                Ok(response) => {
                    // A short turn may complete before the turn/start response arrives.
                    let turn_id = response["turn"]["id"].as_str().unwrap_or("");
                    if !turn_id.is_empty()
                        && !self.completed_turns.lock().unwrap().contains(turn_id)
                    {
                        chat.active_turn = Some(turn_id.into());
                    }
                }
                Err(error) => {
                    chat.active_turn = None;
                    chat.activity = None;
                    chat.error = Some(error.clone());
                    chat.messages.retain(|m| m.id != user_id);
                }
            }
        }
        self.persist()?;
        result.map(|_| json!({"id":id}))
    }

    pub fn interrupt(&self, id: &str) -> Reply {
        let turn = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.active_turn.clone())
            .ok_or("线程没有在运行")?;
        self.rpc("turn/interrupt", json!({"threadId":id,"turnId":turn}))
    }

    pub fn reply(&self, body: &Value) -> Reply {
        let id = &body["id"];
        let request = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .flat_map(|c| &c.requests)
            .find(|r| &r["id"] == id)
            .cloned()
            .ok_or("请求已结束或不存在")?;
        let method = request["method"].as_str().unwrap_or("");
        let result = match method {
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
                let decision = body["decision"].as_str().unwrap_or("decline");
                if !["accept", "decline", "cancel"].contains(&decision) {
                    return Err("无效的决定".into());
                }
                json!({"decision":decision})
            }
            "item/permissions/requestApproval" => json!({"permissions":{},"scope":"turn"}),
            "item/tool/requestUserInput" => json!({"answers":body["answers"]}),
            "mcpServer/elicitation/request" => json!({"action":"decline","content":null}),
            _ => return Err("不支持此类请求".into()),
        };
        self.write(&json!({"id":id,"result":result}))?;
        for chat in &mut self.store.lock().unwrap().chats {
            chat.requests.retain(|r| &r["id"] != id);
        }
        Ok(json!({"ok":true}))
    }

    fn receive(&self, message: Value) {
        if message.get("method").is_none() {
            if let Some(id) = message["id"].as_u64() {
                if let Some(sender) = self.pending.lock().unwrap().remove(&id) {
                    let result = if message.get("error").is_some() {
                        Err(message["error"]["message"]
                            .as_str()
                            .unwrap_or("Codex 请求失败")
                            .to_string())
                    } else {
                        Ok(message["result"].clone())
                    };
                    let _ = sender.send(result);
                }
            }
            return;
        }
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        let thread_id = params["threadId"].as_str().unwrap_or("");
        let mut store = self.store.lock().unwrap();
        let Some(chat) = store.chats.iter_mut().find(|c| c.id == thread_id) else {
            return;
        };
        if message.get("id").is_some() {
            chat.requests.push(message);
            return;
        }
        match method {
            "turn/started" => {
                chat.active_turn = params["turn"]["id"].as_str().map(String::from);
                chat.activity = Some("正在思考".into());
            }
            "turn/completed" => {
                if let Some(id) = params["turn"]["id"].as_str() {
                    self.completed_turns.lock().unwrap().insert(id.into());
                }
                chat.active_turn = None;
                chat.activity = None;
                chat.requests.clear();
                chat.error = params["turn"]["error"]["message"]
                    .as_str()
                    .map(String::from);
                if params["turn"]["status"] == "interrupted" {
                    chat.activity = Some("已停止".into());
                }
            }
            "error" => {
                chat.error = params["error"]["message"].as_str().map(String::from);
            }
            "item/agentMessage/delta" => {
                let id = params["itemId"].as_str().unwrap_or("");
                let delta = params["delta"].as_str().unwrap_or("");
                if let Some(item) = chat.messages.iter_mut().find(|m| m.id == id) {
                    item.text.push_str(delta);
                } else {
                    chat.messages.push(Message {
                        id: id.into(),
                        role: "assistant".into(),
                        text: delta.into(),
                    });
                }
                chat.activity = Some("正在回复".into());
            }
            "item/started" | "item/completed" => {
                let item = &params["item"];
                if item["type"] == "agentMessage" {
                    let id = item["id"].as_str().unwrap_or("");
                    if let Some(text) = item["text"].as_str().filter(|text| !text.is_empty()) {
                        if let Some(existing) = chat.messages.iter_mut().find(|m| m.id == id) {
                            existing.text = text.into();
                        } else {
                            chat.messages.push(Message {
                                id: id.into(),
                                role: "assistant".into(),
                                text: text.into(),
                            });
                        }
                    }
                } else if item["type"] == "commandExecution" {
                    chat.activity = Some("正在执行命令".into());
                } else if item["type"] == "fileChange" {
                    chat.activity = Some("正在更新文件".into());
                }
            }
            _ => {}
        }
        drop(store);
        if method == "turn/completed" {
            let _ = self.persist();
        }
    }

    pub fn stop_process(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.ready.store(false, Ordering::SeqCst);
        *self.writer.lock().unwrap() = None;
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

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop_process();
    }
}

fn atomic_json(path: &Path, value: &Value) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(value).unwrap())
        .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}

pub fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix("\\\\?\\").unwrap_or(&value).to_string()
}

fn messages_from_turns(turns: &[Value]) -> Vec<Message> {
    let mut messages = vec![];
    for turn in turns {
        if let Some(items) = turn["items"].as_array() {
            for item in items {
                let role = match item["type"].as_str() {
                    Some("userMessage") => "user",
                    Some("agentMessage") => "assistant",
                    _ => continue,
                };
                let text = if role == "assistant" {
                    item["text"].as_str().unwrap_or("").to_string()
                } else {
                    item["content"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|p| p["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                messages.push(Message {
                    id: item["id"].as_str().unwrap_or("").into(),
                    role: role.into(),
                    text,
                });
            }
        }
    }
    messages
}
