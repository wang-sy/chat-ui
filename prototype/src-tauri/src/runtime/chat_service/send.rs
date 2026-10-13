use super::protocol::*;
use crate::runtime::{
    normalized_permission, permission_settings, service_tier_for_speed, Chat, Message, Reply,
    Runtime,
};
use crate::space::{space_path, Space};
use serde_json::{json, Value};
use std::sync::Arc;

impl Runtime {
    pub fn send(self: &Arc<Self>, body: &Value) -> Reply {
        let text = body["text"].as_str().unwrap_or("").trim();
        if text.is_empty() || text.len() > 100_000 {
            return Err("请输入 1 至 100000 字节的消息".into());
        }
        self.connect()?;
        let _gate = self.mutation_gate.lock().unwrap();
        let cwd = body["cwd"].as_str().ok_or("请先选择项目目录")?;
        if !self
            .store
            .lock()
            .unwrap()
            .projects
            .iter()
            .any(|project| project.path == cwd)
        {
            return Err("请先选择项目目录".into());
        }
        let model = body["model"].as_str().ok_or("请选择模型")?;
        let service_tier = service_tier_for_speed(body["speed"].as_str());
        let permission = normalized_permission(body.get("permission"))?;
        if !self
            .models
            .lock()
            .unwrap()
            .iter()
            .any(|m| m["model"] == model)
        {
            return Err("所选模型不在 Codex 模型列表中".into());
        }
        let user_id = body["requestId"].as_str().ok_or("缺少消息 ID")?.to_string();
        let target_view_id = body["viewId"].as_str().map(str::to_owned);
        if let Some(existing) = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|chat| {
                chat.cwd == cwd
                    && chat.messages.iter().any(|message| message.id == user_id)
                    && match (
                        target_view_id.as_deref(),
                        chat_role(chat),
                        chat.view_id.as_deref(),
                    ) {
                        (Some(view_id), "view", Some(owned_view)) => view_id == owned_view,
                        (None, "main", _) => true,
                        _ => false,
                    }
            })
            .map(|chat| chat.id.clone())
        {
            return Ok(json!({"id":existing}));
        }
        let id = if let Some(view_id) = target_view_id.clone() {
            let (existing_thread, view_title, view_brief, parent_thread_id) = {
                let mut spaces = self.spaces.lock().unwrap();
                let space = spaces
                    .entry(cwd.to_string())
                    .or_insert_with(|| Space::load(&space_path(cwd), cwd));
                let view = space
                    .views
                    .iter()
                    .find(|view| view.id == view_id)
                    .ok_or("View 不存在")?;
                (
                    view.thread_id.clone(),
                    view.title.clone(),
                    view.brief.clone(),
                    space.main_thread_id.clone(),
                )
            };
            if let Some(thread_id) = existing_thread {
                self.resume(&thread_id)?;
                thread_id
            } else {
                let (approval_policy, approvals_reviewer, sandbox, _) =
                    permission_settings(permission, cwd);
                let result = self.rpc("thread/start", json!({"cwd":cwd,"model":model,"serviceTier":service_tier,"approvalPolicy":approval_policy,"approvalsReviewer":approvals_reviewer,"sandbox":sandbox,"developerInstructions":view_thread_instructions(&view_id, &view_title, &view_brief),"threadSource":"chat_ui_view"}))?;
                let id = result["thread"]["id"]
                    .as_str()
                    .ok_or("Codex 没有返回 View Thread ID")?
                    .to_string();
                self.store.lock().unwrap().chats.insert(
                    0,
                    Chat {
                        id: id.clone(),
                        cwd: cwd.to_string(),
                        title: view_title,
                        model: model.to_string(),
                        effort: body["effort"].as_str().unwrap_or("").into(),
                        speed: body["speed"].as_str().unwrap_or("standard").into(),
                        permission: permission.to_string(),
                        role: "view".into(),
                        view_id: Some(view_id.clone()),
                        parent_thread_id,
                        ..Chat::default()
                    },
                );
                if let Some(space) = self.spaces.lock().unwrap().get_mut(cwd) {
                    if let Some(view) = space.views.iter_mut().find(|view| view.id == view_id) {
                        view.thread_id = Some(id.clone());
                    }
                    let _ = space.set_view_status(&view_id, "working");
                }
                self.resumed.lock().unwrap().insert(id.clone());
                id
            }
        } else if let Some(id) = body["threadId"].as_str() {
            let chat = self
                .store
                .lock()
                .unwrap()
                .chats
                .iter()
                .find(|chat| chat.id == id)
                .cloned()
                .ok_or("找不到聊天")?;
            if chat.cwd != cwd {
                return Err("线程不属于当前 Space".into());
            }
            if chat_role(&chat) != "main" {
                return Err("View Thread 必须通过对应 View 发送消息".into());
            }
            self.resume(id)?;
            id.to_string()
        } else {
            let existing = self
                .spaces
                .lock()
                .unwrap()
                .get(cwd)
                .and_then(|space| space.main_thread_id.clone());
            let existing = existing.filter(|id| {
                self.store
                    .lock()
                    .unwrap()
                    .chats
                    .iter()
                    .any(|chat| chat.id == *id)
            });
            if let Some(id) = existing {
                self.resume(&id)?;
                id
            } else {
                let (approval_policy, approvals_reviewer, sandbox, _) =
                    permission_settings(permission, cwd);
                let result = self.rpc("thread/start", json!({"cwd":cwd,"model":model,"serviceTier":service_tier,"approvalPolicy":approval_policy,"approvalsReviewer":approvals_reviewer,"sandbox":sandbox,"developerInstructions":MAIN_THREAD_INSTRUCTIONS,"threadSource":"chat_ui_main"}))?;
                let id = result["thread"]["id"]
                    .as_str()
                    .ok_or("Codex 没有返回线程 ID")?
                    .to_string();
                self.store.lock().unwrap().chats.insert(
                    0,
                    Chat {
                        id: id.clone(),
                        cwd: cwd.to_string(),
                        title: text.chars().take(40).collect(),
                        model: model.to_string(),
                        effort: body["effort"].as_str().unwrap_or("").into(),
                        speed: body["speed"].as_str().unwrap_or("standard").into(),
                        permission: permission.to_string(),
                        role: "main".into(),
                        ..Chat::default()
                    },
                );
                if let Some(space) = self.spaces.lock().unwrap().get_mut(cwd) {
                    space.main_thread_id = Some(id.clone());
                }
                self.resumed.lock().unwrap().insert(id.clone());
                id
            }
        };
        {
            let store = self.store.lock().unwrap();
            let chat = store
                .chats
                .iter()
                .find(|chat| chat.id == id)
                .ok_or("找不到聊天")?;
            if chat.cwd != cwd {
                return Err("线程不属于当前 Space".into());
            }
            match (
                target_view_id.as_deref(),
                chat_role(chat),
                chat.view_id.as_deref(),
            ) {
                (Some(view_id), "view", Some(owned_view)) if view_id == owned_view => {}
                (Some(_), "view", _) => return Err("View Thread 与 View 不匹配".into()),
                (Some(_), _, _) => return Err("当前线程不是 View Thread".into()),
                (None, "main", _) => {}
                (None, _, _) => return Err("View Thread 必须通过对应 View 发送消息".into()),
            }
        }
        let is_main_thread = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|chat| chat.id == id)
            .is_some_and(|chat| chat_role(chat) == "main");
        if is_main_thread {
            let space_cwd = body["cwd"].as_str().unwrap_or("").to_string();
            if let Some(space) = self.spaces.lock().unwrap().get_mut(&space_cwd) {
                if space.main_thread_id.is_none() {
                    space.main_thread_id = Some(id.clone());
                }
            }
        }
        {
            let mut store = self.store.lock().unwrap();
            let chat = store
                .chats
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or("找不到聊天")?;
            if chat.messages.iter().any(|m| m.id == user_id) {
                return Ok(json!({"id":id}));
            }
            if chat.active_turn.is_some() {
                return Err("这个线程正在工作，请停止后再发送。".into());
            }
            chat.active_turn = Some("starting".into());
            chat.activity = Some("正在思考".into());
            chat.activity_detail = None;
            chat.error = None;
            chat.model = model.to_string();
            chat.effort = body["effort"].as_str().unwrap_or("").into();
            chat.speed = body["speed"].as_str().unwrap_or("standard").into();
            chat.permission = permission.to_string();
            chat.messages.push(Message {
                id: user_id.clone(),
                role: "user".into(),
                text: text.into(),
            });
            crate::runtime::transcript::record_user(chat, &user_id, text);
        }
        self.publish_event(
            "chat-ui/chat/messageAccepted",
            &json!({"threadId": id, "messageId": user_id}),
        );
        if let Some(view_id) = target_view_id.as_deref() {
            self.set_view_status(cwd, view_id, "working");
        }
        let thread_cwd = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|chat| chat.id == id)
            .map(|chat| chat.cwd.clone())
            .unwrap_or_default();
        let thread_role = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .find(|chat| chat.id == id)
            .map(chat_role)
            .unwrap_or("main")
            .to_string();
        let (approval_policy, approvals_reviewer, _, sandbox_policy) =
            permission_settings(permission, &thread_cwd);
        let input_text = if thread_role == "main" {
            let snapshot = self
                .spaces
                .lock()
                .ok()
                .and_then(|spaces| spaces.get(&thread_cwd).map(space_context));
            match snapshot {
                Some(snapshot) => format!(
                    "User request:\n{text}\n\nCurrent Space snapshot (metadata only):\n{snapshot}"
                ),
                None => text.to_owned(),
            }
        } else {
            text.to_owned()
        };
        let mut params = json!({
            "threadId":id,
            "model":model,
            "summary":"auto",
            "serviceTierForTurn":service_tier,
            "approvalPolicy":approval_policy,
            "approvalsReviewer":approvals_reviewer,
            "sandboxPolicy":sandbox_policy,
            "input":[{"type":"text","text":input_text}],
            "outputSchema": if thread_role == "view" { view_output_schema() } else { main_output_schema() }
        });
        if let Some(effort) = body["effort"].as_str().filter(|v| !v.is_empty()) {
            params["effort"] = json!(effort);
        }
        let result = self.rpc("turn/start", params);
        {
            let mut store = self.store.lock().unwrap();
            let chat = store.chats.iter_mut().find(|c| c.id == id).unwrap();
            match &result {
                Ok(response) => {
                    // A short turn may complete before the turn/start response arrives.
                    let turn_id = response["turn"]["id"].as_str().unwrap_or("");
                    if !turn_id.is_empty()
                        && !self.completed_turns.lock().unwrap().contains(turn_id)
                    {
                        chat.active_turn = Some(turn_id.into());
                    }
                }
                Err(error) => {
                    chat.active_turn = None;
                    chat.activity = None;
                    chat.error = Some(error.clone());
                    chat.messages.retain(|m| m.id != user_id);
                    chat.transcript.retain(|item| item.id != user_id);
                }
            }
        }
        self.persist()?;
        if let Err(error) = &result {
            if let Some(view_id) = target_view_id.as_deref() {
                self.set_view_status(cwd, view_id, "error");
            }
            self.publish_event(
                "chat-ui/chat/sendFailed",
                &json!({"threadId": id, "messageId": user_id, "error": error}),
            );
        }
        result.map(|_| json!({"id":id}))
    }
}
