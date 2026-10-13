use super::{
    now_string, stable_id, starter_html, Space, ViewConnection, ViewRecord, MAX_VIEW_SIZE,
    MIN_VIEW_HEIGHT, MIN_VIEW_WIDTH,
};
use serde_json::Value;

impl Space {
    pub fn apply(&mut self, body: &Value) -> Result<(), String> {
        let action = body["action"].as_str().unwrap_or("");
        match action {
            "select" => {
                let id = body["viewId"].as_str().filter(|id| self.has_view(id));
                if self.selected_view.as_deref() == id {
                    return Ok(());
                }
                self.selected_view = id.map(str::to_owned);
            }
            "createView" => {
                let title = body["title"].as_str().unwrap_or("新 View").trim();
                if title.is_empty() || title.len() > 120 {
                    return Err("View 名称长度必须在 1 到 120 个字符之间".into());
                }
                let id = format!(
                    "view-{}",
                    stable_id(&format!("{}:{}", self.revision, title))
                );
                self.views.push(ViewRecord {
                    key: String::new(),
                    id: id.clone(),
                    title: title.into(),
                    kind: body["kind"].as_str().unwrap_or("document").into(),
                    thread_id: None,
                    status: "draft".into(),
                    brief: String::new(),
                    summary: String::new(),
                    x: 120.0 + (self.views.len() as f64 * 28.0) % 360.0,
                    y: 90.0 + (self.views.len() as f64 * 32.0) % 260.0,
                    width: 380.0,
                    height: 244.0,
                    collapsed: false,
                    version: 1,
                    html: starter_html(title, "等待 Main Thread 分配任务"),
                    updated_at: now_string(),
                });
                self.selected_view = Some(id);
            }
            "deleteView" => {
                let id = body["viewId"].as_str().ok_or("缺少 View ID")?;
                if !self.has_view(id) {
                    return Err("View 不存在".into());
                }
                self.views.retain(|view| view.id != id);
                self.connections
                    .retain(|edge| edge.from != id && edge.to != id);
                if self.selected_view.as_deref() == Some(id) {
                    self.selected_view = None;
                }
            }
            "layout" => {
                let id = body["viewId"].as_str().ok_or("缺少 View ID")?;
                let view = self
                    .views
                    .iter_mut()
                    .find(|view| view.id == id)
                    .ok_or("View 不存在")?;
                for (key, target) in [
                    ("x", &mut view.x),
                    ("y", &mut view.y),
                    ("width", &mut view.width),
                    ("height", &mut view.height),
                ] {
                    if let Some(value) = body[key].as_f64() {
                        if key == "width" {
                            *target = value.clamp(MIN_VIEW_WIDTH, MAX_VIEW_SIZE);
                        } else if key == "height" {
                            *target = value.clamp(MIN_VIEW_HEIGHT, MAX_VIEW_SIZE);
                        } else {
                            *target = value.clamp(-2000.0, 10000.0);
                        }
                    }
                }
                if let Some(collapsed) = body["collapsed"].as_bool() {
                    view.collapsed = collapsed;
                }
            }
            "connect" => {
                let from = body["from"].as_str().ok_or("缺少起点 View")?;
                let to = body["to"].as_str().ok_or("缺少终点 View")?;
                if from == to || !self.has_view(from) || !self.has_view(to) {
                    return Err("连接必须来自两个不同的 View".into());
                }
                if !self
                    .connections
                    .iter()
                    .any(|edge| edge.from == from && edge.to == to)
                {
                    self.connections.push(ViewConnection {
                        id: format!("{}-{}", from, to),
                        from: from.into(),
                        to: to.into(),
                        label: body["label"].as_str().map(str::to_owned),
                    });
                }
            }
            _ => return Err("未知的 Space 操作".into()),
        }
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }
}
