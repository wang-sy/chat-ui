use super::Space;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionState {
    pub status: String,
    pub view_ids: Vec<String>,
}

impl Space {
    pub(crate) fn integration_ready(&self) -> bool {
        self.integration.as_ref().is_some_and(|run| {
            run.status == "waiting"
                && !run.view_ids.is_empty()
                && run.view_ids.iter().all(|id| {
                    self.views
                        .iter()
                        .any(|view| view.id == *id && view.status == "ready")
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::{ConnectionPlan, SpacePlan, ViewPlan};

    #[test]
    fn semantic_embed_references_survive_rename_and_reload() {
        let mut space = Space::starter("/tmp/composition-keys");
        let mut plan = SpacePlan {
            views: vec![ViewPlan {
                id: Some("architecture".into()),
                title: "架构".into(),
                brief: "SVG".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let id = space.apply_plan(&plan).unwrap()[0].id.clone();
        assert_eq!(space.views[0].key, "architecture");
        assert!(space.views[0].html.is_empty());
        plan.views[0].title = "系统关系图".into();
        space.apply_plan(&plan).unwrap();
        assert_eq!(space.views.len(), 1);
        assert_eq!(space.views[0].id, id);
        let reloaded: Space =
            serde_json::from_str(&serde_json::to_string(&space).unwrap()).unwrap();
        assert_eq!(reloaded.views[0].key, "architecture");
        let revision = space.revision;
        plan.views.push(plan.views[0].clone());
        assert!(space.apply_plan(&plan).is_err());
        assert_eq!(space.revision, revision);
    }

    #[test]
    fn synthesis_resolves_existing_semantic_connections_without_reassigning_views() {
        let mut space = Space::starter("/tmp/composition-edges");
        let plan = SpacePlan {
            views: ["architecture", "metrics"]
                .into_iter()
                .map(|key| ViewPlan {
                    id: Some(key.into()),
                    title: key.into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        space.apply_plan(&plan).unwrap();
        let integration = SpacePlan {
            connections: vec![ConnectionPlan {
                from: "architecture".into(),
                to: "metrics".into(),
                label: Some("提供观测事件".into()),
            }],
            ..Default::default()
        };
        assert!(space.apply_plan(&integration).unwrap().is_empty());
        assert_eq!(space.connections.len(), 1);
        assert_eq!(space.connections[0].from, space.views[0].id);
        assert_eq!(space.connections[0].to, space.views[1].id);
    }

    #[test]
    fn integration_waits_for_real_results_and_only_runs_once() {
        let mut space = Space::fixture("/tmp/composition");
        space.integration = Some(CompositionState {
            status: "waiting".into(),
            view_ids: vec!["architecture".into(), "payment-flow".into()],
        });
        space.views[1].status = "working".into();
        assert!(!space.integration_ready());
        space.views[1].status = "error".into();
        assert!(!space.integration_ready());
        space.views[1].status = "ready".into();
        assert!(space.integration_ready());
        space.integration.as_mut().unwrap().status = "running".into();
        assert!(!space.integration_ready());
        space.integration.as_mut().unwrap().status = "complete".into();
        assert!(!space.integration_ready());
    }
}
