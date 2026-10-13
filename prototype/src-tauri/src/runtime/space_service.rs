use super::{Reply, Runtime};
use crate::space::{space_path, Space};
use serde_json::{json, Value};

impl Runtime {
    pub fn space(&self, cwd: &str) -> Reply {
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
        let mut spaces = self.spaces.lock().unwrap();
        let space = spaces
            .entry(cwd.to_string())
            .or_insert_with(|| Space::load(&space_path(cwd), cwd));
        serde_json::to_value(space.clone()).map_err(|e| e.to_string())
    }

    pub fn mutate_space(&self, body: &Value) -> Reply {
        let cwd = body["cwd"].as_str().ok_or("缺少项目目录")?;
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
        let mut spaces = self.spaces.lock().unwrap();
        let space = spaces
            .entry(cwd.to_string())
            .or_insert_with(|| Space::load(&space_path(cwd), cwd));
        space.apply(body)?;
        let snapshot = space.clone();
        drop(spaces);
        self.persist()?;
        self.publish_event(
            "chat-ui/space/changed",
            &json!({"cwd": cwd, "action": body["action"]}),
        );
        serde_json::to_value(snapshot).map_err(|e| e.to_string())
    }
}
