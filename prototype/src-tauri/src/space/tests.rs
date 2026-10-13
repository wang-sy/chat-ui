use super::*;
use serde_json::json;

#[test]
fn test_fixture_has_owned_view_surfaces() {
    let space = Space::fixture("/tmp/chat-ui");
    assert_eq!(space.views.len(), 3);
    assert!(space
        .views
        .iter()
        .all(|view| view.thread_id.is_none() && view.html.contains("<!doctype html>")));
    assert_eq!(space.connections.len(), 1);
}

#[test]
fn layout_changes_are_bounded() {
    let mut space = Space::fixture("/tmp/chat-ui");
    space
        .apply(&json!({"action":"layout","viewId":"architecture","x":99999.0,"width":20.0}))
        .unwrap();
    assert_eq!(space.views[0].x, 10000.0);
    assert_eq!(space.views[0].width, MIN_VIEW_WIDTH);
}

#[test]
fn view_creation_and_connection_validate_ownership() {
    let mut space = Space::fixture("/tmp/chat-ui");
    space
        .apply(&json!({"action":"createView","title":"新页面","kind":"document"}))
        .unwrap();
    let id = space.selected_view.clone().unwrap();
    assert!(space.views.iter().any(|view| view.id == id));
    assert!(space
        .apply(&json!({"action":"connect","from":id,"to":"missing"}))
        .is_err());
}

#[test]
fn case_c02_main_plan_creates_owned_views_and_maps_requested_edges() {
    let mut space = Space::fixture("/tmp/chat-ui");
    let plan: SpacePlan = serde_json::from_value(json!({
        "kind":"space_plan",
        "mainPage":{"title":"推荐工作台","summary":"总览","html":"<!doctype html><h1>总览</h1>"},
        "views":[{"id":"brief","title":"简报","kind":"document","brief":"展示结论"},{"id":"metrics","title":"指标","kind":"dashboard","brief":"展示指标"}],
        "connections":[{"from":"brief","to":"metrics","label":"依据"}],
        "removeViews":[]
    })).unwrap();
    let planned = space.apply_plan(&plan).unwrap();
    assert_eq!(planned.len(), 2);
    assert_eq!(space.main_page.title, "推荐工作台");
    assert_eq!(space.connections.len(), 1);
    assert_eq!(space.connections[0].from, planned[0].id);
    assert_eq!(space.connections[0].to, planned[1].id);
    assert!(space
        .views
        .iter()
        .any(|view| view.title == "简报" && view.status == "planning"));
    let retry = space.apply_plan(&plan).unwrap();
    assert_eq!(
        planned.iter().map(|view| &view.id).collect::<Vec<_>>(),
        retry.iter().map(|view| &view.id).collect::<Vec<_>>()
    );
    assert_eq!(
        space
            .views
            .iter()
            .filter(|view| view.title == "简报" || view.title == "指标")
            .count(),
        2
    );
}

#[test]
fn case_c03_targeted_plan_reuses_one_view_without_touching_siblings() {
    let mut space = Space::fixture("/tmp/chat-ui");
    space.views[0].thread_id = Some("view-thread-architecture".into());
    let sibling = space.views[1].clone();
    let target_version = space.views[0].version;
    space
        .apply_plan(
            &serde_json::from_value(json!({
                "kind":"space_plan",
                "mainPage":{"title":"","summary":"","html":""},
                "views":[{"id":"architecture","title":"系统架构","kind":"architecture","brief":"增加筛选和空状态","x":88.0,"y":64.0,"width":420.0,"height":260.0}],
                "connections":[],
                "removeViews":[]
            }))
            .unwrap(),
        )
        .unwrap();
    let target = &space.views[0];
    let untouched = &space.views[1];
    assert_eq!(target.id, "architecture");
    assert_eq!(
        target.thread_id.as_deref(),
        Some("view-thread-architecture")
    );
    assert!(target.version > target_version);
    assert_eq!(target.x, 88.0);
    assert_eq!(target.brief, "增加筛选和空状态");
    assert_eq!(untouched.id, sibling.id);
    assert_eq!(untouched.version, sibling.version);
    assert_eq!(untouched.html, sibling.html);
    assert_eq!((untouched.x, untouched.y), (sibling.x, sibling.y));
}

#[test]
fn case_c08_view_page_update_advances_version_and_preserves_html_on_error() {
    let mut space = Space::fixture("/tmp/chat-ui");
    let version = space.views[0].version;
    let original_html = space.views[0].html.clone();
    space
        .update_view_page(
            "architecture",
            "<!doctype html><h1>新页面</h1>",
            "ready",
            Some("已完成"),
        )
        .unwrap();
    assert_eq!(space.views[0].status, "ready");
    assert_eq!(space.views[0].version, version + 1);
    assert!(space.views[0].html.contains("新页面"));
    assert_eq!(space.views[0].summary, "已完成");
    assert_ne!(space.views[0].brief, "已完成");
    assert!(space
        .update_view_page("architecture", "", "ready", None)
        .is_err());
    space
        .update_view_page("architecture", "", "error", Some("View 超时"))
        .unwrap();
    assert_eq!(space.views[0].status, "error");
    assert_eq!(space.views[0].html, "<!doctype html><h1>新页面</h1>");
    assert_ne!(space.views[0].html, original_html);
    assert_eq!(space.views[0].summary, "View 超时");
}

