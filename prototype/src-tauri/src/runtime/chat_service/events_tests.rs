use super::*;
use crate::runtime::Chat;
use crate::space::Space;
use serde_json::json;
use std::sync::Arc;

fn runtime_with_main() -> Arc<Runtime> {
    let runtime = Runtime::new(
        std::env::temp_dir().join(format!("chat-ui-events-case-{}", std::process::id())),
    );
    runtime.store.lock().unwrap().chats.push(Chat {
        id: "main-thread".into(),
        cwd: "/tmp/chat-ui-events".into(),
        role: "main".into(),
        ..Chat::default()
    });
    runtime
}

#[test]
fn case_c06_streamed_events_keep_execution_status_and_latest_detail() {
    let runtime = runtime_with_main();
    runtime.store.lock().unwrap().chats[0].token_usage = Some(json!({
        "last": {"totalTokens": 999}
    }));
    runtime.receive(json!({
        "method":"turn/started",
        "params":{"threadId":"main-thread","turn":{"id":"turn-1"}}
    }));
    runtime.receive(json!({
        "method":"item/reasoning/summaryTextDelta",
        "params":{"threadId":"main-thread","delta":"正在拆分任务"}
    }));
    runtime.receive(json!({
        "method":"item/commandExecution/started",
        "params":{"threadId":"main-thread","item":{"type":"commandExecution","command":["pnpm","test"]}}
    }));
    let chat = runtime
        .store
        .lock()
        .unwrap()
        .chats
        .iter()
        .find(|chat| chat.id == "main-thread")
        .cloned()
        .unwrap();
    assert_eq!(chat.active_turn.as_deref(), Some("turn-1"));
    assert_eq!(chat.activity.as_deref(), Some("正在执行命令"));
    assert_eq!(chat.activity_detail.as_deref(), Some("pnpm test"));
    assert!(chat.token_usage.is_none());

    runtime.receive(json!({
        "method":"item/agentMessage/delta",
        "params":{"threadId":"main-thread","itemId":"assistant-1","delta":"结果正在生成"}
    }));
    let chat = runtime
        .store
        .lock()
        .unwrap()
        .chats
        .iter()
        .find(|chat| chat.id == "main-thread")
        .cloned()
        .unwrap();
    assert_eq!(chat.activity.as_deref(), Some("正在回复"));
    assert_eq!(chat.messages[0].text, "结果正在生成");
    assert!(runtime.current_event_sequence() >= 4);

    runtime.receive(json!({
        "method":"thread/status/changed",
        "params":{"threadId":"main-thread","status":{"type":"idle","activeFlags":[]}}
    }));
    let chat = runtime
        .store
        .lock()
        .unwrap()
        .chats
        .iter()
        .find(|chat| chat.id == "main-thread")
        .cloned()
        .unwrap();
    assert!(chat.active_turn.is_none());
    assert!(chat.activity.is_none());
    assert!(chat.activity_detail.is_none());
}

#[test]
fn case_c06_main_waits_for_planning_and_working_views() {
    let cwd = "/tmp/chat-ui-orchestration";
    let runtime = Runtime::new(
        std::env::temp_dir().join(format!("chat-ui-orchestration-case-{}", std::process::id())),
    );
    let mut space = Space::fixture(cwd);
    space.main_thread_id = Some("main-thread".into());
    space.views[0].status = "planning".into();
    space.views[1].status = "working".into();
    space.views[2].status = "ready".into();
    runtime.spaces.lock().unwrap().insert(cwd.into(), space);
    runtime.store.lock().unwrap().chats.push(Chat {
        id: "main-thread".into(),
        cwd: cwd.into(),
        role: "main".into(),
        ..Chat::default()
    });
    runtime.refresh_main_orchestration(cwd);
    assert_eq!(
        runtime.store.lock().unwrap().chats[0].activity.as_deref(),
        Some("正在等待 View · 2 个")
    );

    {
        let mut spaces = runtime.spaces.lock().unwrap();
        let space = spaces.get_mut(cwd).unwrap();
        space.views[0].status = "ready".into();
        space.views[1].status = "error".into();
    }
    runtime.refresh_main_orchestration(cwd);
    assert!(runtime.store.lock().unwrap().chats[0].activity.is_none());
}

#[test]
fn case_c09_status_events_surface_approval_and_input_waiting_states() {
    let runtime = runtime_with_main();
    for (flag, expected) in [
        ("waitingOnApproval", "等待你的确认"),
        ("waitingOnUserInput", "等待你的输入"),
    ] {
        runtime.receive(json!({
            "method":"thread/status/changed",
            "params":{"threadId":"main-thread","status":{"type":"active","activeFlags":[flag]}}
        }));
        assert_eq!(
            runtime.store.lock().unwrap().chats[0].activity.as_deref(),
            Some(expected)
        );
    }
}
