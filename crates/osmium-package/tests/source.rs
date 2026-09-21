use osmium_package::load_source;
use serde_json::{Value, json};
use std::{fs, path::Path};

fn source() -> tempfile::TempDir {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &to.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic"),
        dir.path(),
    );
    dir
}

fn edit(root: &Path, name: &str, change: impl FnOnce(&mut Value)) {
    let path = root.join(name);
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    change(&mut value);
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

fn fails(root: &Path, code: &str) {
    let errors = load_source(root).unwrap_err();
    assert!(errors.iter().any(|e| e.code == code), "{errors:?}");
}

#[test]
fn load_example_is_read_only_and_does_not_package_author_notes() {
    let dir = source();
    fs::write(dir.path().join("notes.txt"), "private author note").unwrap();
    let original = fs::read(dir.path().join("osmium.json")).unwrap();
    let loaded = load_source(dir.path()).unwrap();
    assert_eq!(loaded.files.len(), 7);
    assert_eq!(loaded.files["osmium.json"], original);
    assert_eq!(fs::read(dir.path().join("osmium.json")).unwrap(), original);
    assert_eq!(loaded.model.prerequisite_order(), ["addition"]);
    assert!(loaded.files["content/introduction.md"].starts_with("# 足し算".as_bytes()));
}

#[test]
fn yaml_manifest_loads_but_two_manifests_fail() {
    let dir = source();
    let manifest = fs::read(dir.path().join("osmium.json")).unwrap();
    // JSON is a YAML subset, and the separate test below exercises block YAML.
    fs::write(dir.path().join("osmium.yaml"), manifest).unwrap();
    fails(dir.path(), "OSM_MANIFEST");
    fs::remove_file(dir.path().join("osmium.json")).unwrap();
    let yaml = "schema_version: '0.1'\npackage_id: org.example/arithmetic\npackage_version: 0.1.0\ntitle: 足し算\nlanguage: ja-JP\ncapabilities:\n  required: []\n  optional: []\nentities:\n  concepts: entities/concepts.json\n  objectives: entities/objectives.json\n  curricula: entities/curricula.json\n  resources: entities/resources.json\n  assessments: entities/assessments.json\n";
    fs::write(dir.path().join("osmium.yaml"), yaml).unwrap();
    assert!(load_source(dir.path()).is_ok());
}

#[test]
fn missing_resource_and_parent_escape_fail_before_reading_outside() {
    let dir = source();
    edit(dir.path(), "entities/resources.json", |v| {
        v[0]["path"] = json!("missing.md")
    });
    fails(dir.path(), "OSM_MISSING_FILE");
    edit(dir.path(), "entities/resources.json", |v| {
        v[0]["path"] = json!("../outside.md")
    });
    fails(dir.path(), "OSM_PATH");
    edit(dir.path(), "osmium.json", |v| {
        v["entities"]["concepts"] = json!("../outside.json")
    });
    fails(dir.path(), "OSM_PATH");
}

#[test]
fn semantic_diagnostic_names_the_actual_entity_file() {
    let dir = source();
    edit(dir.path(), "entities/objectives.json", |v| {
        v[0]["concept"] = json!("missing")
    });
    let errors = load_source(dir.path()).unwrap_err();
    let error = errors.iter().find(|e| e.code == "OSM_REFERENCE").unwrap();
    assert_eq!(error.file.as_deref(), Some("entities/objectives.json"));
    assert_eq!(error.path, "/0/concept");
}

#[test]
fn duplicate_json_and_invalid_language_fail() {
    let dir = source();
    edit(dir.path(), "osmium.json", |v| {
        v["language"] = json!("ja_JP")
    });
    fails(dir.path(), "OSM_LANGUAGE");
    fs::write(
        dir.path().join("osmium.json"),
        b"{\"schema_version\":\"0.1\",\"schema_version\":\"2.0\"}",
    )
    .unwrap();
    fails(dir.path(), "OSM_JSON");
}

#[test]
fn incompatible_packages_fail_and_unknown_optional_data_survives_loading() {
    let dir = source();
    edit(dir.path(), "osmium.json", |v| {
        v["schema_version"] = json!("2.0")
    });
    fails(dir.path(), "OSM_SCHEMA_VERSION");
    edit(dir.path(), "osmium.json", |v| {
        v["schema_version"] = json!("0.1");
        v["capabilities"]["required"] = json!(["org.example.future.v1"]);
    });
    fails(dir.path(), "OSM_CAPABILITY");
    edit(dir.path(), "osmium.json", |v| {
        v["capabilities"]["required"] = json!([]);
        v["capabilities"]["optional"] = json!(["org.example.future.v1"]);
        v["extensions"]["org.example.future.v1"] = json!({"kept": [null, true, "保存"]});
    });
    let loaded = load_source(dir.path()).unwrap();
    assert_eq!(
        loaded.model.documents().manifest["extensions"]["org.example.future.v1"],
        json!({"kept": [null, true, "保存"]})
    );
}

#[test]
fn unicode_collisions_and_invalid_markdown_bytes_fail() {
    let dir = source();
    fs::write(dir.path().join("é.txt"), "a").unwrap();
    fs::write(dir.path().join("e\u{301}.txt"), "b").unwrap();
    fails(dir.path(), "OSM_PATH_COLLISION");
    let dir = source();
    fs::write(dir.path().join("content/introduction.md"), [0xff]).unwrap();
    fails(dir.path(), "OSM_UTF8");
}

#[test]
fn oversized_resource_and_tree_fail() {
    let dir = source();
    fs::File::create(dir.path().join("content/introduction.md"))
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    fails(dir.path(), "OSM_INPUT_LIMIT");
    let dir = source();
    fs::File::create(dir.path().join("unreferenced.bin"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    fails(dir.path(), "OSM_INPUT_LIMIT");
}

#[test]
fn too_many_entries_and_excess_directory_depth_fail() {
    let dir = source();
    for i in 0..4097 {
        fs::write(dir.path().join(format!("f{i}")), []).unwrap();
    }
    fails(dir.path(), "OSM_INPUT_LIMIT");
    let dir = source();
    let deep = dir
        .path()
        .join(std::iter::repeat_n("a", 34).collect::<Vec<_>>().join("/"));
    fs::create_dir_all(deep).unwrap();
    fails(dir.path(), "OSM_INPUT_LIMIT");
}

#[test]
fn directory_links_are_rejected() {
    let dir = source();
    let outside = tempfile::tempdir().unwrap();
    let link = dir.path().join("link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    #[cfg(windows)]
    {
        // Junctions do not require the symlink privilege on Windows.
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    fails(dir.path(), "OSM_LINK");
    // Remove only the link; never recursively remove an external target.
    #[cfg(windows)]
    fs::remove_dir(&link).unwrap();
    #[cfg(unix)]
    fs::remove_file(&link).unwrap();
}
