//! Narrow authoring edits to Reference records and Evidence relations.
//!
//! These are the only operations that write to a Source. They exist because the
//! E2E authoring benchmark showed that hand-editing two separate JSON documents
//! for one reference was the most error-prone step: a Reference and the
//! Evidence relation that points at it must agree, and nothing checked that
//! agreement until the whole package was re-read.
//!
//! Everything else stays an ordinary JSON/YAML/Markdown edit. This module does
//! not implement general CRUD, does not fetch a locator, and does not decide
//! whether a reference is *good* evidence — that is review, not tooling.

use crate::load_source;
use osmium_core::parsing::parse_json;
use osmium_core::reference;
use osmium_core::schema::Diagnostic;
use serde::Serialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// What the caller wants registered. Every field is optional except the ID and
/// the kind, so a citation-only reference stays expressible.
#[derive(Debug, Clone, Default)]
pub struct ReferenceDraft {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub locator: Option<String>,
    pub citation: Option<String>,
    pub reference_type: Option<String>,
    pub publisher: Option<String>,
    pub authors: Vec<String>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub accessed_at: Option<String>,
    pub edition: Option<String>,
    pub version: Option<String>,
    /// `public`, `attribution_only` or `private`.
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceAdded {
    pub reference_id: String,
    /// Registry field actually used. `sources` means the package has not been
    /// migrated to the canonical name yet.
    pub registry: String,
    pub file: String,
    /// False when the ID was already registered and nothing was written.
    pub created: bool,
}

#[derive(Debug, Serialize)]
pub struct ReferenceAttached {
    pub reference_id: String,
    pub target_kind: String,
    pub target_id: String,
    /// Evidence field actually used. `source_ids` means the entity has not been
    /// migrated to the canonical name yet.
    pub evidence_field: String,
    pub file: String,
    pub changed: bool,
}

#[derive(Debug, Serialize)]
pub struct ReferenceList {
    pub registry: String,
    pub count: usize,
    pub references: Vec<Value>,
}

fn diagnostic(file: &str, path: &str, code: &str, message: impl Into<String>) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: code.into(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file: Some(file.into()),
        line: None,
        column: None,
        path: path.into(),
        message: message.into(),
        suggestions: Vec::new(),
    }]
}

/// The manifest file name, rejecting a Source that has none or two.
fn manifest_name(root: &Path) -> Result<&'static str, Vec<Diagnostic>> {
    let json = root.join("osmium.json").is_file();
    let yaml = root.join("osmium.yaml").is_file();
    match (json, yaml) {
        (true, false) => Ok("osmium.json"),
        (false, true) => Ok("osmium.yaml"),
        (true, true) => Err(diagnostic(
            "osmium.json",
            "",
            "OSM_MANIFEST",
            "source must contain exactly one manifest",
        )),
        (false, false) => Err(diagnostic(
            "osmium.json",
            "",
            "OSM_MISSING_FILE",
            "no osmium.json or osmium.yaml in this source",
        )),
    }
}

/// Parse a JSON authoring document that this module is about to rewrite.
fn read_json(root: &Path, name: &str) -> Result<Value, Vec<Diagnostic>> {
    let bytes = fs::read(root.join(name))
        .map_err(|error| diagnostic(name, "", "OSM_IO", error.to_string()))?;
    parse_json(&bytes, name).map_err(|error| vec![*error])
}

fn write_json(root: &Path, name: &str, value: &Value) -> Result<(), Vec<Diagnostic>> {
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|error| diagnostic(name, "", "OSM_INIT_INTERNAL", error.to_string()))?;
    text.push('\n');
    fs::write(root.join(name), text)
        .map_err(|error| diagnostic(name, "", "OSM_IO", error.to_string()))
}

fn normalize(draft: &str) -> Option<&'static str> {
    match draft {
        "public" => Some("public"),
        "attribution_only" => Some("attribution_only"),
        "private" => Some("private"),
        _ => None,
    }
}

