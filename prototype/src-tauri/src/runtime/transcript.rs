use super::Chat;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod items;
mod text;
pub(crate) use text::{assistant_text, user_text};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptItem {
    pub id: String,
    pub turn_id: String,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub status: String,
}

pub(crate) fn seed(chat: &mut Chat) {
    if !chat.transcript.is_empty() {
        return;
    }
    for message in &chat.messages {
        let text = if message.role == "user" {
            user_text(&message.text)
        } else {
            assistant_text(&message.text)
        };
        if text.trim().is_empty() {
            continue;
        }
        chat.transcript.push(TranscriptItem {
            id: message.id.clone(),
            kind: message.role.clone(),
            text,
            title: if message.role == "user" {
                "你"
            } else {
                "回复"
            }
            .into(),
            status: "completed".into(),
            ..TranscriptItem::default()
        });
    }
}

pub(crate) fn record_user(chat: &mut Chat, id: &str, text: &str) {
    seed(chat);
    let text = user_text(text);
    if text.trim().is_empty() {
        return;
    }
    upsert(
        chat,
        TranscriptItem {
            id: id.into(),
            kind: "user".into(),
            title: "你".into(),
            text,
            status: "completed".into(),
            ..TranscriptItem::default()
        },
    );
}

fn upsert(chat: &mut Chat, mut item: TranscriptItem) {
    if let Some(existing) = chat.transcript.iter_mut().find(|entry| entry.id == item.id) {
        if item.text.is_empty() {
            item.text.clone_from(&existing.text);
        }
        *existing = item;
    } else if item.kind == "user" {
        // Local acceptance precedes Codex's canonical userMessage ID. Replace
        // that pending entry once; identical requests on later turns stay distinct.
        if let Some(pending) = chat.transcript.iter_mut().rev().find(|entry| {
            entry.kind == "user" && entry.turn_id.is_empty() && entry.text == item.text
        }) {
            *pending = item;
        } else {
            chat.transcript.push(item);
        }
    } else {
        chat.transcript.push(item);
    }
}

