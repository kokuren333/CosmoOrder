//! One function per command. Each maps CLI arguments onto Core or Package
//! operations and returns a serializable view; none re-implements validation.

use crate::args::{Cli, Command};
use crate::error::{Failure, target_diagnostic};
use osmium_core::query::{self, EntityKind};
use osmium_core::schema::Diagnostic;
use osmium_core::validation::PackageModel;
use osmium_package::init::{self, InitRequest};
use osmium_package::load_source;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Result of `validate`: identity plus entity counts of a valid package.
#[derive(Debug, Serialize)]
struct ValidateView {
    schema_version: String,
    package_id: String,
    package_version: String,
    total_entities: usize,
    files_read: usize,
}

/// Result of `init`.
#[derive(Debug, Serialize)]
struct InitView {
    package_id: String,
    package_version: String,
    schema_version: String,
    language: String,
    title: String,
    directory: String,
    created_files: Vec<String>,
}

pub fn execute(cli: &Cli) -> Result<(serde_json::Value, Vec<Diagnostic>), Failure> {
    let value = match &cli.command {
        Command::Install { path } => {
            let mut library = runtime(cli)?;
            serde_json::to_value(library.install(path).map_err(Failure::from_diagnostics)?)
                .map_err(internal_serialization)?
        }
        Command::Packages => {
            let library = runtime(cli)?;
            serde_json::to_value(library.packages().map_err(Failure::from_diagnostics)?)
                .map_err(internal_serialization)?
        }
        Command::Learn {
            package_id,
            package_version,
        } => runtime(cli)?
            .lesson(package_id, package_version.as_deref())
            .map_err(Failure::from_diagnostics)?,
        Command::Read {
            package_id,
            resource_id,
            package_version,
        } => runtime(cli)?
            .resource(package_id, package_version.as_deref(), resource_id)
            .map_err(Failure::from_diagnostics)?,
        Command::Answer {
            package_id,
            assessment_id,
            response,
            package_version,
            request_id,
            duration_ms,
            hints_used,
        } => {
            let response = osmium_core::parsing::parse_json(response.as_bytes(), "response")
                .map_err(|e| Failure::new(vec![*e], crate::envelope::Exit::Usage))?;
            let request = osmium_store::AttemptRequest {
                assessment_id: assessment_id.clone(),
                response,
                request_id: request_id
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                duration_ms: *duration_ms,
                hints_used: *hints_used,
            };
            serde_json::to_value(
                runtime(cli)?
                    .answer(package_id, package_version.as_deref(), &request)
                    .map_err(Failure::from_diagnostics)?,
            )
            .map_err(internal_serialization)?
        }
        Command::Progress {
            package_id,
            package_version,
        } => serde_json::to_value(
            runtime(cli)?
                .progress(package_id, package_version.as_deref())
                .map_err(Failure::from_diagnostics)?,
        )
        .map_err(internal_serialization)?,
        Command::History {
            package_id,
            package_version,
            limit,
            offset,
        } => serde_json::to_value(
            runtime(cli)?
                .history(package_id, package_version.as_deref(), *limit, *offset)
                .map_err(Failure::from_diagnostics)?,
        )
        .map_err(internal_serialization)?,
        Command::RebuildProgress => {
            serde_json::json!({"events_replayed":runtime(cli)?.rebuild_progress().map_err(Failure::from_diagnostics)?})
        }
        Command::ExportState { output } => {
            serde_json::json!({"events_exported":runtime(cli)?.export_state(output).map_err(Failure::from_diagnostics)?, "output":output})
        }
        Command::BackupState { output } => {
            runtime(cli)?
                .backup(output)
                .map_err(Failure::from_diagnostics)?;
            serde_json::json!({"output":output})
        }
        Command::Build {
            source,
            destination,
        } => serde_json::to_value(
            osmium_package::distribution::build(source, destination)
                .map_err(Failure::from_diagnostics)?,
        )
        .map_err(internal_serialization)?,
        Command::Lint { path, .. } => {
            let loaded = load(path)?;
            let mut diagnostics = osmium_core::lint::lint(&loaded.model);
            let docs = loaded.model.documents();
            for resource in docs.resources.as_array().unwrap() {
                let id = resource["id"].as_str().unwrap_or_default();
                if let Some(body) = resource["path"]
                    .as_str()
                    .and_then(|path| loaded.files.get(path))
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                {
                    if body.trim().chars().count() < 240 {
                        diagnostics.push(Diagnostic { code:"OSM_LINT_RESOURCE_SHORT".into(), severity:"warning".into(), entity_type:Some("resource".into()), entity_id:Some(id.into()), file:resource["path"].as_str().map(str::to_owned), line:None, column:None, path:"/".into(), message:"resource body is brief; review whether it teaches the concept without additional material".into(), suggestions:vec!["consider explanation, example, distinction, or summary where useful".into()] });
                    }
                }
            }
            diagnostics.sort_by(|a, b| {
                (&a.file, &a.entity_id, &a.code, &a.message).cmp(&(
                    &b.file,
                    &b.entity_id,
                    &b.code,
                    &b.message,
                ))
            });
            return Ok((
                serde_json::json!({"finding_count": diagnostics.len(), "content_is_untrusted": true}),
                diagnostics,
            ));
        }
        Command::Init {
            directory,
            package_id,
            language,
            ..
        } => serde_json::to_value(init_command(
            directory,
            package_id.as_deref(),
            language.as_deref(),
        )?)
        .map_err(internal_serialization)?,
        Command::Validate { path, .. } => {
            let loaded = load(path)?;
            serde_json::to_value(ValidateView {
                schema_version: text(&loaded.model, "schema_version"),
                package_id: text(&loaded.model, "package_id"),
                package_version: text(&loaded.model, "package_version"),
                // Reuse the Core view so `validate` and `inspect` can never
                // disagree about how many entities a package holds.
                total_entities: query::inspect(&loaded.model, 0).total_entities,
                files_read: loaded.files.len(),
            })
            .map_err(internal_serialization)?
        }
        Command::Inspect { path, limit, .. } => {
            if *limit > query::MAX_PREREQUISITE_ORDER {
                return Err(Failure::usage(
                    "OSM_INSPECT_LIMIT",
                    "inspect limit exceeds 64",
                ));
            }
            let loaded = load(path)?;
            serde_json::to_value(query::inspect(&loaded.model, *limit))
                .map_err(internal_serialization)?
        }
        Command::Query {
            path,
            kind,
            limit,
            offset,
            ..
        } => {
            let loaded = load(path)?;
            let kind = parse_kind(kind)?;
            let view = query::query(&loaded.model, kind, *offset, *limit).map_err(option_value)?;
            serde_json::to_value(view).map_err(internal_serialization)?
        }
        Command::Context {
            path,
            entity_id,
            kind,
            depth,
            limit,
            ..
        } => {
            let loaded = load(path)?;
            let kind = match kind {
                Some(value) => parse_kind(value)?,
                None => {
                    query::resolve_target(&loaded.model, entity_id)
                        .map_err(Failure::from_diagnostics)?
                        .0
                }
            };
            let view = query::context(&loaded.model, kind, entity_id, *depth, *limit)
                .map_err(option_value)?;
            serde_json::to_value(view).map_err(internal_serialization)?
        }
    };
    Ok((value, Vec::new()))
}

