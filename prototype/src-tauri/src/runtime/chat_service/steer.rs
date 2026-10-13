use crate::runtime::{Message, Reply, Runtime};
use serde_json::{json, Value};
use std::sync::Arc;

impl Runtime {
    pub fn steer(self: &Arc<Self>, body: &Value) -> Reply {
        let text = body["text"].as_str().unwrap_or("").trim();
        if text.is_empty() || text.len() > 100_000 {
            return Err("请输入 1 至 100000 字节的消息".into());
        }
        self.connect()?;
        let _gate = self.mutation_gate.lock().unwrap();
        let thread_id = body["threadId"].as_str().ok_or("缺少线程 ID")?.to_string();
        let cwd = body["cwd"].as_str().ok_or("缺少项目目录")?;
        let request_id = body["requestId"].as_str().ok_or("缺少消息 ID")?.to_string();
        let turn_id = {
            let store = self.store.lock().unwrap();
            let chat = store
                .chats
                .iter()
                .find(|chat| chat.id == thread_id)
                .ok_or("找不到聊天")?;
            if chat.cwd != cwd {
                return Err("线程不属于当前 Space".into());
            }
            chat.active_turn
                .clone()
                .filter(|id| id != "starting")
                .ok_or("线程没有可接收消息的活动回合")?
        };
        if self
            .store
            .lock()
            .unwrap()
            .chats
            .iter()
            .any(|chat| chat.messages.iter().any(|message| message.id == request_id))
        {
            return Ok(json!({"id":thread_id,"steered":true}));
        }
        if let Some(chat) = self
            .store
            .lock()
            .unwrap()
            .chats
            .iter_mut()
            .find(|chat| chat.id == thread_id)
        {
            chat.messages.push(Message {
                id: request_id.clone(),
                role: "user".into(),
                text: text.into(),
            });
            crate::runtime::transcript::record_user(chat, &request_id, text);
            chat.activity = Some("正在处理追加消息".into());
            chat.error = None;
        }
        self.publish_event(
            "chat-ui/turn/steerRequested",
            &json!({"threadId": thread_id, "requestId": request_id}),
        );
        let result = self.rpc(
            "turn/steer",
            json!({
                "threadId":thread_id,
                "expectedTurnId":turn_id,
                "clientUserMessageId":request_id,
                "input":[{"type":"text","text":text}]
            }),
        );
        if let Err(error) = &result {
            if let Ok(mut store) = self.store.lock() {
                if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                    chat.messages.retain(|message| message.id != request_id);
                    chat.transcript.retain(|item| item.id != request_id);
                    chat.error = Some(error.clone());
                }
            }
            self.publish_event(
                "chat-ui/turn/steerFailed",
                &json!({"threadId": thread_id, "requestId": request_id, "error": error}),
            );
            return Err(error.clone());
        }
        if let Some(next_turn) = result
            .as_ref()
            .ok()
            .and_then(|value| value["turnId"].as_str())
            .filter(|value| !value.is_empty())
        {
            if let Ok(mut store) = self.store.lock() {
                if let Some(chat) = store.chats.iter_mut().find(|chat| chat.id == thread_id) {
                    chat.active_turn = Some(next_turn.into());
                }
            }
        }
        self.persist()?;
        Ok(json!({"id":thread_id,"steered":true}))
    }
}