pub(crate) fn record_event(chat: &mut Chat, method: &str, params: &Value) {
    seed(chat);
    let turn_id = params["turnId"]
        .as_str()
        .or_else(|| params["turn"]["id"].as_str())
        .or(chat.active_turn.as_deref())
        .unwrap_or("")
        .to_string();
    if method.starts_with("item/") {
        if let Some(retry) = chat
            .transcript
            .iter_mut()
            .find(|item| item.id == format!("retry-{turn_id}") && item.status == "running")
        {
            retry.status = "completed".into();
            retry.title = "连接已恢复".into();
        }
    }
    match method {
        "error" => {
            let retry = params["willRetry"] == true;
            upsert(
                chat,
                TranscriptItem {
                    id: format!("{}-{turn_id}", if retry { "retry" } else { "error" }),
                    turn_id,
                    kind: "notice".into(),
                    title: if retry {
                        "正在重新连接 Codex"
                    } else {
                        "执行未完成"
                    }
                    .into(),
                    text: params["error"]["message"].as_str().unwrap_or("").into(),
                    status: if retry { "running" } else { "failed" }.into(),
                },
            );
        }
        "item/started" | "item/completed" | "item/commandExecution/started" => {
            let status = if method == "item/completed" {
                "completed"
            } else {
                "running"
            };
            if let Some(item) = items::from_item(&params["item"], &turn_id, status) {
                upsert(chat, item);
            }
        }
        "item/agentMessage/delta" => {
            let id = params["itemId"].as_str().unwrap_or("");
            if let Some(message) = chat.messages.iter().find(|message| message.id == id) {
                upsert(
                    chat,
                    TranscriptItem {
                        id: id.into(),
                        turn_id,
                        kind: "assistant".into(),
                        title: "回复".into(),
                        text: assistant_text(&message.text),
                        status: "running".into(),
                    },
                );
            }
        }
        "item/reasoning/summaryTextDelta"
        | "item/commandExecution/outputDelta"
        | "item/fileChange/outputDelta"
        | "item/plan/delta"
        | "item/mcpToolCall/progress" => {
            let (kind, title) = match method {
                "item/reasoning/summaryTextDelta" => ("reasoning", "思考摘要"),
                "item/commandExecution/outputDelta" => ("command", "运行命令"),
                "item/fileChange/outputDelta" => ("file", "更新文件"),
                "item/plan/delta" => ("plan", "执行计划"),
                _ => ("tool", "工具进度"),
            };
            let id = params["itemId"].as_str().unwrap_or("");
            if id.is_empty() {
                return;
            }
            if !chat.transcript.iter().any(|entry| entry.id == id) {
                upsert(
                    chat,
                    TranscriptItem {
                        id: id.into(),
                        turn_id,
                        kind: kind.into(),
                        title: title.into(),
                        status: "running".into(),
                        ..TranscriptItem::default()
                    },
                );
            }
            if let Some(entry) = chat.transcript.iter_mut().find(|entry| entry.id == id) {
                let delta = params["delta"]
                    .as_str()
                    .or_else(|| params["message"].as_str())
                    .unwrap_or("");
                if method == "item/mcpToolCall/progress" && !entry.text.is_empty() {
                    entry.text.push('\n');
                }
                entry.text.push_str(delta);
                entry.text = text::bounded_output(&entry.text);
            }
        }
        "item/reasoning/summaryPartAdded" => {
            if let Some(entry) = chat
                .transcript
                .iter_mut()
                .find(|entry| params["itemId"] == entry.id)
            {
                if !entry.text.is_empty() {
                    entry.text.push('\n');
                }
            }
        }
        "turn/plan/updated" => {
            let mut body = params["explanation"].as_str().unwrap_or("").to_string();
            for step in params["plan"].as_array().into_iter().flatten() {
                body.push_str(&format!(
                    "\n{} {}",
                    if step["status"] == "completed" {
                        "✓"
                    } else {
                        "·"
                    },
                    step["step"].as_str().unwrap_or("")
                ));
            }
            upsert(
                chat,
                TranscriptItem {
                    id: format!("plan-{turn_id}"),
                    turn_id,
                    kind: "plan".into(),
                    title: "执行计划".into(),
                    text: body,
                    status: "running".into(),
                },
            );
        }
        "turn/completed" => {
            let status = params["turn"]["status"].as_str().unwrap_or("completed");
            for entry in &mut chat.transcript {
                if entry.turn_id == turn_id && entry.status == "running" {
                    entry.status = status.into();
                }
            }
        }
        _ => {}
    }
}

/// Backfill historical tool steps from Codex on first open after restart. The
/// persisted transcript remains the fallback when Codex cannot read a thread.
pub(crate) fn restore(chat: &mut Chat, turns: &[Value]) {
    let mut restored = Chat::default();
    for turn in turns {
        let turn_id = turn["id"].as_str().unwrap_or("");
        for item in turn["items"].as_array().into_iter().flatten() {
            let status = if item["status"] == "inProgress" {
                "running"
            } else {
                "completed"
            };
            if let Some(item) = items::from_item(item, turn_id, status) {
                upsert(&mut restored, item);
            }
        }
        if turn["status"] != "inProgress" {
            record_event(&mut restored, "turn/completed", &json!({"turn":turn}));
        }
    }
    if restored.transcript.is_empty() {
        return;
    }
    // Preserve live-only progress and plan events that thread/read may omit.
    for saved in &chat.transcript {
        if restored.transcript.iter().any(|entry| entry.id == saved.id) {
            continue;
        }
        if matches!(saved.kind.as_str(), "user" | "assistant") {
            continue;
        }
        let index = restored
            .transcript
            .iter()
            .rposition(|entry| entry.turn_id == saved.turn_id);
        if let Some(index) = index {
            restored.transcript.insert(index + 1, saved.clone());
        }
    }
    chat.transcript = restored.transcript;
}

#[cfg(test)]
mod tests;
