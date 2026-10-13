use super::protocol::*;
use crate::runtime::Runtime;
use crate::space::{space_path, CompositionState, SpacePlan};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

impl Runtime {
    /// A completed Main Thread turn is the only place where a Space plan is
    /// accepted. View Threads are then started from that plan, keeping
    /// composition and page implementation on separate ownership paths.
    pub(crate) fn handle_completed_turn(self: &Arc<Self>, thread_id: &str) {
        let Some((role, cwd, view_id, latest_text, model, effort, speed, permission)) = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|chat| chat.id == thread_id)
            .map(|chat| {
                (
                    chat_role(chat).to_string(),
                    chat.cwd.clone(),
                    chat.view_id.clone(),
                    chat.messages
                        .iter()
                        .rev()
                        .find(|message| message.role == "assistant")
                        .map(|message| message.text.clone())
                        .unwrap_or_default(),
                    chat.model.clone(),
                    chat.effort.clone(),
                    chat.speed.clone(),
                    chat.permission.clone(),
                )
            })
        else {
            return;
        };
        let (display, payload) = response_text_and_payload(&latest_text);
        if !display.is_empty() && display != latest_text {
            if let Some(chat) = self
                .store
                .lock()
                .unwrap()
                .chats
                .iter_mut()
                .find(|chat| chat.id == thread_id)
            {
                if let Some(message) = chat
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|message| message.role == "assistant")
                {
                    message.text = display;
                }
            }
        }
        if role == "view" {
            self.publish_view_result(&cwd, view_id.as_deref(), payload.as_ref(), thread_id);
            return;
        }
        let Some(payload) = payload else {
            self.finish_integration(&cwd, "failed");
            self.set_chat_error(thread_id, "Main Thread 没有返回结构化结果，请重试。");
            return;
        };
        match plan_from_payload(&payload) {
            Ok(Some(plan)) => self.apply_main_plan(
                &cwd,
                thread_id,
                &plan,
                TurnSettings::new(&model, &effort, &speed, &permission),
            ),
            Ok(None) => {
                if self.is_integrating(&cwd) {
                    self.finish_integration(&cwd, "failed");
                    self.set_chat_error(thread_id, "整合未返回主页面更新，已有子结果仍可用。");
                }
            }
            Err(error) => {
                self.finish_integration(&cwd, "failed");
                self.set_chat_error(thread_id, &error);
            }
        }
        self.try_integrate(&cwd);
    }

    fn publish_view_result(
        self: &Arc<Self>,
        cwd: &str,
        view_id: Option<&str>,
        payload: Option<&serde_json::Value>,
        thread_id: &str,
    ) {
        let Some(view_id) = view_id else { return };
        let Some(page) = payload.and_then(page_from_payload) else {
            self.set_view_status(cwd, view_id, "error");
            self.set_chat_error(thread_id, "View Thread 返回的页面结果无效，请重试。");
            return;
        };
        let html = page["html"].as_str().unwrap_or("");
        let summary = page["summary"].as_str();
        let status = matches!(page["status"].as_str(), Some("ready" | "error"))
            .then(|| page["status"].as_str().unwrap())
            .unwrap_or("error");
        let updated = if let Ok(mut spaces) = self.spaces.lock() {
            if let Some(space) = spaces.get_mut(cwd) {
                space.update_view_page(view_id, html, status, summary)
            } else {
                Err("Space 不存在".into())
            }
        } else {
            Err("Space 状态锁不可用".into())
        };
        if let Err(error) = updated {
            self.set_view_status(cwd, view_id, "error");
            self.set_chat_error(thread_id, &format!("View 页面结果无效：{error}"));
            return;
        }
        if status == "error" {
            self.set_chat_error(
                thread_id,
                summary
                    .filter(|text| !text.trim().is_empty())
                    .unwrap_or("View 未能完成，请补充信息后重试。"),
            );
        }
        let _ = self.persist();
        self.refresh_main_orchestration(cwd);
        self.try_integrate(cwd);
    }

    fn apply_main_plan(
        self: &Arc<Self>,
        cwd: &str,
        main_thread_id: &str,
        plan: &SpacePlan,
        settings: TurnSettings,
    ) {
        let integrating = self.is_integrating(cwd);
        if integrating {
            if let Err(error) = super::protocol::validate_integration_plan(plan) {
                self.finish_integration(cwd, "failed");
                self.set_chat_error(main_thread_id, error);
                return;
            }
        }
        let removed_threads = self
            .spaces
            .lock()
            .ok()
            .and_then(|spaces| spaces.get(cwd).cloned())
            .map(|space| {
                let removals: HashSet<&str> =
                    plan.remove_views.iter().map(String::as_str).collect();
                space
                    .views
                    .iter()
                    .filter_map(|view| {
                        removals.contains(view.id.as_str()).then(|| {
                            view.thread_id
                                .clone()
                                .map(|thread_id| (thread_id, view.id.clone()))
                        })?
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let planned = {
            let mut spaces = match self.spaces.lock() {
                Ok(spaces) => spaces,
                Err(_) => return,
            };
            let space = spaces
                .entry(cwd.to_string())
                .or_insert_with(|| crate::space::Space::load(&space_path(cwd), cwd));
            if space.main_thread_id.is_none() {
                space.main_thread_id = Some(main_thread_id.into());
            }
            match space.apply_plan(plan) {
                Ok(planned) => {
                    if integrating {
                        if let Some(run) = &mut space.integration {
                            run.status = "complete".into();
                        }
                    } else {
                        space.integration = (!planned.is_empty()).then(|| CompositionState {
                            status: "waiting".into(),
                            view_ids: planned.iter().map(|view| view.id.clone()).collect(),
                        });
                    }
                    planned
                }
                Err(error) => {
                    drop(spaces);
                    self.set_chat_error(main_thread_id, &error);
                    return;
                }
            }
        };

        for (thread_id, view_id) in removed_threads {
            let active_turn = self
                .store
                .lock()
                .ok()
                .and_then(|store| {
                    store
                        .chats
                        .iter()
                        .find(|chat| chat.id == thread_id)
                        .cloned()
                })
                .and_then(|chat| chat.active_turn);
            if let Some(turn_id) = active_turn {
                let _ = self.rpc(
                    "turn/interrupt",
                    json!({"threadId":thread_id,"turnId":turn_id}),
                );
            }
            if let Ok(mut store) = self.store.lock() {
                if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                    chat.active_turn = None;
                    chat.activity = None;
                    chat.error = Some(format!("View {view_id} 已从 Space 移除。"));
                }
            }
        }

        self.set_chat_activity(main_thread_id, Some("正在组织 View"));
        let _ = self.persist();
        for view in planned {
            if let Some(thread_id) = view.thread_id {
                let active = self
                    .store
                    .lock()
                    .unwrap()
                    .chats
                    .iter()
                    .find(|chat| chat.id == thread_id)
                    .and_then(|chat| chat.active_turn.clone());
                if active.is_none() {
                    self.start_view_turn(ViewTask {
                        cwd: cwd.into(),
                        parent_thread_id: main_thread_id.into(),
                        thread_id: Some(thread_id),
                        view_id: view.id,
                        title: view.title,
                        brief: view.brief,
                        settings: settings.clone(),
                    });
                }
            } else {
                self.start_view_thread(ViewTask {
                    cwd: cwd.into(),
                    parent_thread_id: main_thread_id.into(),
                    thread_id: None,
                    view_id: view.id,
                    title: view.title,
                    brief: view.brief,
                    settings: settings.clone(),
                });
            }
        }
        self.refresh_main_orchestration(cwd);
        let _ = self.persist();
    }
}
