use super::protocol::{space_context, TurnSettings};
use crate::runtime::Runtime;
use serde_json::json;
use std::sync::Arc;

impl Runtime {
    /// One synthesis turn for a Main assignment, after all assigned artifacts
    /// are actually ready. The mutation gate also serializes user sends.
    pub(crate) fn try_integrate(self: &Arc<Self>, cwd: &str) {
        let _gate = self.mutation_gate.lock().unwrap();
        let Some(space) = self
            .spaces
            .lock()
            .ok()
            .and_then(|spaces| spaces.get(cwd).cloned())
        else {
            return;
        };
        if !space.integration_ready() {
            return;
        }
        let Some(id) = space.main_thread_id.as_deref() else {
            return;
        };
        let Some(chat) = self.store.lock().ok().and_then(|store| {
            store
                .chats
                .iter()
                .find(|chat| chat.id == id && chat.active_turn.is_none())
                .cloned()
        }) else {
            return;
        };
        if let Err(error) = self.resume(id) {
            self.finish_integration(cwd, "failed");
            self.set_chat_error(id, &error);
            return;
        }
        self.finish_integration(cwd, "running");
        if let Some(chat) = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter_mut()
            .find(|chat| chat.id == id)
        {
            chat.active_turn = Some("starting".into());
            chat.activity = Some("正在整合 View 结果".into());
            chat.activity_detail = Some("把已完成的子图、指标和关系整合进主页面".into());
            chat.error = None;
        }
        let prompt = format!(
            "Runtime integration pass: your assigned View artifacts are now ready. \
             Embed EVERY retained View in Main Page using live <chat-view> elements. \
             These are bare iframes with automatic height, without host titles, status badges, \
             borders or collapse controls. You own the layout and interaction; use folding \
             only when useful, never a mandatory card or accordion around every result. \
             Prefer showing results together. Do not replace them with a directory of links. \
             Apply the shared visual decision policy: compose the real diagrams/charts; add \
             a compact relationship overview only when it explains cross-View meaning. \
             Do not duplicate a single child's visual or invent causal arrows for comparisons. \
             Use factual summaries/provenance below. Return a space_plan with views:[] and \
             removeViews:[]; update ONLY mainPage and connections. Do not run tools, write files, \
             assign work or regenerate any child. Existing slots will receive live child HTML.\n\n{}",
            space_context(&space)
        );
        self.publish_event(
            "chat-ui/main/integrationStarted",
            &json!({"cwd":cwd,"threadId":id}),
        );
        let settings = TurnSettings::new(&chat.model, &chat.effort, &chat.speed, &chat.permission);
        if let Err(error) = self.launch_turn(id, cwd, &settings, &prompt, "integration") {
            self.finish_integration(cwd, "failed");
            self.set_chat_error(id, &format!("整合未完成，已有子结果仍可用：{error}"));
        }
        let _ = self.persist();
    }

    pub(crate) fn is_integrating(&self, cwd: &str) -> bool {
        self.spaces.lock().ok().is_some_and(|spaces| {
            spaces
                .get(cwd)
                .and_then(|space| space.integration.as_ref())
                .is_some_and(|run| run.status == "running")
        })
    }

    pub(crate) fn finish_integration(&self, cwd: &str, status: &str) {
        if let Ok(mut spaces) = self.spaces.lock() {
            if let Some(space) = spaces.get_mut(cwd) {
                if let Some(run) = &mut space.integration {
                    run.status = status.into();
                    space.revision = space.revision.saturating_add(1);
                }
            }
        }
    }
}
