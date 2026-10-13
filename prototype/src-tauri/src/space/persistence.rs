use super::{now_string, safe_segment, Space};
use std::collections::HashSet;
use std::path::Path;

impl Space {
    pub(crate) fn recover_after_restart(&mut self) {
        let mut changed = false;
        if let Some(run) = &mut self.integration {
            if matches!(run.status.as_str(), "waiting" | "running") {
                run.status = "interrupted".into();
                changed = true;
            }
        }
        for view in &mut self.views {
            if matches!(view.status.as_str(), "planning" | "working") {
                view.status = "error".into();
                view.version = view.version.saturating_add(1);
                view.updated_at = now_string();
                changed = true;
            }
        }
        if changed {
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub fn save(&self, project_path: &Path) -> Result<(), String> {
        let directory = project_path.join(".space");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let target = directory.join("space.json");
        let temporary = directory.join("space.json.tmp");
        atomic_write(
            &temporary,
            &target,
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )?;
        let main_directory = directory.join("main");
        let main_file = main_directory.join("index.html");
        if !self.main_page.html.trim().is_empty() {
            std::fs::create_dir_all(&main_directory).map_err(|e| e.to_string())?;
            atomic_write(
                &main_file.with_extension("html.tmp"),
                &main_file,
                self.main_page.html.as_bytes(),
            )?;
        } else if main_file.exists() {
            std::fs::remove_file(main_file).map_err(|e| e.to_string())?;
        }
        let views_root = directory.join("views");
        let active_view_paths: HashSet<String> = self
            .views
            .iter()
            .map(|view| safe_segment(&view.id))
            .collect();
        for view in &self.views {
            if view.html.trim().is_empty() {
                continue;
            }
            let view_directory = views_root.join(safe_segment(&view.id));
            std::fs::create_dir_all(&view_directory).map_err(|e| e.to_string())?;
            let index = view_directory.join("index.html");
            atomic_write(
                &index.with_extension("html.tmp"),
                &index,
                view.html.as_bytes(),
            )?;
        }
        if views_root.is_dir() {
            for entry in std::fs::read_dir(&views_root).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().map_err(|e| e.to_string())?.is_dir()
                    && !active_view_paths
                        .contains(&entry.file_name().to_string_lossy().into_owned())
                {
                    std::fs::remove_dir_all(entry.path()).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
}

fn atomic_write(temporary: &Path, target: &Path, contents: &[u8]) -> Result<(), String> {
    std::fs::write(temporary, contents).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    if target.exists() {
        std::fs::remove_file(target).map_err(|e| e.to_string())?;
    }
    std::fs::rename(temporary, target).map_err(|e| e.to_string())
}
