use super::*;
use crate::runtime::{Message, Project, Runtime};

#[test]
fn structured_response_is_readable_at_every_stream_boundary() {
    let source = r#"{"response":"先检查\n再画图 \"节点\" \u4f60\u597d \ud83d\ude80","page":{"html":"<script>private()</script>"}}"#;
    let final_text = "先检查\n再画图 \"节点\" 你好 🚀";
    for (offset, _) in source.char_indices() {
        let visible = assistant_text(&source[..offset]);
        assert!(final_text.starts_with(&visible), "{offset}: {visible}");
        assert!(!visible.contains("private"));
    }
    assert_eq!(assistant_text(source), final_text);
    assert_eq!(assistant_text("正在检查依赖…"), "正在检查依赖…");
    assert_eq!(
        assistant_text(r#"{"plan":{"response":"hidden"},"response":"done"}"#),
        "done"
    );
}

#[test]
fn orchestration_prompts_are_readable_assignments_not_internal_context() {
    assert_eq!(user_text("Runtime integration pass: private"), "");
    assert_eq!(
        user_text("User request:\n解释流程\n\nCurrent Space snapshot (metadata only): {}"),
        "解释流程"
    );
    let prompt = "Create or refine the visual artifact X.\n\nAssigned contract:\n画出支付流程\n\nReturn the structured View result required by your instructions.";
    assert_eq!(user_text(prompt), "Main Thread 分派任务\n\n画出支付流程");
}

#[test]
fn canonical_user_ids_replace_pending_once_without_deduplicating_later_requests() {
    let mut chat = Chat::default();
    for (turn, id) in [("t1", "u1"), ("t2", "u2")] {
        record_user(&mut chat, &format!("local-{id}"), "继续");
        for method in ["item/started", "item/completed"] {
            record_event(
                &mut chat,
                method,
                &json!({"turnId":turn,"item":{"type":"userMessage","id":id,"content":[{"text":"继续"}]}}),
            );
        }
    }
    assert_eq!(chat.transcript.len(), 2);
    assert_eq!(chat.transcript[0].id, "u1");
    assert_eq!(chat.transcript[1].turn_id, "t2");
}

#[test]
fn command_and_public_summary_streams_accumulate_and_survive_completion() {
    let mut chat = Chat::default();
    record_event(
        &mut chat,
        "item/started",
        &json!({"turnId":"t","item":{"id":"cmd","type":"commandExecution","command":"check","status":"inProgress"}}),
    );
    for delta in ["step one\n", "step two\n"] {
        record_event(
            &mut chat,
            "item/commandExecution/outputDelta",
            &json!({"turnId":"t","itemId":"cmd","delta":delta}),
        );
    }
    record_event(
        &mut chat,
        "item/reasoning/summaryTextDelta",
        &json!({"turnId":"t","itemId":"r","delta":"检查"}),
    );
    record_event(
        &mut chat,
        "item/reasoning/summaryTextDelta",
        &json!({"turnId":"t","itemId":"r","delta":"结果"}),
    );
    record_event(
        &mut chat,
        "item/reasoning/textDelta",
        &json!({"turnId":"t","itemId":"r","delta":"private raw reasoning"}),
    );
    assert_eq!(chat.transcript[0].text, "step one\nstep two\n");
    assert_eq!(chat.transcript[1].text, "检查结果");
    record_event(
        &mut chat,
        "item/completed",
        &json!({"turnId":"t","item":{"id":"cmd","type":"commandExecution","command":"check","exitCode":1}}),
    );
    record_event(
        &mut chat,
        "turn/completed",
        &json!({"turn":{"id":"t","status":"interrupted"}}),
    );
    assert_eq!(chat.transcript[0].status, "failed");
    assert_eq!(chat.transcript[0].text, "step one\nstep two\n");
    assert_eq!(chat.transcript[1].status, "interrupted");
}

#[test]
fn restoration_preserves_order_and_keeps_the_owning_thread_isolated() {
    let mut main = Chat::default();
    let mut view = Chat::default();
    let turns = vec![json!({"id":"t","status":"completed","items":[
        {"id":"u","type":"userMessage","content":[{"text":"做图"}]},
        {"id":"c","type":"commandExecution","command":"check","aggregatedOutput":"ok","status":"completed"},
        {"id":"a","type":"agentMessage","text":"{\"response\":\"图已完成\",\"page\":{\"html\":\"private\"}}"}
    ]})];
    record_event(
        &mut view,
        "turn/plan/updated",
        &json!({"turnId":"t","plan":[{"step":"检查","status":"completed"}]}),
    );
    restore(&mut view, &turns);
    restore(&mut main, &[]);
    assert_eq!(view.transcript[0].kind, "user");
    assert_eq!(view.transcript[1].text, "ok");
    assert_eq!(view.transcript[2].text, "图已完成");
    assert!(view.transcript.iter().any(|item| item.kind == "plan"));
    assert!(main.transcript.is_empty());
    restore(&mut view, &turns);
    assert_eq!(view.transcript.len(), 4);
}

#[test]
fn persisted_history_survives_restart_and_legacy_chats_migrate() {
    let path = std::env::temp_dir().join(format!("chat-ui-transcript-{}", std::process::id()));
    let runtime = Runtime::new(path.join("state"));
    let mut chat = Chat {
        id: "persisted".into(),
        cwd: path.to_string_lossy().into(),
        messages: vec![Message {
            id: "old".into(),
            role: "assistant".into(),
            text: "旧回复".into(),
        }],
        ..Chat::default()
    };
    seed(&mut chat);
    record_event(
        &mut chat,
        "item/reasoning/summaryTextDelta",
        &json!({"turnId":"t","itemId":"r","delta":"正在检查"}),
    );
    runtime.store.lock().unwrap().projects.push(Project {
        path: chat.cwd.clone(),
        name: "history".into(),
    });
    runtime.store.lock().unwrap().chats.push(chat);
    runtime.persist().unwrap();
    let restarted = Runtime::new(path.join("state"));
    let store = restarted.store.lock().unwrap();
    assert_eq!(store.chats[0].transcript[0].text, "旧回复");
    assert_eq!(store.chats[0].transcript[1].text, "正在检查");
    assert_eq!(store.chats[0].transcript[1].status, "interrupted");
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn reconnects_are_retained_as_progress_and_settle_when_output_resumes() {
    let mut chat = Chat::default();
    record_event(
        &mut chat,
        "error",
        &json!({"turnId":"t","willRetry":true,"error":{"message":"Reconnecting... 2/5"}}),
    );
    record_event(
        &mut chat,
        "error",
        &json!({"turnId":"t","willRetry":true,"error":{"message":"Reconnecting... 3/5"}}),
    );
    assert_eq!(chat.transcript.len(), 1);
    assert_eq!(chat.transcript[0].status, "running");
    record_event(
        &mut chat,
        "item/started",
        &json!({"turnId":"t","item":{"type":"agentMessage","id":"a","text":"继续处理"}}),
    );
    assert_eq!(chat.transcript[0].status, "completed");
    assert_eq!(chat.transcript[0].title, "连接已恢复");
}
