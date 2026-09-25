//! Round-trip-preserving edits for the small Desktop authoring surface.
//! Unknown JSON properties are retained by mutating the parsed documents.

pub mod workspace;

use crate::load_source;
use osmium_core::schema::Diagnostic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEditor {
    pub source_directory: String,
    pub package_id: String,
    /// JSON manifests can be round-tripped by the package authoring API today;
    /// YAML remains loadable/reviewable until Core owns a YAML serializer.
    pub editable: bool,
    pub title: String,
    pub description: Option<String>,
    pub language: String,
    pub resource_title: String,
    pub markdown: String,
    pub concept_title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceEdits {
    pub source_directory: String,
    pub title: String,
    pub language: String,
    pub resource_title: String,
    pub concept_title: String,
    pub markdown: String,
}

fn diagnostic(code: &str, message: impl ToString, file: Option<String>) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: code.into(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file,
        line: None,
        column: None,
        path: String::new(),
        message: message.to_string(),
        suggestions: Vec::new(),
    }]
}

pub fn open_source(path: impl AsRef<Path>) -> Result<SourceEditor, Vec<Diagnostic>> {
    let root = path.as_ref();
    let loaded = load_source(root)?;
    let editable = root.join("osmium.json").is_file();
    let manifest = &loaded.model.documents().manifest;
    let resources = loaded
        .model
        .documents()
        .resources
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| {
            diagnostic(
                "OSM_EDITOR_RESOURCE",
                "source has no editable Markdown resource",
                None,
            )
        })?;
    let concepts = loaded
        .model
        .documents()
        .concepts
        .as_array()
        .and_then(|items| items.first());
    let resource_path = resources["path"].as_str().unwrap_or_default();
    let markdown = loaded.files.get(resource_path).ok_or_else(|| {
        diagnostic(
            "OSM_EDITOR_RESOURCE",
            "resource file is unavailable",
            Some(resource_path.into()),
        )
    })?;
    let markdown = String::from_utf8(markdown.clone()).map_err(|_| {
        diagnostic(
            "OSM_UTF8",
            "resource is not UTF-8",
            Some(resource_path.into()),
        )
    })?;
    Ok(SourceEditor {
        source_directory: root.to_string_lossy().into_owned(),
        package_id: manifest["package_id"].as_str().unwrap_or_default().into(),
        editable,
        title: manifest["title"].as_str().unwrap_or_default().into(),
        description: manifest["description"].as_str().map(str::to_owned),
        language: manifest["language"].as_str().unwrap_or("en").into(),
        resource_title: resources["title"].as_str().unwrap_or_default().into(),
        markdown,
        concept_title: concepts
            .and_then(|item| item["title"].as_str())
            .unwrap_or_default()
            .into(),
    })
}

