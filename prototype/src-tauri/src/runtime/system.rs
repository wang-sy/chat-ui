use super::{Chat, Runtime};
use crate::space::Space;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn discover_in(path: &std::ffi::OsStr) -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["codex.exe", "codex.cmd", "codex.bat"]
    } else {
        &["codex"]
    };
    for directory in std::env::split_paths(path) {
        if directory.as_os_str().is_empty() || !directory.is_absolute() {
            continue;
        }
        for name in names {
            let candidate = directory.join(name);
            if !candidate.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = candidate.metadata() {
                    if metadata.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
            }
            return Some(candidate);
        }
    }
    None
}

pub fn discover() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("CODEX_EXECUTABLE") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let bundled = Path::new(
            "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
        );
        if bundled.is_file() {
            return Some(bundled.to_path_buf());
        }
    }
    discover_in(&std::env::var_os("PATH")?)
}

pub fn hidden_command(executable: &Path) -> std::process::Command {
    #[allow(unused_mut)]
    let mut command = std::process::Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

pub fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix("\\\\?\\").unwrap_or(&value).to_string()
}

pub fn atomic_json(path: &Path, value: &Value) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(&temporary, contents).map_err(|e| e.to_string())?;
    replace_file(&temporary, path)
}

fn replace_file(temporary: &Path, target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    if target.exists() {
        std::fs::remove_file(target).map_err(|e| e.to_string())?;
    }
    std::fs::rename(temporary, target).map_err(|e| e.to_string())
}

pub fn messages_from_turns(turns: &[Value]) -> Vec<super::Message> {
    let mut messages = vec![];
    for turn in turns {
        for item in turn["items"].as_array().into_iter().flatten() {
            let role = match item["type"].as_str() {
                Some("userMessage") => "user",
                Some("agentMessage") => "assistant",
                _ => continue,
            };
            let text = if role == "assistant" {
                display_assistant_text(item["text"].as_str().unwrap_or(""))
            } else {
                display_user_text(
                    &item["content"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|part| part["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            };
            if text.trim().is_empty() {
                continue;
            }
            messages.push(super::Message {
                id: item["id"].as_str().unwrap_or("").into(),
                role: role.into(),
                text,
            });
        }
    }
    messages
}

fn display_user_text(text: &str) -> String {
    super::transcript::user_text(text)
}

fn display_assistant_text(text: &str) -> String {
    super::transcript::assistant_text(text)
}

pub fn load_store(state_dir: &Path) -> (super::Store, std::collections::HashMap<String, Space>) {
    let mut store: super::Store = std::fs::read(state_dir.join("projects.json"))
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default();
    store.chats.clear();
    for project in &store.projects {
        let path = Path::new(&project.path).join(".space/chat-ui.json");
        let Ok(data) = std::fs::read(path) else {
            continue;
        };
        let Ok(chats) = serde_json::from_slice::<Vec<Chat>>(&data) else {
            continue;
        };
        store.chats.extend(
            chats
                .into_iter()
                .filter(|chat| chat.cwd == project.path)
                .map(|mut chat| {
                    if chat.active_turn.is_some() {
                        chat.error = Some("上次服务退出时线程仍在运行，请重新发送任务。".into());
                    }
                    chat.active_turn = None;
                    chat.activity = None;
                    chat.requests.clear();
                    super::transcript::seed(&mut chat);
                    for item in &mut chat.transcript {
                        if item.status == "running" {
                            item.status = "interrupted".into();
                        }
                    }
                    chat
                }),
        );
    }
    let spaces = store
        .projects
        .iter()
        .map(|project| {
            let mut space = Space::load(&crate::space::space_path(&project.path), &project.path);
            space.recover_after_restart();
            (project.path.clone(), space)
        })
        .collect();
    (store, spaces)
}

pub fn migrate_chat_ownership(
    store: &mut super::Store,
    spaces: &std::collections::HashMap<String, Space>,
) {
    for chat in &mut store.chats {
        if chat.role.is_empty() {
            chat.role = "main".into();
        }
        for space in spaces.values() {
            if space.main_thread_id.as_deref() == Some(chat.id.as_str()) {
                chat.role = "main".into();
            }
            if let Some(view) = space
                .views
                .iter()
                .find(|view| view.thread_id.as_deref() == Some(chat.id.as_str()))
            {
                chat.role = "view".into();
                chat.view_id = Some(view.id.clone());
                chat.parent_thread_id = space.main_thread_id.clone();
            }
        }
    }
}

pub fn persist_all(runtime: &Runtime) -> Result<(), String> {
    let _gate = runtime.persist_gate.lock().map_err(|_| "持久化锁已损坏")?;
    let store = runtime.store.lock().unwrap();
    std::fs::create_dir_all(&runtime.state_dir).map_err(|e| e.to_string())?;
    atomic_json(
        &runtime.state_dir.join("projects.json"),
        &serde_json::json!({"projects": store.projects, "chats": []}),
    )?;
    for project in &store.projects {
        let chats: Vec<&Chat> = store
            .chats
            .iter()
            .filter(|chat| chat.cwd == project.path)
            .collect();
        let directory = Path::new(&project.path).join(".space");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        atomic_json(&directory.join("chat-ui.json"), &serde_json::json!(chats))?;
    }
    drop(store);
    for space in runtime.spaces.lock().unwrap().values() {
        space.save(Path::new(&space.cwd))?;
    }
    Ok(())
}
