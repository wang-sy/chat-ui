fn main() {
    tauri_build::build();
    let root = std::path::Path::new("../dist/client");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut assets = Vec::new();
    collect_assets(root, root, &mut assets);
    assets.sort();
    let mut source = String::from("pub const WEB_ASSETS: &[(&str, &[u8])] = &[\n");
    for (url, path) in assets {
        source.push_str(&format!("({url:?}, include_bytes!({path:?})),\n"));
    }
    source.push_str("];\n");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("web_assets.rs"), source).unwrap();
}

fn collect_assets(
    root: &std::path::Path,
    directory: &std::path::Path,
    assets: &mut Vec<(String, String)>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_assets(root, &path, assets);
        } else if path.is_file() {
            let url = format!(
                "/{}",
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            assets.push((
                url,
                path.canonicalize().unwrap().to_string_lossy().into_owned(),
            ));
        }
    }
}
