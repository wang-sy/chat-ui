use super::system::{load_store, migrate_chat_ownership, persist_all};
use super::{home_dir, Runtime};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl Runtime {
    pub fn new(state_dir: PathBuf) -> Arc<Self> {
        let (mut store, spaces) = load_store(&state_dir);
        migrate_chat_ownership(&mut store, &spaces);
        Arc::new(Self {
            store: std::sync::Mutex::new(store),
            spaces: std::sync::Mutex::new(spaces),
            child: std::sync::Mutex::new(None),
            writer: std::sync::Mutex::new(None),
            pending: std::sync::Mutex::new(std::collections::HashMap::new()),
            models: std::sync::Mutex::new(vec![]),
            issue: std::sync::Mutex::new(None),
            ready: std::sync::atomic::AtomicBool::new(false),
            sequence: std::sync::atomic::AtomicU64::new(1),
            generation: std::sync::atomic::AtomicU64::new(0),
            connect_gate: std::sync::Mutex::new(()),
            mutation_gate: std::sync::Mutex::new(()),
            resumed: std::sync::Mutex::new(std::collections::HashSet::new()),
            completed_turns: std::sync::Mutex::new(std::collections::HashSet::new()),
            orchestrated_turns: std::sync::Mutex::new(std::collections::HashSet::new()),
            persist_gate: std::sync::Mutex::new(()),
            event_log: std::sync::Mutex::new(super::EventLog::default()),
            event_signal: std::sync::Condvar::new(),
            instance_id: format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            state_dir,
        })
    }

    pub fn snapshot(&self) -> Value {
        self.snapshot_at(self.current_event_sequence())
    }

    fn snapshot_at(&self, revision: u64) -> Value {
        let store = self.store.lock().unwrap();
        json!({
            "instanceId": self.instance_id,
            "revision": revision,
            "connected": self.ready.load(Ordering::SeqCst),
            "available": super::discover().is_some(),
            "issue": *self.issue.lock().unwrap(),
            "projects": store.projects,
            "chats": store.chats,
            "models": *self.models.lock().unwrap(),
            "home": home_dir(),
        })
    }

    pub fn persist(&self) -> Result<(), String> {
        persist_all(self)
    }

    pub(crate) fn publish_event(&self, method: &str, params: &Value) {
        let Ok(mut log) = self.event_log.lock() else {
            return;
        };
        let sequence = log.next;
        log.next = log.next.saturating_add(1);
        let payload = json!({
            "sequence": sequence,
            "method": method,
            "params": params,
            "state": self.snapshot_at(sequence),
        });
        // Encode once and share the wire payload with all subscribers. A token
        // delta must not retain 512 deep copies of every thread's history.
        let encoded: Arc<str> = payload.to_string().into();
        log.buffered_bytes += encoded.len();
        log.entries.push_back((sequence, encoded));
        while log.entries.len() > 1
            && (log.entries.len() > 512 || log.buffered_bytes > 8 * 1024 * 1024)
        {
            if let Some((_, payload)) = log.entries.pop_front() {
                log.buffered_bytes -= payload.len();
            }
        }
        self.event_signal.notify_all();
    }

    pub(crate) fn current_event_sequence(&self) -> u64 {
        self.event_log
            .lock()
            .map(|log| log.next.saturating_sub(1))
            .unwrap_or_default()
    }

    pub(crate) fn wait_for_events(
        &self,
        cursor: u64,
        timeout: std::time::Duration,
    ) -> Vec<(u64, Arc<str>)> {
        let Ok(mut log) = self.event_log.lock() else {
            return Vec::new();
        };
        loop {
            let events = log
                .entries
                .iter()
                .filter(|(sequence, _)| *sequence > cursor)
                .take(16)
                .cloned()
                .collect::<Vec<_>>();
            if !events.is_empty() {
                return events;
            }
            let Ok((next, wait)) = self.event_signal.wait_timeout(log, timeout) else {
                return Vec::new();
            };
            log = next;
            if wait.timed_out() {
                return Vec::new();
            }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop_process();
    }
}
