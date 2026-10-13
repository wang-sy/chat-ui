use crate::runtime::Chat;
use crate::space::{Space, SpacePlan};
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub(crate) struct TurnSettings {
    pub model: String,
    pub effort: String,
    pub speed: String,
    pub permission: String,
}

impl TurnSettings {
    pub(crate) fn new(model: &str, effort: &str, speed: &str, permission: &str) -> Self {
        Self {
            model: model.into(),
            effort: effort.into(),
            speed: speed.into(),
            permission: permission.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ViewTask {
    pub cwd: String,
    pub parent_thread_id: String,
    pub thread_id: Option<String>,
    pub view_id: String,
    pub title: String,
    pub brief: String,
    pub settings: TurnSettings,
}

pub(crate) const MAIN_THREAD_INSTRUCTIONS: &str = concat!(
    include_str!("prompts/main.md"),
    include_str!("prompts/progress.md"),
    include_str!("prompts/visual.md")
);

pub(crate) fn view_thread_instructions(view_id: &str, title: &str, brief: &str) -> String {
    format!(
        "{}{}{}\n\nAssigned View: {title:?} (id: {view_id})\nContract:\n{brief}",
        include_str!("prompts/view.md"),
        include_str!("prompts/progress.md"),
        include_str!("prompts/visual.md")
    )
}

pub(crate) fn main_output_schema() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["response","plan"],
        "properties": {
            "response":{"type":"string"},
            "plan":{"anyOf":[{"type":"null"},{
                "type":"object", "additionalProperties":false,
                "required":["kind","mainPage","views","connections","removeViews"],
                "properties": {
                    "kind":{"type":"string","enum":["space_plan"]},
                    "mainPage":{"type":"object","additionalProperties":false,"required":["title","summary","html"],"properties":{"title":{"type":"string"},"summary":{"type":"string"},"html":{"type":"string"}}},
                    "views":{"type":"array","maxItems":32,"items":{"type":"object","additionalProperties":false,"required":["id","title","kind","brief","x","y","width","height"],"properties":{"id":{"type":"string"},"title":{"type":"string"},"kind":{"type":"string"},"brief":{"type":"string"},"x":{"type":"number"},"y":{"type":"number"},"width":{"type":"number"},"height":{"type":"number"}}}},
                    "connections":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["from","to","label"],"properties":{"from":{"type":"string"},"to":{"type":"string"},"label":{"anyOf":[{"type":"string"},{"type":"null"}]}}}},
                    "removeViews":{"type":"array","maxItems":32,"items":{"type":"string"}}
                }
            }]}
        }
    })
}

pub(crate) fn view_output_schema() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["response","page"],
        "properties": {
            "response":{"type":"string"},
            "page":{"type":"object","additionalProperties":false,"required":["title","summary","html","status"],"properties":{"title":{"type":"string"},"summary":{"type":"string"},"html":{"type":"string"},"status":{"type":"string","enum":["ready","error"]}}}
        }
    })
}

/// A synthesis pass cannot reassign or remove child work, even if the model asks.
pub(crate) fn integration_output_schema() -> Value {
    let mut schema = main_output_schema();
    let mut plan = schema["properties"]["plan"]["anyOf"][1].clone();
    plan["properties"]["views"]["maxItems"] = json!(0);
    plan["properties"]["removeViews"]["maxItems"] = json!(0);
    schema["properties"]["plan"] = plan;
    schema
}

pub(crate) fn validate_integration_plan(plan: &SpacePlan) -> Result<(), &'static str> {
    if !plan.views.is_empty() || !plan.remove_views.is_empty() {
        return Err("整合阶段不能重新分派或删除子 View，已有结果已保留。");
    }
    if !plan.main_page.html.contains("<chat-view") {
        return Err("整合结果缺少内嵌子 View，已有页面已保留。");
    }
    Ok(())
}

pub(crate) fn chat_role(chat: &Chat) -> &str {
    if chat.role == "view" {
        "view"
    } else {
        "main"
    }
}