/// Diagnostic codes that describe a rejected option value rather than a
/// package defect. They are invocation mistakes, so they exit 2.
const OPTION_VALUE_CODES: [&str; 3] = ["OSM_QUERY_LIMIT", "OSM_CONTEXT_DEPTH", "OSM_CONTEXT_LIMIT"];

fn option_value(diagnostics: Vec<Diagnostic>) -> Failure {
    if diagnostics
        .iter()
        .all(|diagnostic| OPTION_VALUE_CODES.contains(&diagnostic.code.as_str()))
    {
        // Keep the specific codes so a caller can tell the options apart, but
        // report the invocation mistake rather than an invalid package.
        Failure::new(diagnostics, crate::envelope::Exit::Usage)
    } else {
        Failure::from_diagnostics(diagnostics)
    }
}

fn internal_serialization(error: serde_json::Error) -> Failure {
    Failure::internal(
        "OSM_OUTPUT",
        format!("could not serialize the result: {error}"),
    )
}

/// Load and fully validate Source, distribution directory, or ZIP.
/// A missing input path is classified as an invocation mistake.
fn load(path: &Path) -> Result<osmium_package::LoadedSource, Failure> {
    let meta = std::fs::symlink_metadata(path).map_err(|error| {
        Failure::new(
            vec![target_diagnostic(
                path,
                &format!("cannot read the package path: {error}"),
            )],
            crate::envelope::Exit::Usage,
        )
    })?;
    if meta.file_type().is_symlink() {
        return Err(Failure::new(
            vec![target_diagnostic(
                path,
                "the package path must not be a symbolic link",
            )],
            crate::envelope::Exit::Usage,
        ));
    }
    if meta.is_file() || path.join("manifest.json").exists() {
        let distribution = osmium_package::distribution::read_distribution(path)
            .map_err(Failure::from_diagnostics)?;
        Ok(osmium_package::LoadedSource {
            model: distribution.model,
            files: distribution.files,
        })
    } else {
        load_source(path).map_err(Failure::from_diagnostics)
    }
}

