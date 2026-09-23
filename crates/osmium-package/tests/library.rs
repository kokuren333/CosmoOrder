use osmium_package::{distribution::build, library::Library};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic")
}

fn copied_source() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes) in osmium_package::load_source(source()).unwrap().files {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    dir
}

#[test]
fn zip_directory_reinstall_and_reopen_keep_identical_files() {
    let temp = tempfile::tempdir().unwrap();
    let zip = temp.path().join("course.osmium");
    let directory = temp.path().join("course");
    build(&source(), &zip).unwrap();
    build(&source(), &directory).unwrap();
    let home = temp.path().join("data");
    let mut library = Library::open(&home).unwrap();
    let first = library.install(&zip).unwrap();
    assert!(!first.already_installed);
    assert!(first.package.path.is_dir());
    assert_eq!(first.package.entity_counts["concepts"], 1);
    assert_eq!(first.package.entity_counts["objectives"], 1);
    assert_eq!(first.package.entity_counts["resources"], 1);
    assert_eq!(first.package.entity_counts["assessments"], 2);
    assert!(library.install(&directory).unwrap().already_installed);
    assert_eq!(library.packages().unwrap().len(), 1);
    assert_eq!(fs::read_dir(home.join("staging")).unwrap().count(), 0);
    drop(library);
    // Neither source nor transport is needed after installation.
    fs::remove_file(zip).unwrap();
    let reopened = Library::open(&home).unwrap();
    assert_eq!(
        reopened
            .read("org.example/arithmetic", None)
            .unwrap()
            .digest,
        first.package.digest
    );
}

#[test]
fn same_version_conflict_is_non_destructive_and_new_versions_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original.osmium");
    build(&source(), &original).unwrap();
    let mut library = Library::open(&temp.path().join("data")).unwrap();
    let installed = library.install(&original).unwrap();
    let changed = copied_source();
    let path = changed.path().join("osmium.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["title"] = json!("New title");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let conflict = temp.path().join("conflict.osmium");
    build(changed.path(), &conflict).unwrap();
    assert_eq!(
        library.install(&conflict).unwrap_err()[0].code,
        "OSM_VERSION_CONFLICT"
    );
    assert_eq!(
        library.read("org.example/arithmetic", None).unwrap().digest,
        installed.package.digest
    );
    manifest["package_version"] = json!("0.2.0");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let update = temp.path().join("update.osmium");
    build(changed.path(), &update).unwrap();
    library.install(&update).unwrap();
    assert_eq!(library.packages().unwrap().len(), 2);
    assert_eq!(
        library.read("org.example/arithmetic", None).unwrap_err()[0].code,
        "OSM_VERSION_REQUIRED"
    );
    assert_eq!(
        library
            .read("org.example/arithmetic", Some("0.1.0"))
            .unwrap()
            .digest,
        installed.package.digest
    );
}

#[test]
fn failed_install_preserves_existing_package_and_leaves_no_partial_output() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("course");
    build(&source(), &directory).unwrap();
    let home = temp.path().join("data");
    let mut library = Library::open(&home).unwrap();
    let installed = library.install(&directory).unwrap();
    fs::write(directory.join("content/introduction.md"), "tampered").unwrap();
    assert!(library.install(&directory).is_err());
    assert_eq!(library.packages().unwrap().len(), 1);
    assert_eq!(
        library.read("org.example/arithmetic", None).unwrap().digest,
        installed.package.digest
    );
    assert_eq!(fs::read_dir(home.join("staging")).unwrap().count(), 0);
    // Crash leftovers are outside the visible library and are never registered.
    fs::create_dir(home.join("staging/interrupted")).unwrap();
    fs::write(home.join("staging/interrupted/manifest.json"), "partial").unwrap();
    assert_eq!(library.packages().unwrap().len(), 1);
}

#[test]
fn library_lock_is_exclusive_and_released_on_drop() {
    let temp = tempfile::tempdir().unwrap();
    let first = Library::open(temp.path()).unwrap();
    assert!(Library::open(temp.path()).is_err());
    drop(first);
    assert!(Library::open(temp.path()).is_ok());
}

#[test]
fn staging_io_failure_never_publishes_a_partial_package() {
    let temp = tempfile::tempdir().unwrap();
    let zip = temp.path().join("course.osmium");
    build(&source(), &zip).unwrap();
    let home = temp.path().join("home");
    let mut library = Library::open(&home).unwrap();
    fs::remove_dir(home.join("staging")).unwrap();
    fs::write(home.join("staging"), "unavailable").unwrap();
    assert_eq!(library.install(&zip).unwrap_err()[0].code, "OSM_IO");
    assert!(library.packages().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(home.join("staging")).unwrap(),
        "unavailable"
    );
}

#[test]
fn tampered_installed_files_are_reported_instead_of_silently_replaced() {
    let temp = tempfile::tempdir().unwrap();
    let zip = temp.path().join("course.osmium");
    build(&source(), &zip).unwrap();
    let mut library = Library::open(&temp.path().join("data")).unwrap();
    let installed = library.install(&zip).unwrap();
    fs::write(
        installed.package.path.join("content/introduction.md"),
        "local damage",
    )
    .unwrap();
    assert!(library.packages().is_err());
    assert!(library.install(&zip).is_err());
    assert_eq!(
        fs::read_to_string(installed.package.path.join("content/introduction.md")).unwrap(),
        "local damage"
    );
}

#[cfg(windows)]
#[test]
fn library_home_rejects_windows_junctions() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let junction = temp.path().join("linked");
    fs::create_dir(&target).unwrap();
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:OSMIUM_TEST_LINK -Target $env:OSMIUM_TEST_TARGET | Out-Null"])
        .env("OSMIUM_TEST_LINK", &junction).env("OSMIUM_TEST_TARGET", &target).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Library::open(&junction).is_err());
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    fs::remove_dir(junction).unwrap();
}