/// Register one Reference in the manifest registry.
///
/// The registry is never renamed: a package that still uses `sources` keeps
/// using it so a half-migrated package cannot end up with two registries.
pub fn add_reference(
    root: &Path,
    draft: &ReferenceDraft,
) -> Result<ReferenceAdded, Vec<Diagnostic>> {
    let manifest_name = manifest_name(root)?;
    if manifest_name.ends_with(".yaml") {
        return Err(diagnostic(
            manifest_name,
            "",
            "OSM_INIT_INVALID",
            "reference add writes JSON manifests only; edit the YAML manifest directly",
        ));
    }
    if !osmium_core::schema::is_valid_id(&draft.id) {
        return Err(diagnostic(
            manifest_name,
            "/references",
            "OSM_INIT_INVALID",
            format!("invalid reference ID: {}", draft.id),
        ));
    }
    if draft.kind.is_empty() {
        return Err(diagnostic(
            manifest_name,
            "/references",
            "OSM_INIT_INVALID",
            "a reference requires a kind",
        ));
    }
    if let Some(visibility) = &draft.visibility
        && normalize(visibility).is_none()
    {
        return Err(diagnostic(
            manifest_name,
            "/references",
            "OSM_INIT_INVALID",
            format!("unknown visibility: {visibility}"),
        ));
    }
    if draft.reference_type.is_some()
        && draft.kind == "package_asset"
        && draft
            .locator
            .as_deref()
            .is_none_or(|locator| osmium_core::validation::validate_relative_path(locator).is_err())
    {
        return Err(diagnostic(
            manifest_name,
            "/references",
            "OSM_INIT_INVALID",
            "a package_asset reference requires a safe package-relative locator",
        ));
    }

    // Validate the whole package first so an edit never repairs a broken source.
    let loaded = load_source(root)?;
    let mut manifest = read_json(root, manifest_name)?;
    let registry = reference::registry_field(&manifest).unwrap_or(reference::REFERENCES);
    let entries = manifest
        .as_object_mut()
        .unwrap()
        .entry(registry)
        .or_insert_with(|| Value::Array(Vec::new()));
    let entries = entries.as_array_mut().ok_or_else(|| {
        diagnostic(
            manifest_name,
            &format!("/{registry}"),
            "OSM_INIT_INVALID",
            "the reference registry is not an array",
        )
    })?;
    if entries
        .iter()
        .any(|entry| entry["id"].as_str() == Some(draft.id.as_str()))
    {
        return Ok(ReferenceAdded {
            reference_id: draft.id.clone(),
            registry: registry.to_owned(),
            file: manifest_name.to_owned(),
            created: false,
        });
    }

    if draft.kind == "package_asset" {
        let locator = draft.locator.as_deref().ok_or_else(|| {
            diagnostic(
                manifest_name,
                "/references",
                "OSM_INIT_INVALID",
                "a package_asset reference requires a package-relative locator",
            )
        })?;
        if osmium_core::validation::validate_relative_path(locator).is_err() {
            return Err(diagnostic(
                manifest_name,
                "/references",
                "OSM_INIT_INVALID",
                "a package_asset reference requires a safe package-relative locator",
            ));
        }
        let root_path = root
            .canonicalize()
            .map_err(|error| diagnostic(manifest_name, "", "OSM_IO", error.to_string()))?;
        let canonical_asset = root_path.join(locator).canonicalize().map_err(|error| {
            diagnostic(
                manifest_name,
                "/references",
                "OSM_INIT_INVALID",
                error.to_string(),
            )
        })?;
        let relative = canonical_asset
            .strip_prefix(&root_path)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        if !canonical_asset.is_file() || relative.as_deref() != Some(locator) {
            return Err(diagnostic(
                manifest_name,
                "/references",
                "OSM_INIT_INVALID",
                "package_asset locator must name an existing file with its exact package-relative path",
            ));
        }
    }

    let mut record = Map::new();
    record.insert("id".into(), Value::String(draft.id.clone()));
    record.insert("kind".into(), Value::String(draft.kind.clone()));
    let visibility = normalize(draft.visibility.as_deref().unwrap_or("public")).unwrap();
    record.insert("visibility".into(), Value::String(visibility.into()));
    // The two axes are written alongside the compatibility enum so a new record
    // is explicit about both questions from the start.
    match visibility {
        "private" => {
            record.insert("record_visibility".into(), Value::String("private".into()));
        }
        "attribution_only" => {
            record.insert("record_visibility".into(), Value::String("public".into()));
            record.insert("locator_visibility".into(), Value::String("hidden".into()));
        }
        _ => {
            record.insert("record_visibility".into(), Value::String("public".into()));
            record.insert("locator_visibility".into(), Value::String("public".into()));
        }
    }
    for (key, value) in [
        ("title", draft.title.as_ref()),
        ("locator", draft.locator.as_ref()),
        ("citation", draft.citation.as_ref()),
        ("type", draft.reference_type.as_ref()),
        ("publisher", draft.publisher.as_ref()),
        ("published_at", draft.published_at.as_ref()),
        ("updated_at", draft.updated_at.as_ref()),
        ("accessed_at", draft.accessed_at.as_ref()),
        ("edition", draft.edition.as_ref()),
        ("version", draft.version.as_ref()),
    ] {
        if let Some(value) = value {
            if value.trim().is_empty() {
                return Err(diagnostic(
                    manifest_name,
                    &format!("/{registry}"),
                    "OSM_INIT_INVALID",
                    format!("{key} must not be blank"),
                ));
            }
            record.insert(key.into(), Value::String(value.clone()));
        }
    }
    if !draft.authors.is_empty() {
        record.insert(
            "authors".into(),
            Value::Array(
                draft
                    .authors
                    .iter()
                    .map(|author| Value::String(author.clone()))
                    .collect(),
            ),
        );
    }
    entries.push(Value::Object(record));

    // Validate the proposed manifest and graph in memory before touching the
    // user's file. Besides returning a normal usage diagnostic for a misspelled
    // `--kind` or `--type`, this protects against leaving an unloadable source
    // behind when the final loader pass rejects the new record.
    let documents = loaded.model.documents();
    if let Err(mut errors) =
        osmium_core::validation::validate_package(osmium_core::validation::PackageDocuments {
            manifest: manifest.clone(),
            concepts: documents.concepts.clone(),
            objectives: documents.objectives.clone(),
            curricula: documents.curricula.clone(),
            resources: documents.resources.clone(),
            assessments: documents.assessments.clone(),
        })
    {
        if errors.iter().all(|error| error.code == "OSM_SCHEMA") {
            for error in &mut errors {
                error.code = "OSM_INIT_INVALID".into();
                error.message = format!("invalid reference draft: {}", error.message);
            }
        }
        return Err(errors);
    }

    write_json(root, manifest_name, &manifest)?;
    // The edit must produce a Source that loads with the same loader a
    // third-party package uses; otherwise it is rolled back by the caller's
    // version control, not by a half-written file.
    load_source(root)?;
    Ok(ReferenceAdded {
        reference_id: draft.id.clone(),
        registry: registry.to_owned(),
        file: manifest_name.to_owned(),
        created: true,
    })
}