pub(crate) fn structured_object(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = trimmed
        .strip_prefix("```json")
        .and_then(|value| value.strip_suffix("```"))
        .or_else(|| {
            trimmed
                .strip_prefix("```json\n")
                .and_then(|value| value.strip_suffix("\n```"))
        })
        .unwrap_or(trimmed)
        .trim();
    if let Ok(value) = serde_json::from_str::<Value>(candidate) {
        return if value.is_object() { Some(value) } else { None };
    }
    // Models occasionally add one explanatory sentence around the required
    // object.  Parse the first complete JSON value without accepting arbitrary
    // prose as a plan.
    let start = candidate.find('{')?;
    serde_json::Deserializer::from_str(&candidate[start..])
        .into_iter::<Value>()
        .next()
        .and_then(Result::ok)
        .filter(|value| value.is_object())
}

pub(crate) fn response_text_and_payload(text: &str) -> (String, Option<Value>) {
    let Some(value) = structured_object(text) else {
        return (text.into(), None);
    };
    let response = value["response"].as_str().unwrap_or("").trim().to_string();
    (
        if response.is_empty() {
            text.into()
        } else {
            response
        },
        Some(value),
    )
}

pub(crate) fn plan_from_payload(payload: &Value) -> Result<Option<SpacePlan>, String> {
    let plan = if payload.get("kind").and_then(Value::as_str) == Some("space_plan") {
        payload.clone()
    } else {
        let Some(plan) = payload.get("plan") else {
            return Err("Main Thread 缺少 plan 字段".into());
        };
        if plan.is_null() {
            return Ok(None);
        }
        plan.clone()
    };
    if plan.get("kind").and_then(Value::as_str) != Some("space_plan") {
        return Err("Main Thread 返回的 Space 计划类型无效".into());
    }
    serde_json::from_value(plan)
        .map(Some)
        .map_err(|_| "Main Thread 返回的 Space 计划格式不完整".into())
}

pub(crate) fn page_from_payload(payload: &Value) -> Option<&Value> {
    let page = payload.get("page")?;
    (page.get("html").and_then(Value::as_str).is_some()).then_some(page)
}

/// Give Main Thread a compact, current snapshot on every turn. This keeps
/// orchestration grounded after a child finishes without leaking full HTML or
/// another View's private implementation into the prompt.
pub(crate) fn space_context(space: &Space) -> String {
    let context = json!({
        "revision": space.revision,
        "composition": {
            "mode": "all-views-embedded",
            "defaultExpanded": true,
            "requiredViewIds": space.views.iter().map(|view| &view.id).collect::<Vec<_>>()
        },
        "mainPage": {
            "title": space.main_page.title,
            "summary": space.main_page.summary,
            "version": space.main_page.version
        },
        "views": space.views.iter().map(|view| json!({
            "id": view.id,
            "key": view.key,
            "title": view.title,
            "kind": view.kind,
            "status": view.status,
            "version": view.version,
            "brief": view.brief,
            "summary": view.summary,
            "threadId": view.thread_id
        })).collect::<Vec<_>>(),
        "connections": space.connections
    });
    context.to_string().chars().take(24_000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_c01_main_plan_parser_distinguishes_noop_from_invalid_output() {
        assert!(matches!(
            plan_from_payload(&json!({"response":"ok","plan":null})),
            Ok(None)
        ));
        assert!(plan_from_payload(&json!({"response":"ok"})).is_err());
        assert!(plan_from_payload(&json!({"response":"ok","plan":{}})).is_err());
    }

    #[test]
    fn space_context_contains_owned_metadata_without_page_html() {
        let space = Space::fixture("/tmp/chat-ui");
        let context = space_context(&space);
        assert!(context.contains("architecture"));
        assert!(!context.contains("<!doctype html>"));
    }

    #[test]
    fn integration_requires_embeds_and_cannot_schedule_child_work() {
        let mut plan = SpacePlan::default();
        plan.main_page.html = "<a href='#architecture'>Architecture</a>".into();
        assert!(validate_integration_plan(&plan).is_err());
        plan.main_page.html = "<chat-view view-id='architecture'></chat-view>".into();
        assert!(validate_integration_plan(&plan).is_ok());
        plan.remove_views.push("architecture".into());
        assert!(validate_integration_plan(&plan).is_err());
        let schema = integration_output_schema();
        assert_eq!(
            schema["properties"]["plan"]["properties"]["views"]["maxItems"],
            0
        );
        assert_eq!(
            schema["properties"]["plan"]["properties"]["removeViews"]["maxItems"],
            0
        );
    }
}
