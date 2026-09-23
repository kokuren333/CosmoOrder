//! End-to-end CLI contract tests.
//!
//! They drive the library entry point rather than spawning the binary so the
//! suite stays portable, but they assert the whole observable contract:
//! stdout JSON, stderr text, exit status and Windows path handling.

use osmium_cli::{Exit, run_from};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

struct Outcome {
    exit: Exit,
    stdout: Value,
    stderr: String,
}

/// Run one command line. Arguments are passed as separate OS strings, so a
/// Windows path with spaces or backslashes is never re-split.
fn run(arguments: &[&str]) -> Outcome {
    // A fresh vector per call: `OsString: FromIterator` consumes its source, so
    // reusing one argument list would silently pass no arguments.
    let arguments: Vec<OsString> = arguments.iter().map(OsString::from).collect();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_from(arguments, &mut stdout, &mut stderr);
    let stdout = String::from_utf8(stdout).expect("stdout is UTF-8 JSON");
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout must hold exactly one JSON document: {stdout}"
    );
    Outcome {
        exit,
        stdout: serde_json::from_str(&stdout).expect("stdout is one JSON document"),
        stderr: String::from_utf8(stderr).expect("stderr is UTF-8"),
    }
}

/// Copy the example package into a writable temporary directory.
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
    let directory = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic"),
        directory.path(),
    );
    directory
}

fn edit(root: &Path, name: &str, change: impl FnOnce(&mut Value)) {
    let path = root.join(name);
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    change(&mut value);
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

fn codes(outcome: &Outcome) -> Vec<String> {
    outcome.stdout["diagnostics"]
        .as_array()
        .expect("diagnostics is always an array")
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap().to_owned())
        .collect()
}

fn path_of(directory: &tempfile::TempDir) -> String {
    directory.path().to_string_lossy().into_owned()
}

#[test]
fn lint_reports_advice_without_failing_or_claiming_quality() {
    let directory = source();
    edit(directory.path(), "entities/assessments.json", |v| {
        *v = json!([])
    });
    let outcome = run(&["osmium", "lint", &path_of(&directory), "--json"]);
    assert_eq!(outcome.exit, Exit::Success);
    assert_eq!(outcome.stdout["ok"], true);
    assert!(codes(&outcome).contains(&"OSM_LINT_OBJECTIVE_NO_ASSESSMENT".into()));
    assert!(
        outcome.stdout["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "OSM_LINT_OBJECTIVE_NO_ASSESSMENT"
                && item["entity_type"] == "objective"
                && item["entity_id"] == "addition.basic")
    );
    assert!(codes(&outcome).contains(&"OSM_LINT_METADATA".into()));
    assert!(outcome.stdout["data"].get("quality_score").is_none());
    assert!(outcome.stderr.contains("warning"));
}

#[test]
fn inspect_rejects_oversized_limit_and_json_flag_is_supported() {
    let directory = source();
    assert_eq!(
        run(&["osmium", "inspect", &path_of(&directory), "--limit", "65"]).exit,
        Exit::Usage
    );
    assert_eq!(
        run(&["osmium", "validate", &path_of(&directory), "--json"]).exit,
        Exit::Success
    );
}

#[test]
fn actual_binary_outputs_one_json_document() {
    let directory = source();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_osmium"))
        .arg("validate")
        .arg(directory.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["ok"], true);
    assert!(output.stderr.is_empty());
}

