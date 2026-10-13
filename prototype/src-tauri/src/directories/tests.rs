use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/directory-tests")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> String {
        display_path(&self.0)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn creates_one_folder_and_returns_it_ready_for_selection() {
    let fixture = Fixture::new();
    let result = create(&fixture.path(), "我的 Space 01").unwrap();
    assert!(fixture.0.join("我的 Space 01").is_dir());
    assert_eq!(
        result["path"],
        display_path(&fixture.0.join("我的 Space 01"))
    );
    assert_eq!(result["parent"], fixture.path());
    assert_eq!(result["children"], json!([]));
}

#[test]
fn never_overwrites_or_silently_selects_an_existing_entry() {
    let fixture = Fixture::new();
    std::fs::write(fixture.0.join("existing.txt"), "keep this content").unwrap();
    for name in ["existing.txt", "existing folder"] {
        if name == "existing folder" {
            create(&fixture.path(), name).unwrap();
        }
        assert!(create(&fixture.path(), name).unwrap_err().contains("同名"));
    }
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("existing.txt")).unwrap(),
        "keep this content"
    );
}

#[test]
fn rejects_path_components_and_missing_parents_without_creating_anything() {
    let fixture = Fixture::new();
    for name in [
        "",
        "  ",
        ".",
        "..",
        "../escaped",
        "/absolute",
        "a/b",
        "a\\b",
        "a:b",
        "a\0b",
    ] {
        assert!(create(&fixture.path(), name).is_err(), "{name:?}");
    }
    assert!(create(&fixture.path(), &"x".repeat(256)).is_err());
    assert!(create("", "child").is_err());
    assert!(create("relative", "child").is_err());
    assert!(create(&display_path(&fixture.0.join("missing")), "child").is_err());
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn browsing_lists_visible_folders_and_rejects_files() {
    let fixture = Fixture::new();
    for name in ["zeta", "Alpha", ".hidden"] {
        std::fs::create_dir(fixture.0.join(name)).unwrap();
    }
    std::fs::write(fixture.0.join("file.txt"), "not a folder").unwrap();
    let result = list(Some(&fixture.path())).unwrap();
    let names: Vec<_> = result["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Alpha", "zeta"]);
    assert!(list(Some(&display_path(&fixture.0.join("file.txt")))).is_err());
    assert_eq!(
        resolve(Some("~")).unwrap(),
        std::fs::canonicalize(home_dir()).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn symbolic_parent_resolves_to_the_displayed_real_directory() {
    let fixture = Fixture::new();
    let parent = fixture.0.join("real");
    std::fs::create_dir(&parent).unwrap();
    std::os::unix::fs::symlink(&parent, fixture.0.join("alias")).unwrap();
    let result = create(&display_path(&fixture.0.join("alias")), "child").unwrap();
    assert_eq!(result["path"], display_path(&parent.join("child")));
    assert!(parent.join("child").is_dir());
}
