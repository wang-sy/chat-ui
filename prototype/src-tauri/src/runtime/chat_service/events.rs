use crate::runtime::{Message, Runtime};
use serde_json::{json, Value};
use std::sync::Arc;

impl Runtime {
    pub(crate) fn receive(self: &Arc<Self>, message: Value) {
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
        let method = message["method"].as_str().unwrap_or("").to_string();
        let params = message["params"].clone();
        let thread_id = params["threadId"].as_str().unwrap_or("");
        let completed_turn_id = (method == "turn/completed"
            && params["turn"]["status"] == "completed"
            && params["turn"]["error"].is_null())
        .then(|| params["turn"]["id"].as_str().unwrap_or("").to_string())
        .filter(|id| !id.is_empty());
        let mut store = self.store.lock().unwrap();
        let Some(chat) = store.chats.iter_mut().find(|c| c.id == thread_id) else {
            drop(store);
            self.publish_event(&method, &params);
            return;
        };
        let view_context = (method == "turn/completed" && chat.role == "view")
            .then(|| (chat.cwd.clone(), chat.view_id.clone()));
        let main_cwd =
            (method == "turn/completed" && chat.role != "view").then(|| chat.cwd.clone());
        if message.get("id").is_some() {
            chat.requests.push(message);
            drop(store);
            self.publish_event(&method, &params);
            return;
        }
        if method.starts_with("item/")
            && !method.ends_with("/started")
            && chat.activity.as_deref() == Some("正在重新连接 Codex")
        {
            chat.activity = Some("正在工作".into());
            chat.activity_detail = None;
        }
        match method.as_str() {
            "thread/status/changed" => {
                let status = params["status"]["type"].as_str().unwrap_or("");
                chat.activity = match status {
                    "active" => Some(
                        if params["status"]["activeFlags"]
                            .as_array()
                            .is_some_and(|flags| {
                                flags
                                    .iter()
                                    .any(|flag| flag.as_str() == Some("waitingOnApproval"))
                            })
                        {
                            "等待你的确认"
                        } else if params["status"]["activeFlags"]
                            .as_array()
                            .is_some_and(|flags| {
                                flags
                                    .iter()
                                    .any(|flag| flag.as_str() == Some("waitingOnUserInput"))
                            })
                        {
                            "等待你的输入"
                        } else {
                            "正在工作"
                        }
                        .into(),
                    ),
                    "idle" => {
                        chat.active_turn = None;
                        chat.activity_detail = None;
                        None
                    }
                    "systemError" => Some("Codex 服务遇到错误".into()),
                    _ => chat.activity.clone(),
                };
            }
            "turn/started" => {
                chat.active_turn = params["turn"]["id"].as_str().map(String::from);
                chat.activity = Some("正在思考".into());
                chat.activity_detail = None;
                // `last` belongs to the previous turn until Codex publishes
                // the first usage update for this one. Do not show that old
                // number beside a fresh "正在思考" status.
                chat.token_usage = None;
                chat.error = None;
                chat.turn_plan = None;
            }
            "turn/completed" => {
                if let Some(id) = params["turn"]["id"].as_str() {
                    self.completed_turns.lock().unwrap().insert(id.into());
                }
                chat.active_turn = None;
                chat.activity = None;
                chat.activity_detail = None;
                chat.requests.clear();
                chat.error = params["turn"]["error"]["message"]
                    .as_str()
                    .map(String::from);
                if params["turn"]["status"] == "interrupted" {
                    chat.activity = Some("已停止".into());
                }
            }
            "error" => {
                let detail = params["error"]["message"].as_str().map(String::from);
                if params["willRetry"] == true {
                    chat.activity = Some("正在重新连接 Codex".into());
                    chat.activity_detail = detail;
                    chat.error = None;
                } else {
                    chat.error = detail;
                }
            }
            // Advisory notices stay in the event stream; they are not turn failures.
            "warning" => {}
            "serverRequest/resolved" => {
                chat.requests
                    .retain(|request| request["id"] != params["requestId"]);
            }
            "turn/plan/updated" => {
                chat.turn_plan = Some(json!({
                    "explanation": params["explanation"],
                    "plan": params["plan"],
                }));
                chat.activity = Some("正在规划".into());
            }
            "thread/tokenUsage/updated" => {
                chat.token_usage = Some(params["tokenUsage"].clone());
            }
            "item/plan/delta" => {
                chat.activity = Some("正在规划".into());
            }
            "item/reasoning/summaryTextDelta" | "item/reasoning/textDelta" => {
                chat.activity = Some("正在推理".into());
            }
            "item/commandExecution/outputDelta"
            | "item/commandExecution/started"
            | "item/fileChange/outputDelta" => {
                chat.activity = Some("正在执行命令".into());
            }
            "item/mcpToolCall/progress" => {
                chat.activity = params["message"].as_str().map(String::from);
            }
            "model/rerouted" => {
                chat.activity = params["toModel"]
                    .as_str()
                    .map(|model| format!("已切换到 {model}"));
                if let Some(model) = params["toModel"].as_str() {
                    chat.model = model.into();
                }
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
        if let Some(detail) = activity_detail(&method, &params) {
            chat.activity_detail = Some(detail);
        }
        crate::runtime::transcript::record_event(chat, &method, &params);
        if let Some(item) = chat
            .transcript
            .iter()
            .rev()
            .find(|entry| params["itemId"] == entry.id && !entry.text.trim().is_empty())
        {
            chat.activity_detail = Some(compact_detail(&item.text, 1200));
        }
        drop(store);
        self.publish_event(&method, &params);
        if method == "turn/completed" {
            if params["turn"]["status"] != "completed" || !params["turn"]["error"].is_null() {
                if let Some(cwd) = main_cwd {
                    self.finish_integration(&cwd, "failed");
                }
                if let Some((cwd, Some(view_id))) = view_context {
                    self.set_view_status(&cwd, &view_id, "error");
                }
            }
            let _ = self.persist();
            if let Some(_turn_id) = completed_turn_id
                .filter(|id| self.orchestrated_turns.lock().unwrap().insert(id.clone()))
            {
                let thread_id = thread_id.to_string();
                let runtime = Arc::clone(self);
                std::thread::spawn(move || runtime.handle_completed_turn(&thread_id));
            }
        }
    }
}

fn activity_detail(method: &str, params: &Value) -> Option<String> {
    let detail = match method {
        "item/commandExecution/outputDelta"
        | "item/commandExecution/started"
        | "item/fileChange/outputDelta" => params["delta"]
            .as_str()
            .or_else(|| params["output"].as_str())
            .or_else(|| params["text"].as_str())
            .map(str::to_owned)
            .or_else(|| command_text(&params["item"])),
        "item/reasoning/summaryTextDelta" => params["delta"]
            .as_str()
            .or_else(|| params["text"].as_str())
            .map(str::to_owned),
        "item/mcpToolCall/progress" => params["message"].as_str().map(str::to_owned),
        "item/agentMessage/delta" => params["delta"].as_str().map(str::to_owned),
        "turn/plan/updated" => params["explanation"].as_str().map(str::to_owned),
        "model/rerouted" => params["toModel"].as_str().map(str::to_owned),
        "item/started" | "item/completed" => {
            let item = &params["item"];
            command_text(item)
                .or_else(|| item["description"].as_str().map(str::to_owned))
                .or_else(|| item["title"].as_str().map(str::to_owned))
        }
        _ => None,
    }?;
    let detail = detail.trim();
    if detail.is_empty() {
        return None;
    }
    Some(compact_detail(detail, 1200))
}

fn command_text(item: &Value) -> Option<String> {
    if let Some(command) = item["command"].as_str() {
        return Some(command.to_owned());
    }
    item["command"].as_array().map(|parts| {
        parts
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn compact_detail(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let compact: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        return compact;
    }
    let tail: String = value
        .chars()
        .rev()
        .take(max_chars.saturating_sub(1))
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("…{tail}")
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