#[test]
fn installed_package_survives_independent_cli_processes() {
    let temp = tempfile::tempdir().unwrap();
    let zip = temp.path().join("course.osmium");
    let home = temp.path().join("home");
    osmium_package::distribution::build(source().path(), &zip).unwrap();
    let invoke = |args: &[&std::ffi::OsStr]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_osmium"))
            .arg("--home")
            .arg(&home)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let installed = invoke(&[std::ffi::OsStr::new("install"), zip.as_os_str()]);
    assert_eq!(installed["data"]["already_installed"], false);
    let repeated = invoke(&[std::ffi::OsStr::new("install"), zip.as_os_str()]);
    assert_eq!(repeated["data"]["already_installed"], true);
    let packages = invoke(&[std::ffi::OsStr::new("packages")]);
    assert_eq!(packages["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        packages["data"][0]["digest"],
        installed["data"]["package"]["digest"]
    );
}

#[test]
fn offline_learning_vertical_slice_survives_cli_process_restart() {
    let temp = tempfile::tempdir().unwrap();
    let zip = temp.path().join("arithmetic.osmium");
    let home = temp.path().join("home");
    let source = source();
    let invoke = |args: &[&str]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_osmium"))
            .arg("--home")
            .arg(&home)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["data"].clone()
    };
    invoke(&["validate", &path_of(&source)]);
    invoke(&["lint", &path_of(&source)]);
    invoke(&[
        "build",
        &path_of(&source),
        "--output",
        &zip.to_string_lossy(),
    ]);
    invoke(&["install", &zip.to_string_lossy()]);
    let lesson = invoke(&["learn", "org.example/arithmetic"]);
    assert_eq!(lesson["concepts"][0]["id"], "addition");
    let resource = lesson["resources"][0]["id"].as_str().unwrap();
    assert!(
        !invoke(&["read", "org.example/arithmetic", resource])["markdown"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    let request = "6e2b0864-2e66-4dd4-a02f-f57b59cfe8b1";
    let answer = [
        "answer",
        "org.example/arithmetic",
        "addition.01",
        "--response",
        "\"b\"",
        "--request-id",
        request,
    ];
    let result = invoke(&answer);
    assert_eq!(result["event"]["score"], 1);
    assert_eq!(invoke(&answer)["replayed"], true);
    invoke(&[
        "answer",
        "org.example/arithmetic",
        "addition.01",
        "--response",
        "\"a\"",
    ]);
    let progress = invoke(&["progress", "org.example/arithmetic"]);
    assert_eq!(progress[0]["attempts"], 2);
    assert_eq!(progress[0]["accuracy"], 0.5);
    assert_eq!(
        invoke(&["history", "org.example/arithmetic"])
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(invoke(&["rebuild-progress"])["events_replayed"], 2);
    assert_eq!(invoke(&["progress", "org.example/arithmetic"]), progress);
    let export = temp.path().join("history.jsonl");
    assert_eq!(
        invoke(&["export-state", "--output", &export.to_string_lossy()])["events_exported"],
        2
    );
    let backup = temp.path().join("backup.sqlite");
    invoke(&["backup-state", "--output", &backup.to_string_lossy()]);
    assert!(backup.exists());
    assert!(home.join("state.sqlite").exists());
}

#[test]
fn validate_reports_a_valid_source_as_success() {
    let directory = source();
    let outcome = run(&["osmium", "validate", &path_of(&directory)]);
    assert_eq!(outcome.exit, Exit::Success);
    assert_eq!(outcome.exit.code(), 0);
    assert_eq!(outcome.stdout["output_version"], "0.1");
    assert_eq!(outcome.stdout["ok"], true);
    assert_eq!(
        outcome.stdout["data"]["package_id"],
        "org.example/arithmetic"
    );
    assert_eq!(outcome.stdout["data"]["package_version"], "0.1.0");
    assert_eq!(outcome.stdout["data"]["schema_version"], "0.1");
    assert_eq!(outcome.stdout["data"]["total_entities"], 6);
    assert_eq!(outcome.stdout["data"]["files_read"], 7);
    assert!(codes(&outcome).is_empty());
    assert_eq!(outcome.stderr, "");
}

#[test]
fn the_json_flag_is_accepted_and_does_not_change_stdout() {
    let directory = source();
    let plain = run(&["osmium", "validate", &path_of(&directory)]);
    let explicit = run(&[
        "osmium",
        "validate",
        &path_of(&directory),
        "--output",
        "json",
    ]);
    assert_eq!(explicit.exit, Exit::Success);
    assert_eq!(plain.stdout, explicit.stdout);
    let rejected = run(&[
        "osmium",
        "validate",
        &path_of(&directory),
        "--output",
        "yaml",
    ]);
    assert_eq!(rejected.exit, Exit::Usage);
    assert_eq!(rejected.stdout["ok"], false);
}

#[test]
fn a_broken_reference_is_an_invalid_package() {
    let directory = source();
    edit(directory.path(), "entities/objectives.json", |value| {
        value[0]["concept"] = json!("missing.concept");
    });
    let outcome = run(&["osmium", "validate", &path_of(&directory)]);
    assert_eq!(outcome.exit, Exit::Invalid);
    assert_eq!(outcome.exit.code(), 1);
    assert_eq!(outcome.stdout["ok"], false);
    assert!(codes(&outcome).contains(&"OSM_REFERENCE".to_owned()));
    assert!(outcome.stderr.contains("OSM_REFERENCE"));
}

#[test]
fn an_unsupported_schema_version_is_an_incompatibility() {
    let directory = source();
    edit(directory.path(), "osmium.json", |value| {
        value["schema_version"] = json!("9.9");
    });
    let outcome = run(&["osmium", "validate", &path_of(&directory)]);
    assert_eq!(outcome.exit, Exit::Incompatible);
    assert_eq!(outcome.exit.code(), 4);
    assert_eq!(codes(&outcome), ["OSM_SCHEMA_VERSION"]);
}

#[test]
fn an_unsupported_required_capability_is_an_incompatibility() {
    let directory = source();
    edit(directory.path(), "osmium.json", |value| {
        value["capabilities"]["required"] = json!(["org.osmium.media.v1"]);
    });
    let outcome = run(&["osmium", "inspect", &path_of(&directory)]);
    assert_eq!(outcome.exit, Exit::Incompatible);
    assert_eq!(outcome.exit.code(), 4);
    assert!(codes(&outcome).contains(&"OSM_CAPABILITY".to_owned()));
}

#[test]
fn missing_path_is_usage_error_and_non_archive_file_is_invalid() {
    let missing = run(&["osmium", "validate", "no-such-package-here"]);
    assert_eq!(missing.exit, Exit::Usage);
    assert_eq!(missing.exit.code(), 2);
    assert_eq!(missing.stdout["ok"], false);

    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("osmium.json");
    fs::write(&file, "{}").unwrap();
    let not_a_directory = run(&["osmium", "validate", &file.to_string_lossy()]);
    assert_eq!(not_a_directory.exit, Exit::Invalid);
    assert_eq!(codes(&not_a_directory), ["OSM_DISTRIBUTION"]);
}

#[test]
fn build_output_path_and_json_output_flag_work_with_distribution_readers() {
    let source = source();
    let output = tempfile::tempdir().unwrap();
    for name in ["folder", "package.osmium"] {
        let path = output.path().join(name).to_string_lossy().into_owned();
        let built = run(&[
            "osmium",
            "build",
            &path_of(&source),
            "--output",
            &path,
            "--json",
        ]);
        assert_eq!(built.exit, Exit::Success, "{}", built.stderr);
        assert_eq!(built.stdout["data"]["digest"].as_str().unwrap().len(), 64);
        for command in ["validate", "inspect", "lint"] {
            assert_eq!(
                run(&["osmium", command, &path, "--json"]).exit,
                Exit::Success
            );
        }
        assert_eq!(
            run(&["osmium", "query", &path, "concepts"]).exit,
            Exit::Success
        );
        assert_eq!(
            run(&["osmium", "context", &path, "addition.basic"]).exit,
            Exit::Success
        );
        assert_eq!(
            run(&["osmium", "build", &path_of(&source), "--output", &path]).exit,
            Exit::Invalid
        );
    }
}

#[test]
fn inspect_reports_manifest_metadata_and_counts() {
    let directory = source();
    let outcome = run(&["osmium", "inspect", &path_of(&directory)]);
    assert_eq!(outcome.exit, Exit::Success);
    let manifest = &outcome.stdout["data"]["manifest"];
    assert_eq!(manifest["package_id"], "org.example/arithmetic");
    assert_eq!(manifest["title"], "足し算の基礎");
    assert_eq!(manifest["language"], "ja-JP");
    assert_eq!(manifest["entity_counts"]["assessments"], 2);
    assert_eq!(manifest["prerequisite_order"], json!(["addition"]));
    assert_eq!(manifest["prerequisite_order_truncated"], false);
    assert_eq!(outcome.stdout["data"]["total_entities"], 6);
}

#[test]
fn query_pages_and_rejects_an_unbounded_limit() {
    let directory = source();
    let path = path_of(&directory);
    let all = run(&["osmium", "query", &path, "assessments"]);
    assert_eq!(all.exit, Exit::Success);
    assert_eq!(all.stdout["data"]["total"], 2);
    assert_eq!(all.stdout["data"]["returned"], 2);
    assert_eq!(all.stdout["data"]["truncated"], false);
    assert_eq!(all.stdout["data"]["entities"][0]["id"], "addition.01");
    assert_eq!(
        all.stdout["data"]["entities"][0]["detail"]["response_type"],
        "single_select"
    );

    let page = run(&["osmium", "query", &path, "assessments", "--offset", "1"]);
    assert_eq!(page.stdout["data"]["returned"], 1);
    assert_eq!(page.stdout["data"]["entities"][0]["id"], "addition.02");

    let over = run(&["osmium", "query", &path, "assessments", "--limit", "100000"]);
    assert_eq!(over.exit, Exit::Usage);
    assert_eq!(codes(&over), ["OSM_QUERY_LIMIT"]);

    let unknown = run(&["osmium", "query", &path, "bogus"]);
    assert_eq!(unknown.exit, Exit::Usage);
    assert_eq!(codes(&unknown), ["OSM_QUERY_KIND"]);
}

#[test]
fn context_returns_the_bounded_neighborhood_of_an_entity() {
    let directory = source();
    let path = path_of(&directory);
    let outcome = run(&["osmium", "context", &path, "addition.basic"]);
    assert_eq!(outcome.exit, Exit::Success);
    let data = &outcome.stdout["data"];
    assert_eq!(data["content_is_untrusted"], true);
    assert_eq!(data["target"]["kind"], "objective");
    let ids: Vec<&str> = data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    for expected in [
        "addition.basic",
        "addition",
        "addition.lesson",
        "addition.01",
    ] {
        assert!(ids.contains(&expected), "{expected} missing from {ids:?}");
    }
    assert_eq!(data["prerequisite_depth"], Value::Null);
}

#[test]
fn context_bounds_depth_and_reports_concept_prerequisites() {
    let directory = source();
    let path = path_of(&directory);
    let shallow = run(&["osmium", "context", &path, "addition", "--depth", "1"]);
    assert_eq!(shallow.exit, Exit::Success);
    assert_eq!(shallow.stdout["data"]["prerequisite_depth"], 0);
    let ids: Vec<&str> = shallow.stdout["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["addition", "addition.basic"]);

    let deep = run(&["osmium", "context", &path, "addition", "--depth", "9"]);
    assert_eq!(deep.exit, Exit::Usage);
    assert_eq!(codes(&deep), ["OSM_CONTEXT_DEPTH"]);
}

#[test]
fn context_requires_a_kind_only_when_an_id_is_ambiguous() {
    let directory = source();
    // A Concept and an Objective may share one ID because IDs are unique per
    // kind, not per package. References must keep resolving so that the
    // ambiguity, not a broken reference, is what the command reports.
    edit(directory.path(), "entities/objectives.json", |value| {
        value[0]["id"] = json!("addition");
    });
    // Every reference must follow the rename, including the second assessment.
    // Resources declare `teaches` and assessments declare `measures`, so the
    // two documents need different edits.
    edit(directory.path(), "entities/resources.json", |value| {
        for entity in value.as_array_mut().unwrap() {
            entity["teaches"] = json!(["addition"]);
        }
    });
    edit(directory.path(), "entities/assessments.json", |value| {
        for entity in value.as_array_mut().unwrap() {
            entity["measures"] = json!(["addition"]);
        }
    });
    edit(directory.path(), "entities/curricula.json", |value| {
        for entity in value.as_array_mut().unwrap() {
            entity["objectives"] = json!(["addition"]);
        }
    });
    let path = path_of(&directory);
    let ambiguous = run(&["osmium", "context", &path, "addition"]);
    assert_eq!(
        ambiguous.exit,
        Exit::Invalid,
        "the package must stay otherwise valid: {}",
        ambiguous.stdout["diagnostics"]
    );
    assert_eq!(
        codes(&ambiguous),
        ["OSM_AMBIGUOUS_ENTITY"],
        "diagnostics: {}",
        ambiguous.stdout["diagnostics"]
    );

    let resolved = run(&["osmium", "context", &path, "addition", "--kind", "concept"]);
    assert_eq!(resolved.exit, Exit::Success);
    assert_eq!(resolved.stdout["data"]["target"]["kind"], "concept");

    let unknown = run(&["osmium", "context", &path, "not.an.entity"]);
    assert_eq!(unknown.exit, Exit::Invalid);
    assert_eq!(codes(&unknown), ["OSM_UNKNOWN_ENTITY"]);
}

#[test]
fn init_creates_a_valid_package_and_never_overwrites() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("new lesson");
    let target = target.to_string_lossy().into_owned();
    let created = run(&["osmium", "init", &target, "--package-id", "org.example/new"]);
    assert_eq!(created.exit, Exit::Success);
    assert_eq!(created.stdout["data"]["package_id"], "org.example/new");
    assert_eq!(
        created.stdout["data"]["created_files"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    assert_eq!(
        created.stdout["data"]["created_files"][0], "osmium.json",
        "the manifest is written first so a partial scaffold is obvious"
    );

    let validated = run(&["osmium", "validate", &target]);
    assert_eq!(validated.exit, Exit::Success);
    assert_eq!(validated.stdout["data"]["package_id"], "org.example/new");
    assert_eq!(validated.stdout["data"]["total_entities"], 5);

    let refused = run(&["osmium", "init", &target]);
    assert_eq!(refused.exit, Exit::Usage);
    assert_eq!(codes(&refused), ["OSM_INIT_EXISTS"]);
    let manifest = fs::read_to_string(PathBuf::from(&target).join("osmium.json")).unwrap();
    assert!(manifest.contains("org.example/new"));
}

#[test]
fn init_derives_identity_from_the_directory_and_rejects_bad_ids() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("Basics 101");
    let target_text = target.to_string_lossy().into_owned();
    let created = run(&["osmium", "init", &target_text]);
    assert_eq!(created.exit, Exit::Success);
    assert_eq!(
        created.stdout["data"]["package_id"],
        "org.example/basics-101"
    );
    assert_eq!(created.stdout["data"]["title"], "Basics 101");

    let other = directory.path().join("other");
    let bad = run(&[
        "osmium",
        "init",
        &other.to_string_lossy(),
        "--package-id",
        "Not/AValid/Id",
    ]);
    assert_eq!(bad.exit, Exit::Usage);
    assert_eq!(codes(&bad), ["OSM_INIT_ID"]);
    assert!(
        !other.exists(),
        "a rejected init must not create the target"
    );

    let bad_language = run(&[
        "osmium",
        "init",
        &directory.path().join("third").to_string_lossy(),
        "--language",
        "not a tag",
    ]);
    assert_eq!(bad_language.exit, Exit::Usage);
    assert_eq!(codes(&bad_language), ["OSM_INIT_LANGUAGE"]);
}

#[test]
fn a_usage_mistake_exits_two_with_an_envelope() {
    let outcome = run(&["osmium", "query", "some-path"]);
    assert_eq!(outcome.exit, Exit::Usage);
    assert_eq!(outcome.stdout["ok"], false);
    assert_eq!(codes(&outcome), ["OSM_USAGE"]);
    assert!(!outcome.stderr.is_empty(), "a human must see the reason");
}

#[test]
fn help_and_version_are_requested_output_not_failures() {
    for arguments in [vec!["osmium", "--help"], vec!["osmium", "--version"]] {
        let arguments: Vec<OsString> = arguments.into_iter().map(OsString::from).collect();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_from(arguments, &mut stdout, &mut stderr);
        assert_eq!(exit, Exit::Success);
        // clap prints requested help to stderr, and that output is not an
        // envelope. The point of the test is that the status is 0 and no JSON
        // document is emitted on stdout.
        assert!(!stderr.is_empty(), "help text must reach the user");
        assert!(stdout.is_empty(), "help is not a machine result");
    }
}

#[test]
fn every_diagnostic_carries_the_contract_fields() {
    let directory = source();
    edit(directory.path(), "entities/assessments.json", |value| {
        value[0]["evaluation"]["answer"] = json!("zzz");
    });
    let outcome = run(&["osmium", "validate", &path_of(&directory)]);
    assert_eq!(outcome.exit, Exit::Invalid);
    let diagnostic = outcome.stdout["diagnostics"]
        .as_array()
        .unwrap()
        .first()
        .expect("an invalid package reports at least one diagnostic")
        .clone();
    for field in [
        "code",
        "severity",
        "file",
        "line",
        "column",
        "path",
        "message",
        "suggestions",
    ] {
        assert!(
            diagnostic.get(field).is_some(),
            "diagnostic is missing {field}: {diagnostic}"
        );
    }
    // The loader maps semantic diagnostics onto the real file name, and no
    // source position is invented when the parser cannot supply one.
    assert_eq!(diagnostic["file"], "entities/assessments.json");
    assert_eq!(diagnostic["line"], Value::Null);
    assert_eq!(diagnostic["column"], Value::Null);
}