pub fn save_source(edits: SourceEdits) -> Result<SourceEditor, Vec<Diagnostic>> {
    let root = Path::new(&edits.source_directory);
    let loaded = load_source(root)?;
    let root = root
        .canonicalize()
        .map_err(|error| diagnostic("OSM_IO", error, Some(edits.source_directory.clone())))?;
    let manifest = &loaded.model.documents().manifest;
    let manifest_name = if root.join("osmium.json").is_file() {
        "osmium.json"
    } else {
        return Err(diagnostic(
            "OSM_EDITOR_FORMAT",
            "editing YAML manifests is unavailable because no Core YAML serializer exists",
            Some("osmium.yaml".into()),
        ));
    };
    let manifest_path = root.join(manifest_name);
    let mut manifest_value: Value =
        serde_json::from_slice(loaded.files.get(manifest_name).ok_or_else(|| {
            diagnostic(
                "OSM_IO",
                "manifest bytes unavailable",
                Some(manifest_name.into()),
            )
        })?)
        .map_err(|error| diagnostic("OSM_JSON", error, Some(manifest_name.into())))?;
    manifest_value["title"] = Value::String(edits.title);
    manifest_value["language"] = Value::String(edits.language);
    let entity_path = manifest["entities"]["resources"]
        .as_str()
        .unwrap_or("entities/resources.json");
    let mut resources: Value =
        serde_json::from_slice(loaded.files.get(entity_path).ok_or_else(|| {
            diagnostic(
                "OSM_IO",
                "resource entity document unavailable",
                Some(entity_path.into()),
            )
        })?)
        .map_err(|error| diagnostic("OSM_JSON", error, Some(entity_path.into())))?;
    let resource = resources
        .as_array_mut()
        .and_then(|items| items.first_mut())
        .ok_or_else(|| {
            diagnostic(
                "OSM_EDITOR_RESOURCE",
                "source has no editable Markdown resource",
                Some(entity_path.into()),
            )
        })?;
    resource["title"] = Value::String(edits.resource_title);
    let concept_path = manifest["entities"]["concepts"]
        .as_str()
        .unwrap_or("entities/concepts.json");
    let mut concepts: Value =
        serde_json::from_slice(loaded.files.get(concept_path).ok_or_else(|| {
            diagnostic(
                "OSM_IO",
                "concept entity document unavailable",
                Some(concept_path.into()),
            )
        })?)
        .map_err(|error| diagnostic("OSM_JSON", error, Some(concept_path.into())))?;
    let concept = concepts
        .as_array_mut()
        .and_then(|items| items.first_mut())
        .ok_or_else(|| {
            diagnostic(
                "OSM_EDITOR_CONCEPT",
                "source has no editable concept",
                Some(concept_path.into()),
            )
        })?;
    concept["title"] = Value::String(edits.concept_title);
    let resource_path = resource["path"].as_str().unwrap_or_default().to_owned();
    let writes = [
        (
            manifest_path,
            serde_json::to_vec_pretty(&manifest_value)
                .map_err(|error| diagnostic("OSM_JSON", error, Some(manifest_name.into())))?,
        ),
        (
            root.join(entity_path),
            serde_json::to_vec_pretty(&resources)
                .map_err(|error| diagnostic("OSM_JSON", error, Some(entity_path.into())))?,
        ),
        (
            root.join(concept_path),
            serde_json::to_vec_pretty(&concepts)
                .map_err(|error| diagnostic("OSM_JSON", error, Some(concept_path.into())))?,
        ),
        (root.join(&resource_path), edits.markdown.into_bytes()),
    ];
    for (path, bytes) in writes {
        if !path.starts_with(&root) {
            return Err(diagnostic(
                "OSM_PATH",
                "edit target escapes source root",
                Some(path.to_string_lossy().into_owned()),
            ));
        }
        let resolved_parent = path.parent().and_then(|parent| parent.canonicalize().ok());
        if resolved_parent
            .as_deref()
            .is_none_or(|parent| !parent.starts_with(&root))
        {
            return Err(diagnostic(
                "OSM_LINK",
                "edit target parent is missing or outside source root",
                Some(path.to_string_lossy().into_owned()),
            ));
        }
        std::fs::write(&path, bytes).map_err(|error| {
            diagnostic("OSM_IO", error, Some(path.to_string_lossy().into_owned()))
        })?;
    }
    open_source(root)
}

#[cfg(test)]
mod tests {
    use super::{SourceEdits, open_source, save_source};
    use crate::init::{InitRequest, init_source};
    use std::fs;

    #[test]
    fn yaml_source_is_read_only_before_the_user_edits() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("yaml-course");
        init_source(&InitRequest {
            directory: root.clone(),
            package_id: "org.example/yaml-course".into(),
            language: "en".into(),
            title: Some("YAML course".into()),
        })
        .unwrap();
        fs::remove_file(root.join("osmium.json")).unwrap();
        fs::write(
            root.join("osmium.yaml"),
            r#"schema_version: "0.1"
package_id: org.example/yaml-course
package_version: "0.1.0"
title: YAML course
language: en
capabilities:
  required: []
  optional: []
entities:
  concepts: entities/concepts.json
  objectives: entities/objectives.json
  curricula: entities/curricula.json
  resources: entities/resources.json
  assessments: entities/assessments.json
extensions: {}
"#,
        )
        .unwrap();

        let editor = open_source(&root).expect("YAML source remains reviewable");
        assert!(!editor.editable);
        let diagnostics = save_source(SourceEdits {
            source_directory: root.to_string_lossy().into_owned(),
            title: "Changed".into(),
            language: "en".into(),
            resource_title: "Changed".into(),
            concept_title: "Changed".into(),
            markdown: "Changed".into(),
        })
        .expect_err("YAML saving stays unsupported");
        assert_eq!(diagnostics[0].code, "OSM_EDITOR_FORMAT");
        assert!(root.join("osmium.yaml").exists());
    }
}
