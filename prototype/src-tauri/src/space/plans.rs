use super::{
    limit_text, now_string, stable_id, PlannedView, Space, SpacePlan, ViewConnection, ViewRecord,
    MAX_VIEW_SIZE, MIN_VIEW_HEIGHT, MIN_VIEW_WIDTH,
};
use std::collections::{HashMap, HashSet};

impl Space {
    /// Apply a Main Thread plan without allowing it to reach into a View's
    /// internal page.  The plan owns composition metadata; View Threads own
    /// the eventual HTML payload.
    pub fn apply_plan(&mut self, plan: &SpacePlan) -> Result<Vec<PlannedView>, String> {
        if plan.views.len() > 32 {
            return Err("一次最多创建 32 个 View".into());
        }
        if plan.remove_views.len() > 32 {
            return Err("一次最多移除 32 个 View".into());
        }
        let mut references = HashSet::new();
        for item in &plan.views {
            if let Some(id) = item.id.as_deref().filter(|id| !id.is_empty()) {
                if !references.insert(id) {
                    return Err("计划包含重复的 View 引用".into());
                }
            }
        }
        if !plan.remove_views.is_empty() {
            let removals: HashSet<&str> = plan.remove_views.iter().map(String::as_str).collect();
            self.views
                .retain(|view| !removals.contains(view.id.as_str()));
            self.connections.retain(|edge| {
                !removals.contains(edge.from.as_str()) && !removals.contains(edge.to.as_str())
            });
            if self
                .selected_view
                .as_deref()
                .is_some_and(|id| removals.contains(id))
            {
                self.selected_view = None;
            }
        }
        if !plan.main_page.title.trim().is_empty() {
            self.main_page.title = limit_text(&plan.main_page.title, 160);
        }
        if !plan.main_page.summary.trim().is_empty() {
            self.main_page.summary = limit_text(&plan.main_page.summary, 1000);
        }
        if !plan.main_page.html.trim().is_empty() {
            self.main_page.html = limit_text(&plan.main_page.html, 200_000);
        }
        self.main_page.version = self.main_page.version.saturating_add(1);
        self.main_page.updated_at = now_string();

        let mut planned = Vec::with_capacity(plan.views.len());
        let mut requested_ids = HashMap::new();
        for (index, item) in plan.views.iter().enumerate() {
            let title = if item.title.trim().is_empty() {
                format!("View {}", index + 1)
            } else {
                limit_text(&item.title, 120)
            };
            let existing_id = item
                .id
                .as_deref()
                .and_then(|id| {
                    self.views
                        .iter()
                        .find(|view| view.id == id || view.key == id)
                })
                .map(|view| view.id.clone())
                .or_else(|| {
                    // Stable ids are preferred, but matching an id-less plan
                    // by title prevents a retry from duplicating a View.
                    self.views
                        .iter()
                        .find(|view| view.title == title)
                        .map(|view| view.id.clone())
                });
            let id = existing_id.unwrap_or_else(|| {
                let requested = item.id.as_deref().unwrap_or("");
                format!(
                    "view-{}",
                    stable_id(&format!("{}:{}:{}", self.cwd, requested, title))
                )
            });
            if let Some(requested) = item.id.as_deref().filter(|requested| !requested.is_empty()) {
                requested_ids.insert(requested.to_string(), id.clone());
            }
            let needs_build = self
                .views
                .iter()
                .find(|view| view.id == id)
                .is_none_or(|view| {
                    view.thread_id.is_none()
                        || matches!(view.status.as_str(), "draft" | "error")
                        || view.title != title
                        || (!item.brief.trim().is_empty() && view.brief != item.brief)
                });
            if let Some(view) = self.views.iter_mut().find(|view| view.id == id) {
                if let Some(key) = item
                    .id
                    .as_deref()
                    .filter(|key| *key != id && !key.is_empty())
                {
                    view.key = limit_text(key, 120);
                }
                if view.title != title
                    || (!item.brief.trim().is_empty() && view.brief != item.brief)
                {
                    view.version = view.version.saturating_add(1);
                    view.updated_at = now_string();
                }
                view.title = title.clone();
                view.kind = if item.kind.trim().is_empty() {
                    "document".into()
                } else {
                    limit_text(&item.kind, 40)
                };
                if !item.brief.trim().is_empty() {
                    view.brief = limit_text(&item.brief, 2000);
                }
                view.x = item
                    .x
                    .map(|value| value.clamp(-2000.0, 10000.0))
                    .unwrap_or(view.x);
                view.y = item
                    .y
                    .map(|value| value.clamp(-2000.0, 10000.0))
                    .unwrap_or(view.y);
                view.width = item
                    .width
                    .map(|value| value.clamp(MIN_VIEW_WIDTH, MAX_VIEW_SIZE))
                    .unwrap_or(view.width);
                view.height = item
                    .height
                    .map(|value| value.clamp(MIN_VIEW_HEIGHT, MAX_VIEW_SIZE))
                    .unwrap_or(view.height);
                if view.status == "ready" && view.thread_id.is_none() {
                    view.status = "planning".into();
                    view.version = view.version.saturating_add(1);
                    view.updated_at = now_string();
                }
            } else {
                let brief = limit_text(&item.brief, 2000);
                self.views.push(ViewRecord {
                    id: id.clone(),
                    key: item
                        .id
                        .as_deref()
                        .map(|key| limit_text(key, 120))
                        .unwrap_or_default(),
                    title: title.clone(),
                    kind: if item.kind.trim().is_empty() {
                        "document".into()
                    } else {
                        limit_text(&item.kind, 40)
                    },
                    thread_id: None,
                    status: "planning".into(),
                    brief: brief.clone(),
                    summary: String::new(),
                    x: item
                        .x
                        .map(|value| value.clamp(-2000.0, 10000.0))
                        .unwrap_or(120.0 + (self.views.len() as f64 * 28.0) % 420.0),
                    y: item
                        .y
                        .map(|value| value.clamp(-2000.0, 10000.0))
                        .unwrap_or(90.0 + (self.views.len() as f64 * 32.0) % 300.0),
                    width: item
                        .width
                        .map(|value| value.clamp(MIN_VIEW_WIDTH, MAX_VIEW_SIZE))
                        .unwrap_or(380.0),
                    height: item
                        .height
                        .map(|value| value.clamp(MIN_VIEW_HEIGHT, MAX_VIEW_SIZE))
                        .unwrap_or(244.0),
                    collapsed: false,
                    version: 1,
                    html: String::new(),
                    updated_at: now_string(),
                });
            }
            let view = self
                .views
                .iter()
                .find(|view| view.id == id)
                .ok_or("无法保存 View")?;
            if needs_build {
                planned.push(PlannedView {
                    id,
                    title,
                    kind: view.kind.clone(),
                    brief: view.brief.clone(),
                    thread_id: view.thread_id.clone(),
                });
            }
        }

        // A plan is a complete composition snapshot. Replacing the edge set
        // makes a Main Thread update authoritative while retaining Views that
        // were not explicitly removed above.
        self.connections.clear();
        for edge in &plan.connections {
            let from = requested_ids
                .get(&edge.from)
                .map(String::as_str)
                .or_else(|| {
                    self.views
                        .iter()
                        .find(|view| view.key == edge.from)
                        .map(|view| view.id.as_str())
                })
                .unwrap_or(&edge.from);
            let to = requested_ids
                .get(&edge.to)
                .map(String::as_str)
                .or_else(|| {
                    self.views
                        .iter()
                        .find(|view| view.key == edge.to)
                        .map(|view| view.id.as_str())
                })
                .unwrap_or(&edge.to);
            if from == to || !self.has_view(from) || !self.has_view(to) {
                continue;
            }
            if !self
                .connections
                .iter()
                .any(|current| current.from == from && current.to == to)
            {
                self.connections.push(ViewConnection {
                    id: format!("{}-{}", from, to),
                    from: from.into(),
                    to: to.into(),
                    label: edge.label.as_deref().map(|value| limit_text(value, 80)),
                });
            }
        }
        if self.selected_view.is_none() {
            self.selected_view = planned.first().map(|view| view.id.clone());
        }
        self.revision = self.revision.saturating_add(1);
        Ok(planned)
    }

    pub fn update_view_page(
        &mut self,
        view_id: &str,
        html: &str,
        status: &str,
        summary: Option<&str>,
    ) -> Result<(), String> {
        if !matches!(status, "ready" | "error") {
            return Err("页面结果必须是 ready 或 error".into());
        }
        let view = self
            .views
            .iter_mut()
            .find(|view| view.id == view_id)
            .ok_or("View 不存在")?;
        let html = limit_text(html, 300_000);
        if status == "ready" && html.trim().is_empty() {
            return Err("View 标记为 ready 时必须返回可运行的 HTML".into());
        }
        if !html.trim().is_empty() {
            view.html = html;
        }
        if let Some(summary) = summary.filter(|value| !value.trim().is_empty()) {
            view.summary = limit_text(summary, 1000);
        }
        view.status = status.into();
        view.version = view.version.saturating_add(1);
        view.updated_at = now_string();
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }
}
