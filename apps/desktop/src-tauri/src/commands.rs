//! Tauri commands: the whole desktop surface.
//!
//! Every command is a thin adapter over an existing application operation in
//! `osmium_store::runtime`. Nothing here validates, grades, orders or
//! reinterprets a package: the shell only decodes IPC arguments, resolves an
//! installed version when the caller left it open, and serializes what Core
//! and Package returned. Grading stays in `osmium_core::evaluation`, reached
//! through `Runtime::answer`, so the UI cannot disagree with the CLI.

use crate::error::{CommandError, CommandResult};
use osmium_core::content::{Content, compile_markdown};
use osmium_package::library::InstalledPackage;
use osmium_store::runtime::Runtime;
use osmium_store::{AttemptRequest, AttemptResult, ObjectiveProgress};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// The desktop holds one application session for the lifetime of the process,
/// which keeps the library lock for the whole run. The CLI opens a session per
/// invocation instead; both use the same operations.
pub struct Desktop {
    runtime: Mutex<Runtime>,
}

impl Desktop {
    pub fn open(home: PathBuf) -> Result<Self, CommandError> {
        let runtime = Runtime::open(&home).map_err(CommandError::new)?;
        Ok(Self {
            runtime: Mutex::new(runtime),
        })
    }

    /// Run one operation with the session lock held.
    fn with<T>(
        &self,
        operation: impl FnOnce(&mut Runtime) -> Result<T, Vec<osmium_core::schema::Diagnostic>>,
    ) -> CommandResult<T> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| CommandError::shell("OSM_SHELL", "the application session is poisoned"))?;
        operation(&mut runtime).map_err(CommandError::new)
    }

    /// Resolve the version to operate on. The UI never has to know the version
    /// resolution rules: leaving it unset means "the only installed version",
    /// and an ambiguous library is reported exactly as the CLI reports it.
    fn resolve_version(
        &self,
        package_id: &str,
        version: Option<String>,
    ) -> CommandResult<Option<String>> {
        if version.is_some() {
            return Ok(version);
        }
        let packages = self.with(|runtime| runtime.packages())?;
        let matching: Vec<&InstalledPackage> = packages
            .iter()
            .filter(|package| package.package_id == package_id)
            .collect();
        match matching.len() {
            0 => Err(CommandError::shell(
                "OSM_NOT_INSTALLED",
                "requested package is not installed",
            )),
            1 => Ok(Some(matching[0].package_version.clone())),
            _ => Err(CommandError::shell(
                "OSM_VERSION_REQUIRED",
                "multiple versions are installed; choose one",
            )),
        }
    }
}

/// Data root, database file and installed package count.
#[derive(Debug, Serialize)]
pub struct StatusView {
    pub home: String,
    pub database: String,
    pub packages: usize,
    pub state_version: i64,
}

/// One installed package as the package list shows it.
#[derive(Debug, Serialize)]
pub struct PackageView {
    pub package_id: String,
    pub package_version: String,
    pub schema_version: String,
    pub title: String,
    pub entity_counts: std::collections::BTreeMap<String, usize>,
    pub digest: String,
    pub selected_version: String,
}

/// Typed, renderer-neutral content returned across Desktop IPC. Markdown is
/// compiled by Core before this boundary and its source text is not duplicated
/// into the renderer DTO.
#[derive(Debug, Serialize)]
pub struct ContentView {
    /// Plain-text projection used for compact previews, derived from the IR.
    pub text: String,
    pub content: Content,
}

impl ContentView {
    fn compile(source: &str) -> CommandResult<Self> {
        let content =
            compile_markdown(source).map_err(|error| CommandError::shell("OSM_CONTENT", error))?;
        let text = content.to_plain_text();
        Ok(Self { text, content })
    }
}

/// A whole lesson: the verified manifest plus every entity document, exactly as
/// the package authored them. The reader navigates this without re-deriving it.
#[derive(Debug, Serialize)]
pub struct LessonView {
    pub package_id: String,
    pub package_version: String,
    pub digest: String,
    pub manifest: Value,
    pub concepts: Value,
    pub objectives: Value,
    pub curricula: Value,
    pub resources: Value,
    pub assessments: Value,
    /// Compiled stimulus Markdown, keyed by assessment ID.
    pub stimuli: std::collections::BTreeMap<String, ContentView>,
}

