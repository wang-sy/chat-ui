use super::{display_path, Project, Reply, Runtime};
use crate::space::Space;
use serde_json::json;
use std::path::Path;

impl Runtime {
    pub fn add_project(&self, path: &str) -> Reply {
        let path = std::fs::canonicalize(path).map_err(|_| "目录不存在或无法访问".to_string())?;
        if !path.is_dir() {
            return Err("请选择文件夹".into());
        }
        let path = display_path(&path);
        let name = Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        {
            let mut store = self.store.lock().unwrap();
            if !store.projects.iter().any(|p| p.path == path) {
                store.projects.push(Project {
                    path: path.clone(),
                    name,
                });
            }
        }
        self.spaces
            .lock()
            .unwrap()
            .entry(path.clone())
            .or_insert_with(|| Space::starter(&path));
        self.persist()?;
        Ok(json!({"path":path}))
    }
}