#[test]
fn case_c10_restart_recovery_marks_inflight_views_as_errors() {
    let mut space = Space::fixture("/tmp/chat-ui");
    space.views[0].status = "working".into();
    let version = space.views[0].version;
    space.recover_after_restart();
    assert_eq!(space.views[0].status, "error");
    assert_eq!(space.views[0].version, version + 1);
}

#[test]
fn status_transitions_are_versioned_and_invalid_states_are_rejected() {
    let mut space = Space::fixture("/tmp/chat-ui");
    let version = space.views[0].version;
    assert!(space.set_view_status("architecture", "working").unwrap());
    assert_eq!(space.views[0].version, version + 1);
    assert!(!space.set_view_status("architecture", "working").unwrap());
    assert!(space.set_view_status("architecture", "unknown").is_err());
}

#[test]
fn case_c05_main_plan_removes_views_and_replaces_connections() {
    let mut space = Space::fixture("/tmp/chat-ui");
    space.selected_view = Some("payment-flow".into());
    let plan: SpacePlan = serde_json::from_value(json!({
        "kind":"space_plan",
        "mainPage":{"title":"主页面","summary":"","html":""},
        "views":[{"id":"architecture","title":"系统架构","kind":"document","brief":""}],
        "connections":[],
        "removeViews":["payment-flow","request-sequence"]
    }))
    .unwrap();
    space.apply_plan(&plan).unwrap();
    assert_eq!(space.views.len(), 1);
    assert_eq!(space.views[0].id, "architecture");
    assert!(space.connections.is_empty());
    assert_eq!(space.selected_view.as_deref(), Some("architecture"));
}

#[test]
fn case_c10_persistence_round_trip_keeps_view_identity_layout_and_version() {
    let root = std::env::temp_dir().join(format!(
        "chat-ui-case-c10-{}-{}",
        std::process::id(),
        stable_id("persistence")
    ));
    let cwd = root.join("project");
    std::fs::create_dir_all(&cwd).unwrap();
    let cwd_text = cwd.to_string_lossy().to_string();
    let mut space = Space::fixture(&cwd_text);
    space.main_thread_id = Some("main-thread".into());
    space.views[0].thread_id = Some("view-thread".into());
    space.views[0].status = "working".into();
    space.views[0].x = 812.0;
    space.views[0].version = 7;
    space.save(&cwd).unwrap();

    let mut restored = Space::load(&space_path(&cwd_text), &cwd_text);
    assert_eq!(restored.main_thread_id.as_deref(), Some("main-thread"));
    assert_eq!(restored.views[0].thread_id.as_deref(), Some("view-thread"));
    assert_eq!(restored.views[0].x, 812.0);
    assert_eq!(restored.views[0].version, 7);
    restored.recover_after_restart();
    assert_eq!(restored.views[0].status, "error");
    assert_eq!(restored.views[0].version, 8);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn idless_retry_reuses_view_by_title() {
    let mut space = Space::fixture("/tmp/chat-ui");
    let plan: SpacePlan = serde_json::from_value(json!({
        "kind":"space_plan",
        "mainPage":{"title":"","summary":"","html":""},
        "views":[{"title":"系统架构","kind":"document","brief":""}],
        "connections":[]
    }))
    .unwrap();
    let first = space.apply_plan(&plan).unwrap();
    let second = space.apply_plan(&plan).unwrap();
    assert_eq!(first[0].id, second[0].id);
    assert_eq!(
        space
            .views
            .iter()
            .filter(|view| view.title == "系统架构")
            .count(),
        1
    );
}

#[test]
fn new_space_starts_empty_without_fake_ready_views() {
    let space = Space::starter("/tmp/new-space");
    assert!(space.views.is_empty());
    assert!(space.connections.is_empty());
    assert!(space.main_page.html.is_empty());
    assert!(space.main_thread_id.is_none());
}

#[test]
fn layout_only_plan_does_not_rebuild_a_completed_view() {
    let mut space = Space::fixture("/tmp/layout-only");
    space.views[0].thread_id = Some("owned-thread".into());
    let original = space.views[0].clone();
    let plan: SpacePlan = serde_json::from_value(json!({
        "views":[{"id":original.id,"title":original.title,"kind":original.kind,
                  "brief":original.brief,"x":1040.0}],
        "connections":[],"removeViews":[]
    }))
    .unwrap();
    assert!(space.apply_plan(&plan).unwrap().is_empty());
    assert_eq!(space.views[0].version, original.version);
    assert_eq!(space.views[0].html, original.html);
    assert_eq!(space.views[0].thread_id, original.thread_id);
    assert_eq!(space.views[0].x, 1040.0);
}
