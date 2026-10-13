use super::{text, TranscriptItem};
use serde_json::Value;

pub(super) fn from_item(item: &Value, turn_id: &str, status: &str) -> Option<TranscriptItem> {
    let (kind, title, body) = match item["type"].as_str()? {
        "userMessage" => (
            "user",
            "你".into(),
            text::user_text(
                &item["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|part| part["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ),
        "agentMessage" => (
            "assistant",
            if item["phase"] == "commentary" {
                "进展"
            } else {
                "回复"
            }
            .into(),
            text::assistant_text(item["text"].as_str().unwrap_or("")),
        ),
        // Only the public summary is shown; private/raw reasoning is not a chat message.
        "reasoning" => ("reasoning", "思考摘要".into(), lines(&item["summary"])),
        "commandExecution" => (
            "command",
            command(item),
            item["aggregatedOutput"].as_str().unwrap_or("").into(),
        ),
        "fileChange" => (
            "file",
            "更新文件".into(),
            item["changes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|change| change["path"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        "mcpToolCall" => (
            "tool",
            format!(
                "{} · {}",
                item["server"].as_str().unwrap_or("工具"),
                item["tool"].as_str().unwrap_or("调用")
            ),
            tool_result(item),
        ),
        "dynamicToolCall" => (
            "tool",
            item["tool"].as_str().unwrap_or("工具调用").into(),
            tool_result(item),
        ),
        "webSearch" => (
            "tool",
            "搜索网页".into(),
            item["query"].as_str().unwrap_or("").into(),
        ),
        "plan" => (
            "plan",
            "执行计划".into(),
            item["text"].as_str().unwrap_or("").into(),
        ),
        "contextCompaction" => ("notice", "整理上下文".into(), String::new()),
        _ => return None,
    };
    if kind == "user" && body.trim().is_empty() {
        return None;
    }
    let failed = item["status"] == "failed"
        || item["success"] == false
        || item["exitCode"].as_i64().is_some_and(|code| code != 0)
        || !item["error"].is_null();
    Some(TranscriptItem {
        id: item["id"].as_str()?.into(),
        turn_id: turn_id.into(),
        kind: kind.into(),
        title,
        text: if matches!(kind, "user" | "assistant") {
            body
        } else {
            text::bounded_output(&body)
        },
        status: if failed {
            "failed".into()
        } else {
            status.into()
        },
    })
}

fn lines(value: &Value) -> String {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().or_else(|| v["text"].as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn command(item: &Value) -> String {
    item["command"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let command = item["command"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            if command.is_empty() {
                "运行命令".into()
            } else {
                command
            }
        })
}

fn tool_result(item: &Value) -> String {
    let result = item.get("result").unwrap_or(&Value::Null);
    let body = result["content"]
        .as_array()
        .or_else(|| item["contentItems"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|part| part["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if body.is_empty() {
        item["error"]["message"].as_str().unwrap_or("").into()
    } else {
        body
    }
}
