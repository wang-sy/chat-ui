use crate::space::Space;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::process::{Child, ChildStdin};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{mpsc, Arc, Condvar, Mutex};

pub use system::hidden_command;
pub(crate) use system::{discover, display_path, home_dir, messages_from_turns};
pub(crate) use types::{
    normalized_permission, permission_settings, service_tier_for_speed, Chat, Message, Project,
    Reply, Store,
};

mod chat_service;
mod codex_process;
mod project_service;
mod space_service;
mod state;
mod system;
mod transcript;
mod types;

pub struct Runtime {
    pub store: Mutex<Store>,
    pub spaces: Mutex<HashMap<String, Space>>,
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
    orchestrated_turns: Mutex<HashSet<String>>,
    persist_gate: Mutex<()>,
    event_log: Mutex<EventLog>,
    event_signal: Condvar,
    instance_id: String,
    state_dir: PathBuf,
}

pub(crate) struct EventLog {
    next: u64,
    entries: VecDeque<(u64, Arc<str>)>,
    buffered_bytes: usize,
}

impl Default for EventLog {
    fn default() -> Self {
        Self {
            next: 1,
            entries: VecDeque::with_capacity(256),
            buffered_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn permission_modes_map_to_explicit_protocol_policies() {
        let (_, reviewer, _, full) = permission_settings("full", "/tmp/project");
        assert_eq!(reviewer, "user");
        assert_eq!(full, json!({"type":"dangerFullAccess"}));
        let (_, reviewer, _, workspace) = permission_settings("auto", "/tmp/project");
        assert_eq!(reviewer, "auto_review");
        assert_eq!(workspace["type"], "workspaceWrite");
        assert_eq!(workspace["writableRoots"][0], "/tmp/project");
    }

    #[test]
    fn unknown_permission_is_rejected_while_legacy_missing_value_defaults_safely() {
        assert_eq!(normalized_permission(None).unwrap(), "ask");
        assert!(normalized_permission(Some(&json!("unexpected"))).is_err());
    }

    #[test]
    fn speed_modes_map_to_codex_service_tiers() {
        assert_eq!(service_tier_for_speed(Some("standard")), "default");
        assert_eq!(service_tier_for_speed(Some("fast")), "priority");
        assert_eq!(service_tier_for_speed(Some("turbo")), "ultrafast");
        assert_eq!(service_tier_for_speed(None), "default");
    }

    #[test]
    fn resumed_messages_hide_orchestration_metadata() {
        let messages = messages_from_turns(&[json!({
            "items": [
                {"id":"u1","type":"userMessage","content":[{"text":"User request:\n做一个面板\n\nCurrent Space snapshot (metadata only): {}"}]},
                {"id":"a1","type":"agentMessage","text":"{\"response\":\"已创建面板\",\"plan\":null}"},
                {"id":"internal","type":"userMessage","content":[{"text":"Runtime integration pass: hidden context"}]}
            ]
        })]);
        assert_eq!(messages[0].text, "做一个面板");
        assert_eq!(messages[1].text, "已创建面板");
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn event_payload_keeps_the_sse_cursor_in_sync() {
        let runtime = Runtime::new(std::env::temp_dir().join("chat-ui-event-test"));
        runtime.publish_event("chat-ui/test", &json!({"ok":true}));
        let events = runtime.wait_for_events(0, std::time::Duration::ZERO);
        assert_eq!(events.len(), 1);
        let payload: Value = serde_json::from_str(&events[0].1).unwrap();
        assert_eq!(events[0].0, payload["sequence"]);
        assert_eq!(payload["method"], "chat-ui/test");
    }

    #[test]
    fn streaming_replay_is_byte_bounded_and_shares_encoded_snapshots() {
        let runtime = Runtime::new(std::env::temp_dir().join("chat-ui-stream-budget-test"));
        let large = "x".repeat(900_000);
        for index in 0..14 {
            runtime.publish_event("test/delta", &json!({"index":index,"output":large}));
        }
        let log = runtime.event_log.lock().unwrap();
        assert!(log.buffered_bytes <= 8 * 1024 * 1024);
        assert!(log.entries.len() < 14);
        let latest = log.entries.back().unwrap().1.clone();
        drop(log);
        let batch = runtime.wait_for_events(0, std::time::Duration::ZERO);
        assert!(batch.len() <= 16);
        assert!(Arc::ptr_eq(&latest, &batch.last().unwrap().1));
        let payload: Value = serde_json::from_str(&latest).unwrap();
        assert_eq!(payload["params"]["index"], 13);
    }
}
