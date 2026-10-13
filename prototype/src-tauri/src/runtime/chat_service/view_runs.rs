use super::protocol::*;
use crate::runtime::{permission_settings, service_tier_for_speed, Chat, Message, Runtime};
use serde_json::json;
use std::sync::Arc;

impl Runtime {
    pub(crate) fn start_view_thread(self: &Arc<Self>, task: ViewTask) {
        let _gate = match self.mutation_gate.lock() {
            Ok(gate) => gate,
            Err(_) => return,
        };
        let (approval_policy, approvals_reviewer, sandbox, _) =
            permission_settings(&task.settings.permission, &task.cwd);
        let result = self.rpc("thread/start", json!({
            "cwd":task.cwd, "model":task.settings.model, "serviceTier":service_tier_for_speed(Some(&task.settings.speed)),
            "approvalPolicy":approval_policy, "approvalsReviewer":approvals_reviewer, "sandbox":sandbox,
            "developerInstructions":view_thread_instructions(&task.view_id, &task.title, &task.brief), "threadSource":"chat_ui_view"
        }));
        let Ok(result) = result else {
            self.set_view_status(&task.cwd, &task.view_id, "error");
            return;
        };
        let Some(thread_id) = result["thread"]["id"].as_str().map(str::to_owned) else {
            self.set_view_status(&task.cwd, &task.view_id, "error");
            return;
        };
        self.store.lock().unwrap().chats.insert(
            0,
            Chat {
                id: thread_id.clone(),
                title: task.title.clone(),
                cwd: task.cwd.clone(),
                model: task.settings.model.clone(),
                effort: task.settings.effort.clone(),
                speed: task.settings.speed.clone(),
                permission: task.settings.permission.clone(),
                role: "view".into(),
                view_id: Some(task.view_id.clone()),
                parent_thread_id: Some(task.parent_thread_id.clone()),
                ..Chat::default()
            },
        );
        if let Ok(mut spaces) = self.spaces.lock() {
            if let Some(space) = spaces.get_mut(&task.cwd) {
                if let Some(view) = space.views.iter_mut().find(|view| view.id == task.view_id) {
                    view.thread_id = Some(thread_id.clone());
                }
                let _ = space.set_view_status(&task.view_id, "working");
            }
        }
        self.resumed.lock().unwrap().insert(thread_id.clone());
        let prompt = view_prompt(&task, false);
        self.prepare_chat_turn(&thread_id, &prompt);
        if self
            .launch_turn(&thread_id, &task.cwd, &task.settings, &prompt, "view")
            .is_err()
        {
            self.set_view_status(&task.cwd, &task.view_id, "error");
        }
        self.publish_event(
            "chat-ui/view/turnStarted",
            &json!({"cwd": task.cwd, "viewId": task.view_id, "threadId": thread_id}),
        );
        let _ = self.persist();
    }

    pub(crate) fn start_view_turn(self: &Arc<Self>, task: ViewTask) {
        let _gate = match self.mutation_gate.lock() {
            Ok(gate) => gate,
            Err(_) => return,
        };
        let Some(thread_id) = task.thread_id.as_deref() else {
            self.set_view_status(&task.cwd, &task.view_id, "error");
            return;
        };
        if self.resume(thread_id).is_err() {
            self.set_view_status(&task.cwd, &task.view_id, "error");
            return;
        }
        let prompt = view_prompt(&task, true);
        self.prepare_chat_turn(thread_id, &prompt);
        self.set_view_status(&task.cwd, &task.view_id, "working");
        if self
            .launch_turn(thread_id, &task.cwd, &task.settings, &prompt, "view")
            .is_err()
        {
            self.set_view_status(&task.cwd, &task.view_id, "error");
        }
        self.publish_event(
            "chat-ui/view/turnStarted",
            &json!({"cwd": task.cwd, "viewId": task.view_id, "threadId": thread_id}),
        );
        let _ = self.persist();
    }

