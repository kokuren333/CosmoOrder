use osmium_package::distribution::build;
use osmium_store::{AttemptRequest, runtime::Runtime};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PACKAGE: &str = "org.example/arithmetic";
fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic")
}
fn request(response: Value) -> AttemptRequest {
    AttemptRequest {
        assessment_id: "addition.01".into(),
        response,
        request_id: uuid::Uuid::new_v4().to_string(),
        duration_ms: Some(1200),
        hints_used: Some(0),
    }
}
fn setup() -> (tempfile::TempDir, Runtime) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("package.osmium");
    build(&source(), &path).unwrap();
    let mut runtime = Runtime::open(&temp.path().join("home")).unwrap();
    runtime.install(&path).unwrap();
    (temp, runtime)
}

#[test]
fn answer_reopen_and_idempotency_preserve_observations() {
    let (temp, mut runtime) = setup();
    let request = request(json!("b"));
    let first = runtime.answer(PACKAGE, None, &request).unwrap();
    assert_eq!(first.event["score"], 1);
    assert_eq!(first.event["event_type"], "assessment_attempt");
    assert_eq!(first.event["concept_ids"], json!(["addition"]));
    assert!(!first.replayed);
    drop(runtime);
    let mut runtime = Runtime::open(&temp.path().join("home")).unwrap();
    let replay = runtime.answer(PACKAGE, None, &request).unwrap();
    assert!(replay.replayed);
    assert_eq!(first.event, replay.event);
    let mut conflicting = request.clone();
    conflicting.response = json!("a");
    assert_eq!(
        runtime.answer(PACKAGE, None, &conflicting).unwrap_err()[0].code,
        "OSM_REQUEST_CONFLICT"
    );
    assert_eq!(runtime.history(PACKAGE, None, 16, 0).unwrap().len(), 1);
    let progress = runtime.progress(PACKAGE, None).unwrap();
    assert_eq!(progress[0].attempts, 1);
    assert_eq!(progress[0].accuracy, Some(1.0));
}

#[test]
fn reinstalling_the_same_digest_restores_its_existing_progress() {
    let (temp, mut runtime) = setup();
    runtime.answer(PACKAGE, None, &request(json!("b"))).unwrap();
    let before = runtime.progress(PACKAGE, None).unwrap();
    let distribution = temp.path().join("package.osmium");

    runtime.uninstall(PACKAGE, "0.1.0").unwrap();
    assert!(runtime.packages().unwrap().is_empty());
    runtime.install(&distribution).unwrap();

    assert_eq!(runtime.progress(PACKAGE, None).unwrap(), before);
}