/// A resource body plus its compiled, inert content IR.
#[derive(Debug, Serialize)]
pub struct ResourceView {
    pub package_id: String,
    pub package_version: String,
    pub digest: String,
    pub resource: Value,
    pub content: Content,
    pub content_is_untrusted: bool,
}

/// The outcome of one graded attempt, straight from the evaluation engine.
#[derive(Debug, Serialize)]
pub struct AttemptView {
    pub event: Value,
    pub replayed: bool,
    pub correct: bool,
    pub score: u8,
    pub feedback: Value,
    pub feedback_content: ContentView,
    pub evaluator: Value,
    pub objective_ids: Vec<String>,
    pub assessment_id: String,
}

fn object_string(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[tauri::command]
pub fn status(desktop: State<'_, Desktop>) -> CommandResult<StatusView> {
    let packages = desktop.with(|runtime| runtime.packages())?;
    desktop.with(|runtime| {
        Ok(StatusView {
            home: runtime.home().to_string_lossy().into_owned(),
            database: runtime.database_path().to_string_lossy().into_owned(),
            packages: packages.len(),
            state_version: osmium_store::STATE_VERSION,
        })
    })
}

#[tauri::command]
pub fn list_packages(desktop: State<'_, Desktop>) -> CommandResult<Vec<PackageView>> {
    let packages = desktop.with(|runtime| runtime.packages())?;
    Ok(packages
        .into_iter()
        .map(|package| PackageView {
            selected_version: package.package_version.clone(),
            package_id: package.package_id,
            package_version: package.package_version,
            schema_version: package.schema_version,
            title: package.title,
            entity_counts: package.entity_counts,
            digest: package.digest,
        })
        .collect())
}

#[tauri::command]
pub fn open_lesson(
    desktop: State<'_, Desktop>,
    package_id: String,
    version: Option<String>,
) -> CommandResult<LessonView> {
    let version = desktop.resolve_version(&package_id, version)?;
    let lesson = desktop.with(|runtime| runtime.lesson(&package_id, version.as_deref()))?;
    let manifest = lesson["manifest"].clone();
    let mut stimuli = std::collections::BTreeMap::new();
    if let Some(items) = lesson["assessments"].as_array() {
        for assessment in items {
            let Some(id) = assessment["id"].as_str() else {
                continue;
            };
            let markdown = assessment["stimulus"]["markdown"]
                .as_str()
                .unwrap_or_default();
            stimuli.insert(id.to_owned(), ContentView::compile(markdown)?);
        }
    }
    Ok(LessonView {
        package_id: object_string(&manifest, "package_id"),
        package_version: object_string(&manifest, "package_version"),
        digest: object_string(&lesson, "digest"),
        manifest,
        concepts: lesson["concepts"].clone(),
        objectives: lesson["objectives"].clone(),
        curricula: lesson["curricula"].clone(),
        resources: lesson["resources"].clone(),
        assessments: lesson["assessments"].clone(),
        stimuli,
    })
}

#[tauri::command]
pub fn read_resource(
    desktop: State<'_, Desktop>,
    package_id: String,
    version: Option<String>,
    resource_id: String,
) -> CommandResult<ResourceView> {
    let version = desktop.resolve_version(&package_id, version)?;
    let view =
        desktop.with(|runtime| runtime.resource(&package_id, version.as_deref(), &resource_id))?;
    let markdown = view["markdown"].as_str().unwrap_or_default();
    // Compilation is Core's job; the shell only decides how to report failure.
    let content =
        compile_markdown(markdown).map_err(|error| CommandError::shell("OSM_CONTENT", error))?;
    Ok(ResourceView {
        package_id: package_id.clone(),
        package_version: version.unwrap_or_default(),
        digest: object_string(&view, "digest"),
        resource: view["resource"].clone(),
        content,
        content_is_untrusted: true,
    })
}

/// One submitted answer, as the renderer sends it. Keeping the fields in a
/// single decoded value keeps the command signature small and makes the IPC
/// payload explicit.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerRequest {
    pub package_id: String,
    pub version: Option<String>,
    pub assessment_id: String,
    pub response: Value,
    pub request_id: Option<String>,
    pub duration_ms: Option<u64>,
    pub hints_used: Option<u64>,
}

