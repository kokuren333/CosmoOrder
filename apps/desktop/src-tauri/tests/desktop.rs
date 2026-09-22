//! Desktop application acceptance without a window.
//!
//! The desktop shell is a thin adapter over `osmium_store::runtime`, so this
//! test drives exactly the operations the shell calls: install the Golden
//! package, read the lesson and a resource, grade an answer, project progress,
//! and then reopen the session from scratch. It also pins the guarantees the
//! UI depends on: resource paths stay inside the distribution file map, the
//! compiled content IR never contains executable markup, and one package
//! version cannot silently change another version's history.

use osmium_core::content::{Block, Span};
use osmium_package::distribution::build;
use osmium_store::AttemptRequest;
use osmium_store::runtime::Runtime;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const PACKAGE_ID: &str = "org.example/arithmetic";

fn golden_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../examples/arithmetic")
        .canonicalize()
        .expect("the Golden package source exists")
}

fn build_golden(home: &Path) -> PathBuf {
    let archive = home.join("arithmetic.osmium");
    build(&golden_source(), &archive).expect("the Golden package builds");
    archive
}

fn open(home: &Path) -> Runtime {
    Runtime::open(home).expect("a session opens")
}

fn attempt(assessment_id: &str, response: Value) -> AttemptRequest {
    AttemptRequest {
        assessment_id: assessment_id.to_owned(),
        response,
        request_id: uuid::Uuid::new_v4().to_string(),
        duration_ms: None,
        hints_used: None,
    }
}

fn markdown_of(content: &Value) -> String {
    content["markdown"].as_str().unwrap_or_default().to_owned()
}

/// Copy a package source tree so a second version can be authored from it.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the copy root is created");
    for entry in std::fs::read_dir(from).expect("the source is readable") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a file is copied");
        }
    }
}

#[test]
fn desktop_reads_the_golden_package_and_records_progress_across_sessions() {
    let directory = TempDir::new().expect("temporary data root");
    let home = directory.path();
    let archive = build_golden(home);

    // install: the same operation the shell's install command uses
    let mut session = open(home);
    let report = session.install(&archive).expect("install succeeds");
    assert_eq!(report.package.package_id, PACKAGE_ID);
    assert!(!report.already_installed);
    drop(session);

    // list, lesson and resource, as the reader sees them
    let session = open(home);
    let packages = session.packages().expect("packages are listed");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].title, "足し算の基礎");

    let lesson = session
        .lesson(PACKAGE_ID, Some("0.1.0"))
        .expect("the lesson loads");
    assert_eq!(lesson["concepts"].as_array().unwrap().len(), 1);
    assert_eq!(lesson["objectives"].as_array().unwrap().len(), 1);
    assert_eq!(lesson["assessments"].as_array().unwrap().len(), 2);

    let resource = session
        .resource(PACKAGE_ID, Some("0.1.0"), "addition.lesson")
        .expect("the resource loads");
    assert_eq!(resource["content_is_untrusted"], json!(true));
    let markdown = markdown_of(&resource);
    assert!(markdown.contains("足し算"), "the body is the package text");
    drop(session);

    // grading: correct and incorrect answers produce distinct events
    let mut session = open(home);
    let correct = session
        .answer(
            PACKAGE_ID,
            Some("0.1.0"),
            &attempt("addition.01", json!("b")),
        )
        .expect("the correct answer is graded");
    assert!(correct.event["correct"].as_bool().unwrap());
    assert_eq!(correct.event["score"], json!(1));
    assert_eq!(
        correct.event["evaluator"]["id"],
        json!("org.osmium.exact.v1")
    );
    assert!(!correct.replayed);

    let incorrect = session
        .answer(
            PACKAGE_ID,
            Some("0.1.0"),
            &attempt("addition.01", json!("a")),
        )
        .expect("the incorrect answer is graded");
    assert!(!incorrect.event["correct"].as_bool().unwrap());
    assert_ne!(
        correct.event["event_id"], incorrect.event["event_id"],
        "a deliberate second answer is a new event"
    );

    let boolean = session
        .answer(
            PACKAGE_ID,
            Some("0.1.0"),
            &attempt("addition.02", json!(true)),
        )
        .expect("the boolean answer is graded");
    assert!(boolean.event["correct"].as_bool().unwrap());

    let progress = session
        .progress(PACKAGE_ID, Some("0.1.0"))
        .expect("progress is projected");
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].objective_id, "addition.basic");
    assert_eq!(progress[0].attempts, 3);
    assert_eq!(progress[0].correct, 2);
    assert_eq!(progress[0].accuracy, Some(2.0 / 3.0));

    let history = session
        .history(PACKAGE_ID, Some("0.1.0"), 32, 0)
        .expect("history is paged");
    assert_eq!(history.len(), 3);

    // an invalid answer is refused and changes nothing
    let refused = session.answer(
        PACKAGE_ID,
        Some("0.1.0"),
        &attempt("addition.01", json!("z")),
    );
    assert!(refused.is_err(), "an unavailable option is rejected");
    assert!(
        session
            .answer(
                PACKAGE_ID,
                Some("0.1.0"),
                &attempt("addition.02", json!("yes"))
            )
            .is_err(),
        "a non-boolean answer to a boolean item is rejected"
    );
    let unchanged = session
        .history(PACKAGE_ID, Some("0.1.0"), 32, 0)
        .expect("history is paged");
    assert_eq!(unchanged.len(), 3, "a refused answer records no event");
    drop(session);

    // a brand new process (fresh session) restores the same state
    let reopened = open(home);
    let restored = reopened
        .progress(PACKAGE_ID, Some("0.1.0"))
        .expect("progress is restored");
    assert_eq!(restored, progress);
    assert_eq!(
        reopened
            .history(PACKAGE_ID, Some("0.1.0"), 32, 0)
            .expect("history is restored")
            .len(),
        3
    );
    assert_eq!(
        reopened.packages().expect("packages are restored").len(),
        1,
        "the installed package survives"
    );
}

