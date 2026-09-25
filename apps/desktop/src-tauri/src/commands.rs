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
use osmium_package::{library::InstalledPackage, load_source};
use osmium_store::runtime::Runtime;
use osmium_store::{AttemptRequest, AttemptResult, ObjectiveProgress};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

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

    fn package_context(
        &self,
        package_id: &str,
        version: Option<&str>,
        kind: &str,
        entity_id: &str,
        depth: usize,
        node_limit: usize,
    ) -> CommandResult<osmium_core::query::ContextView> {
        self.with(|runtime| {
            runtime.context(package_id, version, kind, entity_id, depth, node_limit)
        })
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
    pub integrity_error: Option<String>,
}

/// Source-only authoring review. It is deliberately separate from the
/// installed-package learner DTOs and never enters Runtime or package schema.
#[derive(Debug, Serialize)]
pub struct SourceReviewView {
    pub source_directory: String,
    pub valid: bool,
    pub package_id: Option<String>,
    pub package_version: Option<String>,
    pub schema_version: Option<String>,
    pub title: Option<String>,
    pub references: Vec<Value>,
    pub diagnostics: Vec<crate::error::ErrorView>,
}

/// Validate a user-selected source directory and run Core lint on valid models.
/// Findings are returned as data, including hard validation errors, so the
/// authoring UI can display them without reimplementing validation semantics.
#[tauri::command]
pub fn review_source(source_directory: String) -> CommandResult<SourceReviewView> {
    let path = PathBuf::from(&source_directory);
    let loaded = match load_source(&path) {
        Ok(loaded) => loaded,
        Err(diagnostics) => {
            return Ok(SourceReviewView {
                source_directory,
                valid: false,
                package_id: None,
                package_version: None,
                schema_version: None,
                title: None,
                references: Vec::new(),
                diagnostics: diagnostics
                    .into_iter()
                    .map(crate::error::ErrorView::from)
                    .collect(),
            });
        }
    };

    let manifest = &loaded.model.documents().manifest;
    let diagnostics = osmium_core::lint::lint(&loaded.model)
        .into_iter()
        .map(crate::error::ErrorView::from)
        .collect();
    let references = osmium_core::reference::registry(manifest)
        .cloned()
        .unwrap_or_default();
    Ok(SourceReviewView {
        source_directory,
        valid: true,
        package_id: manifest["package_id"].as_str().map(str::to_owned),
        package_version: manifest["package_version"].as_str().map(str::to_owned),
        schema_version: manifest["schema_version"].as_str().map(str::to_owned),
        title: manifest["title"].as_str().map(str::to_owned),
        references,
        diagnostics,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceRequest {
    pub directory: String,
    pub title: String,
    pub language: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSourceRequest {
    pub source_directory: String,
    pub title: String,
    pub language: String,
    pub resource_title: String,
    pub concept_title: String,
    pub markdown: String,
}

#[derive(Debug, Serialize)]
pub struct SourceEditorView {
    pub source_directory: String,
    pub package_id: String,
    pub editable: bool,
    pub title: String,
    pub description: Option<String>,
    pub language: String,
    pub resource_title: String,
    pub markdown: String,
    pub concept_title: String,
}

#[tauri::command]
pub fn open_authoring_workspace(
    source_directory: String,
) -> CommandResult<osmium_package::authoring::workspace::AuthoringWorkspace> {
    osmium_package::authoring::workspace::open_workspace(&source_directory)
        .map_err(CommandError::new)
}

#[tauri::command]
pub fn save_authoring_workspace(
    edits: osmium_package::authoring::workspace::WorkspaceEdits,
) -> CommandResult<osmium_package::authoring::workspace::AuthoringWorkspace> {
    osmium_package::authoring::workspace::save_workspace(edits).map_err(CommandError::new)
}

fn source_error(code: &str, message: impl ToString, file: Option<String>) -> CommandError {
    CommandError::new(vec![osmium_core::schema::Diagnostic {
        code: code.to_owned(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file,
        line: None,
        column: None,
        path: String::new(),
        message: message.to_string(),
        suggestions: Vec::new(),
    }])
}

#[tauri::command]
pub fn default_source_directory(app: tauri::AppHandle) -> CommandResult<String> {
    app.path()
        .document_dir()
        .map(|directory| {
            directory
                .join("Osmium")
                .join("Courses")
                .to_string_lossy()
                .into_owned()
        })
        .map_err(|error| source_error("OSM_PATH", error, None))
}

#[tauri::command]
pub fn create_source(request: CreateSourceRequest) -> CommandResult<SourceEditorView> {
    let source_directory = osmium_package::init::init_source_with_generated_id(
        &request.directory,
        &request.title,
        &request.language,
    )
    .map_err(|error| {
        source_error(
            error.code,
            error.message,
            Some(error.path.to_string_lossy().into_owned()),
        )
    })?
    .to_string_lossy()
    .into_owned();
    open_source_editor(source_directory)
}

#[tauri::command]
pub fn open_source_editor(source_directory: String) -> CommandResult<SourceEditorView> {
    osmium_package::authoring::open_source(&source_directory)
        .map(|editor| SourceEditorView {
            source_directory: editor.source_directory,
            package_id: editor.package_id,
            editable: editor.editable,
            title: editor.title,
            description: editor.description,
            language: editor.language,
            resource_title: editor.resource_title,
            markdown: editor.markdown,
            concept_title: editor.concept_title,
        })
        .map_err(CommandError::new)
}

#[tauri::command]
pub fn save_source(request: SaveSourceRequest) -> CommandResult<SourceEditorView> {
    osmium_package::authoring::save_source(osmium_package::authoring::SourceEdits {
        source_directory: request.source_directory,
        title: request.title,
        language: request.language,
        resource_title: request.resource_title,
        concept_title: request.concept_title,
        markdown: request.markdown,
    })
    .map(|editor| SourceEditorView {
        source_directory: editor.source_directory,
        package_id: editor.package_id,
        editable: editor.editable,
        title: editor.title,
        description: editor.description,
        language: editor.language,
        resource_title: editor.resource_title,
        markdown: editor.markdown,
        concept_title: editor.concept_title,
    })
    .map_err(CommandError::new)
}

#[tauri::command]
pub fn install_source(
    source_directory: String,
    desktop: State<'_, Desktop>,
) -> CommandResult<InstallReportView> {
    let root = PathBuf::from(&source_directory);
    desktop.with(|runtime| install_source_in_runtime(runtime, &root))
}

#[tauri::command]
pub fn install_distribution(
    distribution_path: String,
    desktop: State<'_, Desktop>,
) -> CommandResult<InstallReportView> {
    desktop
        .with(|runtime| runtime.install(PathBuf::from(distribution_path).as_path()))
        .map(|report| InstallReportView {
            package_id: report.package.package_id,
            package_version: report.package.package_version,
        })
}

#[tauri::command]
pub fn export_source(
    source_directory: String,
    destination: String,
) -> CommandResult<osmium_package::distribution::BuildReport> {
    osmium_package::distribution::build(
        PathBuf::from(source_directory).as_path(),
        PathBuf::from(destination).as_path(),
    )
    .map_err(CommandError::new)
}

#[tauri::command]
pub fn uninstall_package(
    package_id: String,
    package_version: String,
    desktop: State<'_, Desktop>,
) -> CommandResult<osmium_package::library::UninstallReport> {
    desktop.with(|runtime| runtime.uninstall(&package_id, &package_version))
}

fn install_source_in_runtime(
    runtime: &mut Runtime,
    root: &std::path::Path,
) -> Result<InstallReportView, Vec<osmium_core::schema::Diagnostic>> {
    let loaded = load_source(root)?;
    let lint = osmium_core::lint::lint(&loaded.model);
    if lint.iter().any(|diagnostic| diagnostic.severity == "error") {
        return Err(lint);
    }
    let archive = std::env::temp_dir().join(format!("osmium-{}.zip", uuid::Uuid::new_v4()));
    let result = (|| {
        osmium_package::distribution::build(root, &archive)?;
        runtime.install(&archive)
    })();
    let _ = std::fs::remove_file(&archive);
    result.map(|report| InstallReportView {
        package_id: report.package.package_id,
        package_version: report.package.package_version,
    })
}

#[derive(Debug, Serialize)]
pub struct InstallReportView {
    pub package_id: String,
    pub package_version: String,
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
    pub references: Vec<ReferenceView>,
}

/// A reference selected for this Resource, with visibility already applied by
/// the shared Runtime before it crosses the Desktop IPC boundary.
#[derive(Debug, Serialize)]
pub struct ReferenceView {
    pub id: String,
    pub title: String,
    pub visibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator_visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Value>,
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

fn resource_reference_views(references: &Value) -> Vec<ReferenceView> {
    references
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|source| {
            let visibility = source["visibility"].as_str()?;
            let resolved = osmium_core::reference::visibility(source);
            if !resolved.is_record_public() {
                return None;
            }
            Some(ReferenceView {
                id: object_string(source, "id"),
                title: object_string(source, "title"),
                visibility: visibility.to_owned(),
                record_visibility: source["record_visibility"].as_str().map(str::to_owned),
                locator_visibility: source["locator_visibility"].as_str().map(str::to_owned),
                citation: source["citation"].as_str().map(str::to_owned),
                // attribution_only records never carry a locator to UI.
                locator: resolved
                    .is_locator_public()
                    .then(|| source["locator"].as_str().map(str::to_owned))
                    .flatten(),
                reference_type: source["type"].as_str().map(str::to_owned),
                publisher: source["publisher"].as_str().map(str::to_owned),
                authors: source["authors"].as_array().map(|authors| {
                    authors
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                }),
                published_at: source["published_at"].as_str().map(str::to_owned),
                updated_at: source["updated_at"].as_str().map(str::to_owned),
                accessed_at: source["accessed_at"].as_str().map(str::to_owned),
                version: source["version"].as_str().map(str::to_owned),
                edition: source["edition"].as_str().map(str::to_owned),
                identifiers: source.get("identifiers").cloned(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn status(desktop: State<'_, Desktop>) -> CommandResult<StatusView> {
    let packages = desktop.with(|runtime| runtime.package_inventory())?;
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
    let packages = desktop.with(|runtime| runtime.package_inventory())?;
    Ok(packages
        .into_iter()
        .map(|item| PackageView {
            selected_version: item.package.package_version.clone(),
            package_id: item.package.package_id,
            package_version: item.package.package_version,
            schema_version: item.package.schema_version,
            title: item.package.title,
            entity_counts: item.package.entity_counts,
            digest: item.package.digest,
            integrity_error: item.integrity_error,
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
        references: resource_reference_views(&view["references"]),
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

/// Read a bounded Package-local relationship neighborhood for Atlas/Route
/// clients. Entity and relation meaning stays in Core; the shell only selects
/// an installed version and forwards the caller's bounds.
#[tauri::command]
pub fn package_context(
    desktop: State<'_, Desktop>,
    package_id: String,
    version: Option<String>,
    kind: String,
    entity_id: String,
    depth: Option<usize>,
    node_limit: Option<usize>,
) -> CommandResult<osmium_core::query::ContextView> {
    let version = desktop.resolve_version(&package_id, version)?;
    desktop.package_context(
        &package_id,
        version.as_deref(),
        &kind,
        &entity_id,
        depth.unwrap_or(2),
        node_limit.unwrap_or(64),
    )
}

/// Search Concept titles in one installed Package through the Core read model.
/// The result cap keeps the renderer from building a Package-wide index.
#[tauri::command]
pub fn package_concept_search(
    desktop: State<'_, Desktop>,
    package_id: String,
    version: Option<String>,
    query: String,
    limit: Option<usize>,
) -> CommandResult<osmium_core::query::ConceptSearchView> {
    let version = desktop.resolve_version(&package_id, version)?;
    desktop.with(|runtime| {
        runtime.search_concepts(&package_id, version.as_deref(), &query, limit.unwrap_or(20))
    })
}

/// Read a bounded newest-first history page across all Packages, including
/// events whose Package payload is no longer installed.
#[tauri::command]
pub fn all_history(
    desktop: State<'_, Desktop>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> CommandResult<Vec<Value>> {
    let limit = limit.unwrap_or(32).clamp(1, 64);
    let offset = offset.unwrap_or(0);
    desktop.with(|runtime| runtime.history_all(limit, offset))
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
    use super::{ContentView, resource_reference_views};

    #[test]
    fn desktop_content_dto_contains_ir_and_derived_text_but_not_markdown_source() {
        let view = ContentView::compile("# Title\n\nA **typed** paragraph.").expect("content");
        let value = serde_json::to_value(view).expect("serializable DTO");
        assert_eq!(value["text"], "Title\nA typed paragraph.");
        assert!(value["content"]["blocks"].is_array());
        assert!(value.get("markdown").is_none());
    }

    #[test]
    fn resource_reference_dto_preserves_metadata_and_applies_visibility() {
        let raw = serde_json::json!([
            {"id":"public","title":"公開資料","visibility":"public","record_visibility":"public","locator_visibility":"public","type":"article","authors":["A. Author"],"publisher":"Pub","citation":"機関. 資料名。","locator":"https://example.org/public"},
            {"id":"credit","title":"謝辞のみ","visibility":"attribution_only","record_visibility":"public","locator_visibility":"hidden","citation":"発行元. 指針名。","locator":"https://example.org/private-locator"},
            {"id":"private","title":"LEAK_SENTINEL","visibility":"private","citation":"private citation","locator":"C:/private/file.pdf"}
        ]);
        let projected =
            serde_json::to_value(resource_reference_views(&raw)).expect("reference DTOs");
        assert_eq!(projected.as_array().unwrap().len(), 2);
        assert_eq!(projected[0]["locator"], "https://example.org/public");
        assert_eq!(projected[0]["type"], "article");
        assert_eq!(projected[0]["authors"][0], "A. Author");
        assert_eq!(projected[0]["publisher"], "Pub");
        assert_eq!(projected[1]["citation"], "発行元. 指針名。");
        assert!(projected[1].get("locator").is_none());
        assert!(!projected.to_string().contains("LEAK_SENTINEL"));
        assert!(!projected.to_string().contains("private-locator"));

        let contradictory = serde_json::json!([
            {"id":"contradictory","title":"must be private","visibility":"public","record_visibility":"private","locator_visibility":"public","locator":"https://example.org/private"}
        ]);
        assert!(resource_reference_views(&contradictory).is_empty());
    }
}

#[cfg(test)]
mod package_context_tests {
    use super::Desktop;
    use osmium_package::distribution::build;
    use std::path::PathBuf;

    #[test]
    fn desktop_adapter_returns_the_core_bounded_concept_neighborhood() {
        let temp = tempfile::tempdir().expect("temporary data root");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../examples/arithmetic-expanded")
            .canonicalize()
            .expect("Atlas package fixture exists");
        let archive = temp.path().join("arithmetic.osmium");
        build(&source, &archive).expect("fixture builds");
        let desktop = Desktop::open(temp.path().join("home")).expect("desktop session opens");
        desktop
            .with(|runtime| runtime.install(&archive).map(|_| ()))
            .expect("fixture installs through Runtime");

        let view = desktop
            .package_context(
                "org.example/arithmetic-expanded",
                Some("0.1.0"),
                "concept",
                "addition",
                2,
                64,
            )
            .expect("Desktop forwards the bounded context query to Core");
        let ids: std::collections::BTreeSet<&str> =
            view.nodes.iter().map(|node| node.id.as_str()).collect();
        assert!(ids.contains("addition"));
        assert!(ids.contains("counting"));
        assert!(ids.contains("addition.basic"));
        assert!(ids.contains("addition.lesson"));
        assert!(ids.contains("addition.check"));
        assert!(ids.contains("arithmetic.path"));
        assert!(!view.truncated);
        assert!(view.content_is_untrusted);
    }
}

#[cfg(test)]
mod source_review_tests {
    use super::{
        CreateSourceRequest, SaveSourceRequest, create_source, install_source_in_runtime,
        open_source_editor, review_source, save_source,
    };
    use osmium_store::runtime::Runtime;
    use serde_json::{Value, json};
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn valid_source_review_returns_metadata_and_no_diagnostics() {
        let review = review_source(fixture("authoring-valid")).expect("source review");
        assert!(review.valid);
        assert_eq!(
            review.package_id.as_deref(),
            Some("org.example/authoring-review")
        );
        assert_eq!(review.package_version.as_deref(), Some("1.0.0"));
        assert_eq!(review.schema_version.as_deref(), Some("0.1"));
        assert_eq!(review.references.len(), 1);
        assert!(review.diagnostics.is_empty());
    }

    #[test]
    fn broken_source_review_preserves_core_diagnostic_fields() {
        let review = review_source(fixture("authoring-broken")).expect("source review");
        assert!(!review.valid);
        let diagnostic = review
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "OSM_REFERENCE")
            .expect("the Core reference diagnostic is projected");
        assert_eq!(diagnostic.severity, "error");
        assert_eq!(diagnostic.file.as_deref(), Some("entities/objectives.json"));
        assert_eq!(diagnostic.path, "/0/concept");
        assert_eq!(diagnostic.line, None);
        assert_eq!(diagnostic.column, None);
        assert!(diagnostic.suggestions.is_empty());
        let payload = serde_json::to_value(diagnostic).expect("IPC DTO serializes");
        assert_eq!(payload["severity"], "error");
        assert_eq!(payload["file"], "entities/objectives.json");
        assert_eq!(payload["path"], "/0/concept");
    }

    #[test]
    fn desktop_create_edit_save_and_review_use_the_package_loader() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let parent = temp.path().join("courses");
        let created = create_source(CreateSourceRequest {
            directory: parent.to_string_lossy().into_owned(),
            title: "Created Course".into(),
            language: "en".into(),
        })
        .expect("source scaffold created");
        assert_eq!(created.title, "Created Course");
        assert!(
            created
                .package_id
                .starts_with("org.osmium.generated/created-course-")
        );
        let directory = PathBuf::from(&created.source_directory);
        assert!(directory.ends_with("created-course"));
        let manifest_path = directory.join("osmium.json");
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        manifest["extensions"]["org.example.editor.v1"] = json!({"preserve": true});
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let saved = save_source(SaveSourceRequest {
            source_directory: created.source_directory,
            title: "Edited Course".into(),
            language: "en".into(),
            resource_title: "First lesson".into(),
            markdown: "# First lesson\n\nSaved body.".into(),
            concept_title: "Edited concept".into(),
        })
        .expect("source saved");
        assert_eq!(saved.title, "Edited Course");
        assert!(saved.markdown.contains("Saved body"));
        let reopened =
            open_source_editor(saved.source_directory.clone()).expect("re-load saved source");
        assert_eq!(reopened.resource_title, "First lesson");
        assert_eq!(reopened.concept_title, "Edited concept");
        let saved_manifest: Value =
            serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
        assert_eq!(
            saved_manifest["extensions"]["org.example.editor.v1"]["preserve"],
            true
        );
        let reviewed = review_source(saved.source_directory).expect("Core review");
        assert!(reviewed.valid);
    }

    #[test]
    fn install_builds_then_uses_the_shared_runtime_and_installed_library() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let created = create_source(CreateSourceRequest {
            directory: temp.path().join("courses").to_string_lossy().into_owned(),
            title: "GUI Install".into(),
            language: "en".into(),
        })
        .expect("source scaffold");
        let source = PathBuf::from(&created.source_directory);
        let mut runtime = Runtime::open(&temp.path().join("library")).expect("shared runtime");
        let report = install_source_in_runtime(&mut runtime, &source).expect("build and install");
        assert_eq!(report.package_id, created.package_id);
        let packages = runtime.packages().expect("reloaded library");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].title, "GUI Install");
        let lesson = runtime
            .lesson(&report.package_id, Some(&report.package_version))
            .expect("installed learner package");
        assert_eq!(lesson["manifest"]["title"], "GUI Install");
    }

    #[test]
    fn an_invalid_source_cannot_be_installed_for_preview() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let broken =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/authoring-broken");
        let mut runtime = Runtime::open(&temp.path().join("library")).expect("shared runtime");
        let error = install_source_in_runtime(&mut runtime, &broken)
            .expect_err("invalid references block install");
        assert!(
            error
                .iter()
                .any(|diagnostic| diagnostic.code == "OSM_REFERENCE")
        );
        assert!(
            runtime
                .packages()
                .expect("library remains available")
                .is_empty()
        );
    }
}