#[test]
fn invalid_answers_and_transaction_failure_leave_no_half_event() {
    let (_temp, mut runtime) = setup();
    assert!(
        runtime
            .answer(PACKAGE, None, &request(json!(true)))
            .is_err()
    );
    let connection = Connection::open(runtime.database_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_projection BEFORE INSERT ON progress BEGIN SELECT RAISE(ABORT, 'injected projection failure'); END;").unwrap();
    let attempt = request(json!("b"));
    assert!(runtime.answer(PACKAGE, None, &attempt).is_err());
    assert!(runtime.history(PACKAGE, None, 16, 0).unwrap().is_empty());
    assert_eq!(runtime.progress(PACKAGE, None).unwrap()[0].attempts, 0);
    connection
        .execute_batch("DROP TRIGGER fail_projection")
        .unwrap();
    assert!(!runtime.answer(PACKAGE, None, &attempt).unwrap().replayed);
}

#[test]
fn append_only_history_rebuild_export_and_backup_are_consistent() {
    let (temp, mut runtime) = setup();
    runtime.answer(PACKAGE, None, &request(json!("b"))).unwrap();
    runtime.answer(PACKAGE, None, &request(json!("a"))).unwrap();
    let expected = runtime.progress(PACKAGE, None).unwrap();
    assert_eq!(expected[0].accuracy, Some(0.5));
    let connection = Connection::open(runtime.database_path()).unwrap();
    assert!(connection.execute("DELETE FROM events", []).is_err());
    assert!(
        connection
            .execute("UPDATE events SET event_json='{}'", [])
            .is_err()
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM assessment_attempts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    connection.execute("DELETE FROM progress", []).unwrap();
    assert_eq!(runtime.rebuild_progress().unwrap(), 2);
    assert_eq!(runtime.progress(PACKAGE, None).unwrap(), expected);
    let export = temp.path().join("events.jsonl");
    assert_eq!(runtime.export_state(&export).unwrap(), 2);
    assert!(runtime.export_state(&export).is_err());
    let lines: Vec<Value> = fs::read_to_string(export)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["export_version"], "0.1");
    assert_eq!(lines[1]["event"]["score"], 1);
    let backup = temp.path().join("backup.sqlite");
    runtime.backup(&backup).unwrap();
    assert!(runtime.backup(&backup).is_err());
    let saved = Connection::open(backup).unwrap();
    assert_eq!(
        saved
            .query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        saved
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn future_and_foreign_database_versions_are_preserved() {
    for sql in ["PRAGMA user_version=99", "PRAGMA application_id=42"] {
        let (temp, runtime) = setup();
        let path = runtime.database_path().to_path_buf();
        drop(runtime);
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(sql).unwrap();
        drop(connection);
        let before = fs::read(&path).unwrap();
        let errors = Runtime::open(&temp.path().join("home")).err().unwrap();
        assert_eq!(errors[0].code, "OSM_STATE_VERSION");
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn corrupt_and_unversioned_foreign_data_are_not_reinitialized() {
    for corrupt in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        fs::create_dir(&home).unwrap();
        let path = home.join("state.sqlite");
        if corrupt {
            fs::write(&path, "not a SQLite database").unwrap();
        } else {
            Connection::open(&path).unwrap().execute_batch("CREATE TABLE valuable_data(value TEXT); INSERT INTO valuable_data VALUES ('keep');").unwrap();
        }
        let before = fs::read(&path).unwrap();
        assert!(Runtime::open(&home).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }
}

#[test]
fn complete_files_reconcile_missing_database_registration() {
    let (temp, runtime) = setup();
    let path = runtime.database_path().to_path_buf();
    drop(runtime);
    Connection::open(&path)
        .unwrap()
        .execute("DELETE FROM packages", [])
        .unwrap();
    let runtime = Runtime::open(&temp.path().join("home")).unwrap();
    assert_eq!(runtime.packages().unwrap().len(), 1);
    assert_eq!(
        Connection::open(path)
            .unwrap()
            .query_row("SELECT count(*) FROM packages WHERE available=1", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn new_package_version_does_not_reinterpret_old_attempts() {
    let (temp, mut runtime) = setup();
    let old = runtime
        .answer(PACKAGE, None, &request(json!("b")))
        .unwrap()
        .event;
    let updated = temp.path().join("updated-source");
    for (name, bytes) in osmium_package::load_source(source()).unwrap().files {
        let path = updated.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let manifest_path = updated.join("osmium.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["package_version"] = json!("0.2.0");
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let path = updated.join("entities/assessments.json");
    let mut items: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    items[0]["revision"] = json!("2");
    items[0]["evaluation"]["answer"] = json!("a");
    fs::write(path, serde_json::to_vec(&items).unwrap()).unwrap();
    let distribution = temp.path().join("updated.osmium");
    build(&updated, &distribution).unwrap();
    runtime.install(&distribution).unwrap();
    assert_eq!(
        runtime.progress(PACKAGE, Some("0.2.0")).unwrap()[0].attempts,
        0
    );
    assert_eq!(
        runtime.progress(PACKAGE, Some("0.1.0")).unwrap()[0].attempts,
        1
    );
    assert_eq!(
        runtime.history(PACKAGE, Some("0.1.0"), 16, 0).unwrap(),
        [old]
    );
    assert_eq!(
        runtime
            .answer(PACKAGE, Some("0.2.0"), &request(json!("a")))
            .unwrap()
            .event["score"],
        1
    );
}

#[test]
fn uninstall_removes_only_the_requested_version_and_preserves_events() {
    let (temp, mut runtime) = setup();
    runtime
        .answer(PACKAGE, Some("0.1.0"), &request(json!("b")))
        .unwrap();

    let updated = temp.path().join("updated-source");
    for (name, bytes) in osmium_package::load_source(source()).unwrap().files {
        let path = updated.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let manifest_path = updated.join("osmium.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["package_version"] = json!("0.2.0");
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let archive = temp.path().join("updated.osmium");
    build(&updated, &archive).unwrap();
    runtime.install(&archive).unwrap();

    let removed = runtime.uninstall(PACKAGE, "0.1.0").unwrap();
    assert_eq!(removed.package_version, "0.1.0");
    assert_eq!(runtime.packages().unwrap().len(), 1);
    assert_eq!(runtime.packages().unwrap()[0].package_version, "0.2.0");
    assert_eq!(
        runtime
            .history(PACKAGE, Some("0.1.0"), 16, 0)
            .unwrap()
            .len(),
        1
    );
    let all_history = runtime.history_all(16, 0).unwrap();
    assert_eq!(all_history.len(), 1);
    assert_eq!(all_history[0]["package_version"], "0.1.0");
    assert_eq!(
        all_history[0]["assessment_snapshot"]["stimulus"]["markdown"],
        "1 + 1 はいくつ？"
    );
    assert!(runtime.progress(PACKAGE, Some("0.2.0")).unwrap()[0].attempts == 0);
}

#[test]
fn uninstall_succeeds_when_another_installed_version_is_damaged() {
    let (temp, mut runtime) = setup();
    runtime.answer(PACKAGE, Some("0.1.0"), &request(json!("b"))).unwrap();
    let updated = temp.path().join("updated-source");
    for (name, bytes) in osmium_package::load_source(source()).unwrap().files {
        let path = updated.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let manifest_path = updated.join("osmium.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["package_version"] = json!("0.2.0");
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let archive = temp.path().join("updated.osmium");
    build(&updated, &archive).unwrap();
    runtime.install(&archive).unwrap();
    let damaged_path = runtime.packages().unwrap().into_iter().find(|item| item.package_version == "0.2.0").unwrap().path.join("entities/resources.json");
    fs::write(damaged_path, b"tampered unrelated version").unwrap();

    let report = runtime.uninstall(PACKAGE, "0.1.0").unwrap();
    assert_eq!(report.package_version, "0.1.0");
    assert_eq!(runtime.package_inventory().unwrap().len(), 1);
    assert!(runtime.package_inventory().unwrap()[0].integrity_error.is_some());
    assert_eq!(runtime.history_all(16, 0).unwrap().len(), 1);
}

#[test]
fn damaged_payload_can_be_uninstalled_after_restart_without_losing_history() {
    let (temp, mut runtime) = setup();
    runtime
        .answer(PACKAGE, Some("0.1.0"), &request(json!("b")))
        .unwrap();
    let installed = runtime.packages().unwrap().remove(0);
    let damaged = installed.path.join("entities/resources.json");
    drop(runtime);
    fs::write(damaged, b"tampered payload").unwrap();

    let mut runtime = Runtime::open(&temp.path().join("home")).unwrap();
    let inventory = runtime.package_inventory().unwrap();
    assert_eq!(inventory.len(), 1);
    assert!(inventory[0].integrity_error.is_some());
    assert_eq!(
        runtime.uninstall(PACKAGE, "0.1.0").unwrap().package_version,
        "0.1.0"
    );
    assert!(runtime.packages().unwrap().is_empty());
    assert_eq!(
        runtime
            .history(PACKAGE, Some("0.1.0"), 16, 0)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(runtime.history_all(16, 0).unwrap().len(), 1);
}

#[test]
fn learner_reference_dto_keeps_evidence_but_never_an_authoring_locator() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/provenance-visibility-pressure-test");
    let distribution = temp.path().join("provenance.osmium");
    build(&fixture, &distribution).unwrap();
    let mut runtime = Runtime::open(&temp.path().join("home")).unwrap();
    runtime.install(&distribution).unwrap();

    let view = runtime
        .resource("org.osmium.pressure/provenance", None, "addition.lesson")
        .unwrap();
    let references = view["references"].as_array().unwrap();
    assert_eq!(references.len(), 3);
    // A safe publication query survives into the learner DTO.
    let public = references
        .iter()
        .find(|source| source["id"] == "public-safe-query")
        .unwrap();
    assert_eq!(
        public["locator"],
        "https://example.org/article?id=123&lang=ja"
    );
    assert_eq!(public["publisher"], "Example Organization");
    // A hidden locator keeps the record attributable but the URL never reaches
    // the DTO that the Desktop renders.
    let hidden = references
        .iter()
        .find(|source| source["id"] == "hidden-locator")
        .unwrap();
    assert!(hidden.get("locator").is_none());
    assert_eq!(hidden["locator_visibility"], "hidden");
    assert_eq!(
        hidden["citation"],
        "Example Organization. Internal document."
    );
    // No record is private, and the projection never emits a private marker.
    for source in references {
        assert_ne!(source["record_visibility"], "private");
        assert_ne!(source["visibility"], "private");
    }
    // The DTO is a projection: authoring-only fields such as content_hash never
    // travel even for a record that does distribute.
    assert!(
        references
            .iter()
            .all(|source| source.get("content_hash").is_none())
    );
}