#[test]
fn rebuilding_progress_from_events_reproduces_the_projection() {
    let directory = TempDir::new().expect("temporary data root");
    let home = directory.path();
    let archive = build_golden(home);
    let mut session = open(home);
    session.install(&archive).expect("install succeeds");
    session
        .answer(
            PACKAGE_ID,
            Some("0.1.0"),
            &attempt("addition.01", json!("b")),
        )
        .expect("graded");
    session
        .answer(
            PACKAGE_ID,
            Some("0.1.0"),
            &attempt("addition.02", json!(false)),
        )
        .expect("graded");
    let before = session
        .progress(PACKAGE_ID, Some("0.1.0"))
        .expect("progress");
    let replayed = session.rebuild_progress().expect("the projection rebuilds");
    assert_eq!(replayed, 2);
    let after = session
        .progress(PACKAGE_ID, Some("0.1.0"))
        .expect("progress");
    assert_eq!(before, after, "events are the source of truth");
}

#[test]
fn one_package_version_does_not_change_another_versions_history() {
    let directory = TempDir::new().expect("temporary data root");
    let home = directory.path();
    let source = golden_source();
    let first = home.join("first.osmium");
    build(&source, &first).expect("the Golden package builds");

    // A second version of the same package, published as a new archive.
    let second_source = home.join("second");
    copy_tree(&source, &second_source);
    let manifest_path = second_source.join("osmium.json");
    let manifest = std::fs::read_to_string(&manifest_path).expect("the manifest is readable");
    std::fs::write(
        &manifest_path,
        manifest.replace(
            "\"package_version\": \"0.1.0\"",
            "\"package_version\": \"0.2.0\"",
        ),
    )
    .expect("the manifest is writable");
    let second = home.join("second.osmium");
    build(&second_source, &second).expect("the second version builds");

    let mut session = open(home);
    session.install(&first).expect("version 0.1.0 installs");
    session.install(&second).expect("version 0.2.0 installs");
    session
        .answer(
            PACKAGE_ID,
            Some("0.1.0"),
            &attempt("addition.01", json!("b")),
        )
        .expect("the first version records an event");

    let old = session
        .progress(PACKAGE_ID, Some("0.1.0"))
        .expect("old progress");
    assert_eq!(old[0].attempts, 1);
    let new = session
        .progress(PACKAGE_ID, Some("0.2.0"))
        .expect("new progress");
    assert_eq!(new[0].attempts, 0, "versions are not merged automatically");

    // The shell resolves an unspecified version explicitly instead of guessing.
    assert!(
        session.read(PACKAGE_ID, None).is_err(),
        "an ambiguous library requires an explicit version"
    );
}

#[test]
fn resource_bodies_compile_to_inert_content() {
    let directory = TempDir::new().expect("temporary data root");
    let home = directory.path();
    let archive = build_golden(home);
    let mut session = open(home);
    session.install(&archive).expect("install succeeds");

    // Every declared resource path is a key of the verified file map, so a
    // resource can never name a path outside the installed package.
    let package = session
        .read(PACKAGE_ID, Some("0.1.0"))
        .expect("the package reads");
    let resources = package
        .model
        .documents()
        .resources
        .as_array()
        .unwrap()
        .clone();
    assert!(!resources.is_empty());
    for resource in &resources {
        let path = resource["path"].as_str().unwrap();
        assert!(
            package.files.contains_key(path),
            "{path} must be a verified distribution file"
        );
        let view = session
            .resource(PACKAGE_ID, Some("0.1.0"), resource["id"].as_str().unwrap())
            .expect("the resource loads");
        assert!(
            !markdown_of(&view).contains('\r'),
            "distribution Markdown is LF only"
        );
    }
    drop(session);

    // A block-level HTML run stays inert, and an inline one stays text inside
    // its paragraph. Neither produces structure the renderer could execute.
    let markdown = "# 見出し\n\n<script>alert(1)</script>\n\n本文 <b>太字</b> と [link](javascript:alert(1))\n";
    let content = osmium_core::content::compile_markdown(markdown).expect("markdown compiles");
    let mut has_html_block = false;
    let mut saw_inline_html_text = false;
    for block in &content.blocks {
        match block {
            Block::Html { text } => {
                has_html_block = true;
                assert!(
                    text.contains("<script>"),
                    "the tag survives as literal text"
                );
            }
            Block::Paragraph { spans } => {
                for span in spans {
                    assert!(
                        !matches!(span, Span::Link { .. }),
                        "a javascript: destination never becomes a link"
                    );
                    if let Span::Text { text } = span {
                        if text.contains("<b>") {
                            saw_inline_html_text = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    assert!(has_html_block, "block HTML survives only as an inert block");
    assert!(
        saw_inline_html_text,
        "inline HTML survives only as literal text"
    );
    assert!(content.to_plain_text().contains("<script>"));
}