/// Submit an answer. The response is graded by the shared evaluation engine and
/// recorded as an append-only learning event.
#[tauri::command]
pub fn submit_attempt(
    desktop: State<'_, Desktop>,
    request: AnswerRequest,
) -> CommandResult<AttemptView> {
    let version = desktop.resolve_version(&request.package_id, request.version)?;
    let attempt = AttemptRequest {
        assessment_id: request.assessment_id,
        response: request.response,
        // A fresh UUID per submission keeps a retry of the same attempt
        // idempotent while a deliberate second answer is a new event.
        request_id: request
            .request_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        duration_ms: request.duration_ms,
        hints_used: request.hints_used,
    };
    let result: AttemptResult = desktop
        .with(|runtime| runtime.answer(&request.package_id, version.as_deref(), &attempt))?;
    let event = result.event;
    let feedback = event["assessment_snapshot"]["feedback"]["markdown"]
        .as_str()
        .unwrap_or_default();
    Ok(AttemptView {
        assessment_id: event["assessment_id"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        correct: event["correct"].as_bool().unwrap_or(false),
        score: event["score"].as_u64().unwrap_or(0) as u8,
        feedback: event["assessment_snapshot"]["feedback"].clone(),
        feedback_content: ContentView::compile(feedback)?,
        evaluator: event["evaluator"].clone(),
        objective_ids: event["objective_ids"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        event,
        replayed: result.replayed,
    })
}

#[tauri::command]
pub fn progress(
    desktop: State<'_, Desktop>,
    package_id: String,
    version: Option<String>,
) -> CommandResult<Vec<ObjectiveProgress>> {
    let version = desktop.resolve_version(&package_id, version)?;
    desktop.with(|runtime| runtime.progress(&package_id, version.as_deref()))
}

#[tauri::command]
pub fn history(
    desktop: State<'_, Desktop>,
    package_id: String,
    version: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> CommandResult<Vec<Value>> {
    let version = desktop.resolve_version(&package_id, version)?;
    let limit = limit.unwrap_or(32).clamp(1, 64);
    let offset = offset.unwrap_or(0);
    desktop.with(|runtime| runtime.history(&package_id, version.as_deref(), limit, offset))
}

#[tauri::command]
pub fn rebuild_progress(desktop: State<'_, Desktop>) -> CommandResult<Value> {
    let events = desktop.with(|runtime| runtime.rebuild_progress())?;
    Ok(json!({"events_replayed": events}))
}

/// Export the learning event log as JSONL. The caller supplies an absolute
/// destination; the store refuses to overwrite an existing file.
#[tauri::command]
pub fn export_state(desktop: State<'_, Desktop>, output: String) -> CommandResult<Value> {
    if output.trim().is_empty() {
        return Err(CommandError::shell(
            "OSM_SHELL",
            "an export destination is required",
        ));
    }
    let output = PathBuf::from(output);
    let events = desktop.with(|runtime| runtime.export_state(&output))?;
    Ok(json!({"events_exported": events, "output": output.to_string_lossy()}))
}

#[tauri::command]
pub fn backup_state(desktop: State<'_, Desktop>, output: String) -> CommandResult<Value> {
    if output.trim().is_empty() {
        return Err(CommandError::shell(
            "OSM_SHELL",
            "a backup destination is required",
        ));
    }
    let output = PathBuf::from(output);
    desktop.with(|runtime| runtime.backup(&output))?;
    Ok(json!({"output": output.to_string_lossy()}))
}

#[cfg(test)]
mod content_view_tests {
    use super::ContentView;

    #[test]
    fn desktop_content_dto_contains_ir_and_derived_text_but_not_markdown_source() {
        let view = ContentView::compile("# Title\n\nA **typed** paragraph.").expect("content");
        let value = serde_json::to_value(view).expect("serializable DTO");
        assert_eq!(value["text"], "Title\nA typed paragraph.");
        assert!(value["content"]["blocks"].is_array());
        assert!(value.get("markdown").is_none());
    }
}