/// Reject a kind the CLI does not implement.
///
/// A bad option value is an invocation mistake, not an invalid package, so it
/// is classified as usage even though Core reports it as a diagnostic.
fn parse_kind(value: &str) -> Result<EntityKind, Failure> {
    EntityKind::parse(value).ok_or_else(|| {
        Failure::usage(
            "OSM_QUERY_KIND",
            format!(
                "unknown entity kind: {value}; expected one of {}",
                EntityKind::ALL
                    .iter()
                    .map(|kind| kind.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })
}

fn text(model: &PackageModel, field: &str) -> String {
    model.documents().manifest[field]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn runtime(cli: &Cli) -> Result<osmium_store::runtime::Runtime, Failure> {
    let home = match &cli.home {
        Some(path) => path.clone(),
        None => osmium_package::library::default_home().map_err(Failure::from_diagnostics)?,
    };
    osmium_store::runtime::Runtime::open(&home).map_err(Failure::from_diagnostics)
}

fn init_command(
    directory: &Path,
    package_id: Option<&str>,
    language: Option<&str>,
) -> Result<InitView, Failure> {
    let request = InitRequest {
        directory: directory.to_path_buf(),
        // An empty ID asks the package layer to derive one from the directory
        // name, so that rule lives in exactly one place.
        package_id: package_id.unwrap_or_default().to_owned(),
        language: language.unwrap_or(init::SCAFFOLD_LANGUAGE).to_owned(),
    };
    let created_files = init::init_source(&request).map_err(|error| {
        let diagnostic = Diagnostic {
            code: error.code.to_owned(),
            severity: "error".to_owned(),
            entity_type: None,
            entity_id: None,
            file: Some(error.path.to_string_lossy().into_owned()),
            line: None,
            column: None,
            path: String::new(),
            message: error.message,
            suggestions: Vec::new(),
        };
        // Invalid requests are usage failures; I/O and implementation errors
        // retain the shared internal-failure status.
        let exit = if matches!(
            error.code,
            "OSM_INIT_IO" | "OSM_INIT_INTERNAL" | "OSM_INIT_INVALID"
        ) {
            crate::envelope::Exit::Internal
        } else {
            crate::envelope::Exit::Usage
        };
        Failure::new(vec![diagnostic], exit)
    })?;
    let loaded = load(directory)?;
    Ok(InitView {
        package_id: text(&loaded.model, "package_id"),
        package_version: text(&loaded.model, "package_version"),
        schema_version: text(&loaded.model, "schema_version"),
        language: text(&loaded.model, "language"),
        title: text(&loaded.model, "title"),
        directory: PathBuf::from(directory).to_string_lossy().into_owned(),
        created_files,
    })
}
