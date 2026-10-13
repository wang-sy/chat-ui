use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub(crate) type Reply = Result<Value, String>;

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
    #[serde(default)]
    pub effort: String,
    #[serde(default)]
    pub speed: String,
    #[serde(default)]
    pub permission: String,
    /// `main` owns Space composition; `view` owns one isolated page.
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub view_id: Option<String>,
    #[serde(default)]
    pub parent_thread_id: Option<String>,
    pub messages: Vec<Message>,
    /// Readable, ordered messages and public execution events. Raw model output
    /// remains in `messages` until orchestration consumes its structured result.
    #[serde(default)]
    pub transcript: Vec<super::transcript::TranscriptItem>,
    pub active_turn: Option<String>,
    pub activity: Option<String>,
    #[serde(default)]
    pub activity_detail: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub turn_plan: Option<Value>,
    #[serde(default)]
    pub token_usage: Option<Value>,
    #[serde(default)]
    pub requests: Vec<Value>,
}

pub(crate) fn permission_settings(
    mode: &str,
    cwd: &str,
) -> (&'static str, &'static str, &'static str, Value) {
    match mode {
        "auto" => (
            "on-request",
            "auto_review",
            "workspace-write",
            json!({
                "type": "workspaceWrite",
                "writableRoots": [cwd],
                "networkAccess": false
            }),
        ),
        "full" => (
            "never",
            "user",
            "danger-full-access",
            json!({"type": "dangerFullAccess"}),
        ),
        _ => (
            "on-request",
            "user",
            "workspace-write",
            json!({
                "type": "workspaceWrite",
                "writableRoots": [cwd],
                "networkAccess": false
            }),
        ),
    }
}

pub(crate) fn normalized_permission(value: Option<&Value>) -> Result<&'static str, String> {
    match value.and_then(Value::as_str) {
        None | Some("ask") => Ok("ask"),
        Some("auto") => Ok("auto"),
        Some("full") => Ok("full"),
        Some(_) => Err("无效的权限模式".into()),
    }
}

pub(crate) fn service_tier_for_speed(value: Option<&str>) -> &'static str {
    match value {
        // The model catalog calls these product speeds `fast` and
        // `ultrafast`, while the app-server protocol calls the paid fast tier
        // `priority`. Keep this translation in one place so the UI label and
        // the requested Codex tier cannot drift apart.
        Some("fast") => "priority",
        Some("turbo") => "ultrafast",
        _ => "default",
    }
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
