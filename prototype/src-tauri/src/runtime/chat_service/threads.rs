use super::protocol::*;
use crate::runtime::{
    messages_from_turns, normalized_permission, permission_settings, Reply, Runtime,
};
use serde_json::{json, Value};
use std::sync::Arc;

impl Runtime {
    pub fn open_chat(self: &Arc<Self>, id: &str) -> Reply {
        self.connect()?;
        let _gate = self.mutation_gate.lock().unwrap();
        self.resume(id)?;
        self.persist()?;
        self.publish_event("chat-ui/chat/historyLoaded", &json!({"threadId":id}));
        Ok(json!({"id":id}))
    }

    pub(crate) fn resume(&self, id: &str) -> Result<(), String> {
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
        let permission = if chat.permission.is_empty() {
            "ask"
        } else {
            normalized_permission(Some(&Value::String(chat.permission.clone())))?
        };
        let (approval_policy, approvals_reviewer, sandbox, _) =
            permission_settings(permission, &chat.cwd);
        let developer_instructions = if chat_role(&chat) == "view" {
            let view = self
                .spaces
                .lock()
                .unwrap()
                .get(&chat.cwd)
                .and_then(|space| {
                    chat.view_id
                        .as_deref()
                        .and_then(|id| space.views.iter().find(|view| view.id == id))
                        .cloned()
                });
            view.map(|view| view_thread_instructions(&view.id, &view.title, &view.brief))
                .unwrap_or_else(|| {
                    view_thread_instructions(
                        chat.view_id.as_deref().unwrap_or("unknown"),
                        &chat.title,
                        "继续维护这个 View 页面",
                    )
                })
        } else {
            MAIN_THREAD_INSTRUCTIONS.into()
        };
        self.rpc("thread/resume", json!({"threadId":id,"cwd":chat.cwd,"approvalPolicy":approval_policy,"approvalsReviewer":approvals_reviewer,"sandbox":sandbox,"developerInstructions":developer_instructions,"excludeTurns":true}))?;
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
                    crate::runtime::transcript::restore(chat, turns);
                }
            }
        }
        self.resumed.lock().unwrap().insert(id.to_string());
        Ok(())
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
}
