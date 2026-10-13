use crate::runtime::{display_path, home_dir};
use serde_json::{json, Value};
use std::io::ErrorKind;
use std::path::PathBuf;

fn resolve(path: Option<&str>) -> Result<PathBuf, String> {
    let path = match path.filter(|path| !path.is_empty()) {
        None | Some("~") => home_dir(),
        Some(path) if path.starts_with("~/") => home_dir().join(&path[2..]),
        Some(path) => PathBuf::from(path),
    };
    if !path.is_absolute() {
        return Err("请输入完整路径，或以 ~/ 开头的路径".into());
    }
    let path = std::fs::canonicalize(path).map_err(|_| "文件夹不存在或无法访问")?;
    if !path.is_dir() {
        return Err("请选择文件夹".into());
    }
    Ok(path)
}

pub fn list(path: Option<&str>) -> Result<Value, String> {
    let path = resolve(path)?;
    let mut children = vec![];
    for entry in std::fs::read_dir(&path)
        .map_err(|_| "无法读取此文件夹，请检查访问权限")?
        .flatten()
    {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') {
                children.push(json!({"name":name,"path":display_path(&entry.path())}));
            }
        }
    }
    children.sort_by_key(|value| value["name"].as_str().unwrap_or("").to_lowercase());
    let home = home_dir();
    let mut roots = vec![json!({"name":"个人目录","path":display_path(&home)})];
    for (folder, name) in [
        ("Desktop", "桌面"),
        ("Documents", "文稿"),
        ("Downloads", "下载"),
    ] {
        let folder = home.join(folder);
        if folder.is_dir() {
            roots.push(json!({"name":name,"path":display_path(&folder)}));
        }
    }
    #[cfg(windows)]
    for drive in 'A'..='Z' {
        let root = format!("{drive}:\\");
        if std::path::Path::new(&root).is_dir() {
            roots.push(json!({"name":root,"path":root}));
        }
    }
    #[cfg(not(windows))]
    roots.push(json!({"name":"根目录","path":"/"}));
    Ok(
        json!({"path":display_path(&path),"parent":path.parent().map(display_path),
        "children":children,"roots":roots}),
    )
}

pub fn create(parent: &str, name: &str) -> Result<Value, String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err("请输入文件夹名称".into());
    }
    if name.contains(['/', '\\', ':']) || name.chars().any(char::is_control) {
        return Err("名称不能包含 /、\\、: 或控制字符".into());
    }
    if name.len() > 255 {
        return Err("文件夹名称过长，请缩短后重试".into());
    }
    if parent.is_empty() {
        return Err("请先选择新文件夹的位置".into());
    }
    let parent = resolve(Some(parent))?;
    let path = parent.join(name);
    // Create one child only. Never merge with an existing directory or overwrite a file.
    std::fs::create_dir(&path).map_err(|error| match error.kind() {
        ErrorKind::AlreadyExists => "此位置已有同名文件或文件夹，请换一个名称".to_string(),
        ErrorKind::PermissionDenied => "没有权限在此位置创建文件夹，请选择其他位置".to_string(),
        _ => format!("无法创建文件夹：{error}"),
    })?;
    list(Some(&display_path(&path)))
}

#[cfg(test)]
mod tests;
