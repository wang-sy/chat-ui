use super::protocol::*;
use crate::runtime::{permission_settings, service_tier_for_speed, Reply, Runtime};
use serde_json::json;

impl Runtime {
    pub(super) fn launch_turn(
        &self,
        thread_id: &str,
        cwd: &str,
        settings: &TurnSettings,
        text: &str,
        role: &str,
    ) -> Reply {
        let (approval_policy, approvals_reviewer, _, sandbox_policy) =
            permission_settings(&settings.permission, cwd);
        let mut params = json!({
            "threadId":thread_id,
            "summary":"auto",
            "model":settings.model,
            "serviceTierForTurn":service_tier_for_speed(Some(&settings.speed)),
            "approvalPolicy":approval_policy,
            "approvalsReviewer":approvals_reviewer,
            "sandboxPolicy":sandbox_policy,
            "input":[{"type":"text","text":text}],
            "outputSchema": match role { "view" => view_output_schema(), "integration" => integration_output_schema(), _ => main_output_schema() }
        });
        if !settings.effort.is_empty() {
            params["effort"] = json!(settings.effort);
        }
        let result = self.rpc("turn/start", params);
        match &result {
            Ok(response) => {
                let turn_id = response["turn"]["id"].as_str().unwrap_or("");
                if let Ok(mut store) = self.store.lock() {
                    if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                        if !turn_id.is_empty()
                            && !self.completed_turns.lock().unwrap().contains(turn_id)
                        {
                            chat.active_turn = Some(turn_id.into());
                        }
                    }
                }
            }
            Err(error) => {
                if let Ok(mut store) = self.store.lock() {
                    if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                        chat.active_turn = None;
                        chat.activity = None;
                        chat.error = Some(error.clone());
                    }
                }
            }
        }
        result
    }
}