    fn prepare_chat_turn(&self, thread_id: &str, prompt: &str) {
        if let Ok(mut store) = self.store.lock() {
            if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                chat.active_turn = Some("starting".into());
                chat.activity = Some("正在构建 View".into());
                chat.activity_detail = None;
                chat.error = None;
                chat.messages.push(Message {
                    id: format!(
                        "assignment-{}",
                        self.sequence
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    ),
                    role: "user".into(),
                    text: prompt.into(),
                });
                let id = chat.messages.last().unwrap().id.clone();
                crate::runtime::transcript::record_user(chat, &id, prompt);
            }
        }
        self.publish_event(
            "chat-ui/chat/turnPrepared",
            &json!({"threadId": thread_id, "promptBytes": prompt.len()}),
        );
    }

    pub(crate) fn set_view_status(&self, cwd: &str, view_id: &str, status: &str) {
        if let Ok(mut spaces) = self.spaces.lock() {
            if let Some(space) = spaces.get_mut(cwd) {
                let _ = space.set_view_status(view_id, status);
            }
        }
        let _ = self.persist();
        self.publish_event(
            "chat-ui/view/statusChanged",
            &json!({"cwd": cwd, "viewId": view_id, "status": status}),
        );
        self.refresh_main_orchestration(cwd);
    }

    /// Keep the Main Thread's visible activity aligned with child View work.
    /// A completed Main turn can still have several View turns in flight, so
    /// clearing its activity at the end of orchestration would make the UI
    /// look idle while the canvas is still being built.
    pub(crate) fn refresh_main_orchestration(&self, cwd: &str) {
        let Some((main_thread_id, pending_views)) = self.spaces.lock().ok().and_then(|spaces| {
            spaces.get(cwd).map(|space| {
                (
                    space.main_thread_id.clone(),
                    space
                        .views
                        .iter()
                        .filter(|view| matches!(view.status.as_str(), "planning" | "working"))
                        .count(),
                )
            })
        }) else {
            return;
        };
        let Some(main_thread_id) = main_thread_id else {
            return;
        };
        let next_activity = if pending_views > 0 {
            Some(format!("正在等待 View · {pending_views} 个"))
        } else {
            None
        };
        let should_update = self
            .store
            .lock()
            .ok()
            .and_then(|store| {
                store
                    .chats
                    .iter()
                    .find(|chat| chat.id == main_thread_id)
                    .map(|chat| {
                        // A new Main turn owns the status while it is active.
                        if chat.active_turn.is_some() {
                            return false;
                        }
                        match (&chat.activity, &next_activity) {
                            (current, Some(next)) => current.as_deref() != Some(next.as_str()),
                            (Some(current), None) => {
                                current == "正在组织 View" || current.starts_with("正在等待 View")
                            }
                            (None, None) => false,
                        }
                    })
            })
            .unwrap_or(false);
        if should_update {
            self.set_chat_activity(&main_thread_id, next_activity.as_deref());
        }
    }

    pub(crate) fn set_chat_activity(&self, thread_id: &str, activity: Option<&str>) {
        if let Ok(mut store) = self.store.lock() {
            if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                chat.activity = activity.map(str::to_owned);
            }
        }
        self.publish_event(
            "chat-ui/chat/activityChanged",
            &json!({"threadId": thread_id, "activity": activity}),
        );
    }

    pub(crate) fn set_chat_error(&self, thread_id: &str, error: &str) {
        if let Ok(mut store) = self.store.lock() {
            if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                chat.error = Some(error.to_owned());
            }
        }
        let _ = self.persist();
        self.publish_event(
            "chat-ui/chat/errorChanged",
            &json!({"threadId": thread_id, "error": error}),
        );
    }
}

fn view_prompt(task: &ViewTask, update: bool) -> String {
    if update {
        format!(
            "The Main Thread assigned a new pass for View {:?} (id {}) under parent {}.\n\nUpdated contract:\n{}\n\nUpdate only this View and return the structured View result required by your instructions.",
            task.title, task.view_id, task.parent_thread_id, task.brief
        )
    } else {
        format!(
            "Create or refine the visual artifact {:?}. Choose the representation from its goal, following the shared visual decision policy.\n\nAssigned contract:\n{}\n\nReturn the structured View result required by your instructions.",
            task.title, task.brief
        )
    }
}
