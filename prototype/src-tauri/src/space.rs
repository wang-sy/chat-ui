use serde::{Deserialize, Serialize};

mod composition;
mod layout;
mod persistence;
mod plans;
pub(crate) use composition::CompositionState;
use std::path::{Path, PathBuf};

pub(crate) const MIN_VIEW_WIDTH: f64 = 280.0;
pub(crate) const MIN_VIEW_HEIGHT: f64 = 178.0;
pub(crate) const MAX_VIEW_SIZE: f64 = 1000.0;

/// The persisted, product-level workspace. Codex threads are referenced by id;
/// the Space owns composition and View state rather than the child DOM.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Space {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub revision: u64,
    pub main_thread_id: Option<String>,
    #[serde(default)]
    pub main_page: MainPageRecord,
    pub selected_view: Option<String>,
    pub views: Vec<ViewRecord>,
    pub connections: Vec<ViewConnection>,
    #[serde(default)]
    pub integration: Option<CompositionState>,
}

/// The Main Thread owns this page-level manifest. The page is the primary
/// visible Space surface; View records remain separately addressable pages
/// for child Threads and are not rendered as a competing graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainPageRecord {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewRecord {
    pub id: String,
    /// Stable semantic reference used by Main Page embed slots.
    #[serde(default)]
    pub key: String,
    pub title: String,
    pub kind: String,
    pub thread_id: Option<String>,
    pub status: String,
    #[serde(default)]
    pub brief: String,
    #[serde(default)]
    pub summary: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub collapsed: bool,
    pub version: u64,
    pub html: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpacePlan {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub main_page: MainPagePlan,
    #[serde(default)]
    pub views: Vec<ViewPlan>,
    #[serde(default)]
    pub connections: Vec<ConnectionPlan>,
    /// Explicit removals keep deletion intentional. A missing View in a plan
    /// is retained so a partial or interrupted turn cannot erase work.
    #[serde(default)]
    pub remove_views: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainPagePlan {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub html: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewPlan {
    pub id: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_view_kind")]
    pub kind: String,
    #[serde(default)]
    pub brief: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPlan {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PlannedView {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub brief: String,
    pub thread_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewConnection {
    pub id: String,
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

impl Space {
    pub fn starter(cwd: &str) -> Self {
        Self {
            id: format!("space-{}", stable_id(cwd)),
            title: Path::new(cwd)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| "工作空间".into()),
            cwd: cwd.into(),
            revision: 1,
            main_thread_id: None,
            main_page: MainPageRecord {
                title: "Space 页面".into(),
                summary: "Main Thread 负责生成主页面，View Thread 负责交付局部页面。".into(),
                html: String::new(),
                version: 1,
                updated_at: now_string(),
            },
            selected_view: None,
            views: vec![],
            connections: vec![],
            integration: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn fixture(cwd: &str) -> Self {
        let mut space = Self::starter(cwd);
        space.views = vec![
            starter_view(StarterViewSpec {
                id: "architecture",
                title: "系统架构",
                kind: "architecture",
                x: 52.0,
                y: 48.0,
                width: 392.0,
                height: 246.0,
                heading: "系统架构",
                subtitle: "Main Thread → View Threads → runnable pages",
            }),
            starter_view(StarterViewSpec {
                id: "payment-flow",
                title: "支付流程",
                kind: "flow",
                x: 474.0,
                y: 72.0,
                width: 354.0,
                height: 246.0,
                heading: "支付流程",
                subtitle: "用户 → 创建订单 → 风控 → 完成",
            }),
            starter_view(StarterViewSpec {
                id: "request-sequence",
                title: "调用时序",
                kind: "sequence",
                x: 174.0,
                y: 338.0,
                width: 438.0,
                height: 236.0,
                heading: "调用时序",
                subtitle: "Main Thread / View Thread / Runtime",
            }),
        ];
        space.connections = vec![ViewConnection {
            id: "architecture-payment".into(),
            from: "architecture".into(),
            to: "payment-flow".into(),
            label: Some("调度".into()),
        }];
        space
    }

    pub fn load(state_path: &Path, cwd: &str) -> Self {
        std::fs::read(state_path)
            .ok()
            .and_then(|data| serde_json::from_slice::<Self>(&data).ok())
            .filter(|space| space.cwd == cwd)
            .unwrap_or_else(|| Self::starter(cwd))
    }

    fn has_view(&self, id: &str) -> bool {
        self.views.iter().any(|view| view.id == id)
    }

    /// Change a View's lifecycle state through one bounded path so every
    /// visible transition is versioned and persisted consistently.
    pub fn set_view_status(&mut self, view_id: &str, status: &str) -> Result<bool, String> {
        if !matches!(status, "draft" | "planning" | "working" | "ready" | "error") {
            return Err("无效的 View 状态".into());
        }
        let view = self
            .views
            .iter_mut()
            .find(|view| view.id == view_id)
            .ok_or("View 不存在")?;
        if view.status == status {
            return Ok(false);
        }
        view.status = status.into();
        view.version = view.version.saturating_add(1);
        view.updated_at = now_string();
        self.revision = self.revision.saturating_add(1);
        Ok(true)
    }
}

#[cfg(test)]
struct StarterViewSpec<'a> {
    id: &'a str,
    title: &'a str,
    kind: &'a str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    heading: &'a str,
    subtitle: &'a str,
}

#[cfg(test)]
fn starter_view(spec: StarterViewSpec<'_>) -> ViewRecord {
    ViewRecord {
        id: spec.id.into(),
        key: spec.id.into(),
        title: spec.title.into(),
        kind: spec.kind.into(),
        thread_id: None,
        status: "ready".into(),
        brief: spec.subtitle.into(),
        summary: String::new(),
        x: spec.x,
        y: spec.y,
        width: spec.width,
        height: spec.height,
        collapsed: false,
        version: 1,
        html: starter_html(spec.heading, spec.subtitle),
        updated_at: now_string(),
    }
}

fn starter_html(heading: &str, subtitle: &str) -> String {
    // srcDoc is deliberately a complete, isolated document; the runtime never
    // grants it a service token or direct access to another View.
    format!(
        r#"<!doctype html><meta charset="utf-8"><style>body{{margin:0;padding:22px;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#202124;background:#fff}}h1{{font-size:22px;margin:0 0 8px}}p{{color:#70757c;margin:0 0 24px}}.line{{height:8px;border-radius:8px;background:#e7e9ec;margin:12px 0}}.line:nth-child(2){{width:78%;background:#b9c9ee}}.line:nth-child(3){{width:58%;background:#d5def2}}.pill{{display:inline-block;padding:7px 11px;border-radius:9px;background:#f1f2f4;font-size:12px;color:#5f6368}}</style><h1>{heading}</h1><p>{subtitle}</p><div class="line"></div><div class="line"></div><div class="line"></div><span class="pill">等待页面生成</span>"#
    )
}

fn stable_id(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn default_view_kind() -> String {
    "document".into()
}

fn limit_text(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn safe_segment(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        value.into()
    } else {
        stable_id(value)
    }
}

fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

pub fn space_path(cwd: &str) -> PathBuf {
    Path::new(cwd).join(".space/space.json")
}

#[cfg(test)]
mod tests;