/// Attach an existing Reference to a Resource or an Assessment.
pub fn attach_reference(
    root: &Path,
    reference_id: &str,
    resource_id: Option<&str>,
    assessment_id: Option<&str>,
) -> Result<ReferenceAttached, Vec<Diagnostic>> {
    let (target_kind, target_id, entity_file) = match (resource_id, assessment_id) {
        (Some(_), Some(_)) => {
            return Err(diagnostic(
                "osmium.json",
                "",
                "OSM_INIT_INVALID",
                "attach one reference to either a resource or an assessment, not both",
            ));
        }
        (Some(resource), None) => ("resource", resource, "resources"),
        (None, Some(assessment)) => ("assessment", assessment, "assessments"),
        (None, None) => {
            return Err(diagnostic(
                "osmium.json",
                "",
                "OSM_INIT_INVALID",
                "attach requires --resource or --assessment",
            ));
        }
    };

    let manifest_name = manifest_name(root)?;
    let loaded = load_source(root)?;
    let manifest = loaded.model.documents().manifest.clone();
    if reference::registry(&manifest).is_none() {
        return Err(diagnostic(
            manifest_name,
            "",
            "OSM_SOURCE_REFERENCE",
            "this package has no reference registry; register a reference first",
        ));
    }
    if !reference::registry(&manifest)
        .unwrap()
        .iter()
        .any(|record| record["id"].as_str() == Some(reference_id))
    {
        return Err(diagnostic(
            manifest_name,
            "",
            "OSM_SOURCE_REFERENCE",
            format!("unknown reference ID: {reference_id}"),
        ));
    }

    let path = loaded.model.documents().manifest["entities"][entity_file]
        .as_str()
        .ok_or_else(|| {
            diagnostic(
                manifest_name,
                "",
                "OSM_INIT_INVALID",
                format!("manifest declares no {entity_file} document"),
            )
        })?
        .to_owned();
    if !path.ends_with(".json") {
        return Err(diagnostic(
            &path,
            "",
            "OSM_INIT_INVALID",
            "reference attach writes JSON entity documents only",
        ));
    }
    let mut entities = read_json(root, &path)?;
    let entries = entities.as_array_mut().ok_or_else(|| {
        diagnostic(
            &path,
            "",
            "OSM_INIT_INVALID",
            "entity document is not an array",
        )
    })?;
    let entity = entries
        .iter_mut()
        .find(|entity| entity["id"].as_str() == Some(target_id))
        .ok_or_else(|| {
            diagnostic(
                &path,
                "",
                "OSM_INIT_INVALID",
                format!("unknown {target_kind} ID: {target_id}"),
            )
        })?;
    let field = reference::evidence_field_for_write(entity);
    let entity = entity
        .as_object_mut()
        .ok_or_else(|| diagnostic(&path, "", "OSM_INIT_INVALID", "entity is not an object"))?;
    let ids = entity
        .entry(field)
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| {
            diagnostic(
                &path,
                &format!("/{field}"),
                "OSM_INIT_INVALID",
                "the evidence field is not an array",
            )
        })?;
    if ids.iter().any(|id| id.as_str() == Some(reference_id)) {
        return Ok(ReferenceAttached {
            reference_id: reference_id.to_owned(),
            target_kind: target_kind.into(),
            target_id: target_id.to_owned(),
            evidence_field: field.into(),
            file: path,
            changed: false,
        });
    }
    ids.push(Value::String(reference_id.to_owned()));
    write_json(root, &path, &entities)?;
    load_source(root)?;
    Ok(ReferenceAttached {
        reference_id: reference_id.to_owned(),
        target_kind: target_kind.into(),
        target_id: target_id.to_owned(),
        evidence_field: field.into(),
        file: path,
        changed: true,
    })
}

/// Read the registry without writing anything.
pub fn list_references(root: &Path) -> Result<ReferenceList, Vec<Diagnostic>> {
    let loaded = load_source(root)?;
    let manifest = &loaded.model.documents().manifest;
    let registry = reference::registry_field(manifest)
        .unwrap_or(reference::REFERENCES)
        .to_owned();
    let references = reference::registry(manifest).cloned().unwrap_or_default();
    Ok(ReferenceList {
        registry,
        count: references.len(),
        references,
    })
}

/// The canonical file a Source keeps its manifest in, for diagnostics.
pub fn manifest_path(root: &Path) -> PathBuf {
    root.join(manifest_name(root).unwrap_or("osmium.json"))
}
