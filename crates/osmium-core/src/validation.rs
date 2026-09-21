//! Semantic validation of an in-memory source package.
//!
//! File loading, duplicate JSON key rejection and archive checks belong to
//! the input boundary. This module never accesses a filesystem or network.

use crate::schema::{Diagnostic, DocumentKind, SCHEMA_VERSION, validate_document};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Untrusted documents supplied by the source loader. Values preserve optional
/// extensions without tying the portable representation to a UI framework.
#[derive(Debug, Clone)]
pub struct PackageDocuments {
    pub manifest: Value,
    pub concepts: Value,
    pub objectives: Value,
    pub curricula: Value,
    pub resources: Value,
    pub assessments: Value,
}

impl PackageDocuments {
    pub(crate) fn document(&self, kind: DocumentKind) -> &Value {
        match kind {
            DocumentKind::Manifest => &self.manifest,
            DocumentKind::Concepts => &self.concepts,
            DocumentKind::Objectives => &self.objectives,
            DocumentKind::Curricula => &self.curricula,
            DocumentKind::Resources => &self.resources,
            DocumentKind::Assessments => &self.assessments,
            DocumentKind::LearningEvent => {
                panic!("learning events are not package documents")
            }
        }
    }
}

/// Only semantic validation can construct this model. This does not attest
/// that referenced file contents have passed the loader's security checks.
#[derive(Debug, Clone)]
pub struct PackageModel {
    documents: PackageDocuments,
    prerequisite_order: Vec<String>,
}

impl PackageModel {
    pub fn documents(&self) -> &PackageDocuments {
        &self.documents
    }

    /// Deterministic topological order: prerequisites occur before dependents.
    /// This is not a Curriculum recommendation or a mastery estimate.
    pub fn prerequisite_order(&self) -> &[String] {
        &self.prerequisite_order
    }
}

fn error(file: &str, path: String, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: "error".into(),
        file: Some(file.into()),
        line: None,
        column: None,
        path,
        message: message.into(),
        suggestions: Vec::new(),
    }
}

/// Portable relative path rules, including Windows aliases on every platform.
/// Filesystem containment and symlink/reparse checks are still required.
pub fn validate_relative_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() || path.chars().count() > 240 {
        return Err("path must contain 1 to 240 characters");
    }
    if path
        .chars()
        .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
    {
        return Err("path contains a control character or a nonportable separator");
    }
    for component in path.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with(['.', ' '])
        {
            return Err("path contains an empty, dot, parent or trailing dot/space component");
        }
        let stem = component.split('.').next().unwrap().to_uppercase();
        if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|suffix| {
                    matches!(
                        suffix,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            })
        {
            return Err("path contains a reserved Windows device name");
        }
    }
    Ok(())
}

/// Validate all structural and semantic relationships before exposing a model.
/// No additional required capabilities are implemented in format 0.1 yet.
pub fn validate_package(documents: PackageDocuments) -> Result<PackageModel, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    if documents
        .manifest
        .get("schema_version")
        .is_some_and(|v| v != SCHEMA_VERSION)
    {
        return Err(vec![error(
            "manifest",
            "/schema_version".into(),
            "OSM_SCHEMA_VERSION",
            "unsupported schema version",
        )]);
    }
    for kind in DocumentKind::ALL {
        if kind == DocumentKind::LearningEvent {
            continue;
        }
        for mut diagnostic in validate_document(kind, documents.document(kind)) {
            diagnostic.file = Some(kind.definition().into());
            errors.push(diagnostic);
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // These accessors are safe after the fixed schemas have accepted every
    // document; invalid raw inputs never reach the semantic pass.
    let concepts = documents.concepts.as_array().unwrap();
    if language_tags::LanguageTag::parse(documents.manifest["language"].as_str().unwrap()).is_err()
    {
        errors.push(error(
            "manifest",
            "/language".into(),
            "OSM_LANGUAGE",
            "language must be a syntactically well-formed BCP 47 tag",
        ));
    }
    let objectives = documents.objectives.as_array().unwrap();
    let resources = documents.resources.as_array().unwrap();
    let assessments = documents.assessments.as_array().unwrap();
    let mut indices = BTreeMap::new();
    for kind in [
        DocumentKind::Concepts,
        DocumentKind::Objectives,
        DocumentKind::Curricula,
        DocumentKind::Resources,
        DocumentKind::Assessments,
    ] {
        let mut index = BTreeMap::new();
        for (position, entity) in documents
            .document(kind)
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let id = entity["id"].as_str().unwrap();
            if index.insert(id, position).is_some() {
                errors.push(error(
                    kind.definition(),
                    format!("/{position}/id"),
                    "OSM_DUPLICATE_ID",
                    format!("duplicate {} ID: {id}", kind.definition()),
                ));
            }
        }
        indices.insert(kind.definition(), index);
    }

    for (position, required) in documents.manifest["capabilities"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        errors.push(error(
            "manifest",
            format!("/capabilities/required/{position}"),
            "OSM_CAPABILITY",
            format!(
                "unsupported required capability: {}",
                required.as_str().unwrap()
            ),
        ));
    }
    let optional = documents.manifest["capabilities"]["optional"]
        .as_array()
        .unwrap();
    for required in documents.manifest["capabilities"]["required"]
        .as_array()
        .unwrap()
    {
        if optional.contains(required) {
            errors.push(error(
                "manifest",
                "/capabilities".into(),
                "OSM_CAPABILITY_CONFLICT",
                "a capability cannot be both required and optional",
            ));
        }
    }

    for (name, path) in documents.manifest["entities"].as_object().unwrap() {
        if let Err(reason) = validate_relative_path(path.as_str().unwrap()) {
            errors.push(error(
                "manifest",
                format!("/entities/{name}"),
                "OSM_PATH",
                reason,
            ));
        }
    }
    let mut paths = BTreeSet::new();
    for (name, path) in documents.manifest["entities"].as_object().unwrap() {
        if !paths.insert(path.as_str().unwrap().to_lowercase()) {
            errors.push(error(
                "manifest",
                format!("/entities/{name}"),
                "OSM_PATH_COLLISION",
                "entity documents must have distinct paths, including case",
            ));
        }
    }

    let mut reference = |file: &str, path: String, id: &str, target: &str| {
        if !indices[target].contains_key(id) {
            errors.push(error(
                file,
                path,
                "OSM_REFERENCE",
                format!("unknown {target} ID: {id}"),
            ));
        }
    };
    for (position, concept) in concepts.iter().enumerate() {
        for (edge, required) in concept["requires"].as_array().unwrap().iter().enumerate() {
            reference(
                "concepts",
                format!("/{position}/requires/{edge}"),
                required.as_str().unwrap(),
                "concepts",
            );
        }
    }
    for (position, objective) in objectives.iter().enumerate() {
        reference(
            "objectives",
            format!("/{position}/concept"),
            objective["concept"].as_str().unwrap(),
            "concepts",
        );
    }
    for (kind, field) in [
        (DocumentKind::Curricula, "objectives"),
        (DocumentKind::Resources, "teaches"),
        (DocumentKind::Assessments, "measures"),
    ] {
        for (position, entity) in documents
            .document(kind)
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            for (edge, target) in entity[field].as_array().unwrap().iter().enumerate() {
                reference(
                    kind.definition(),
                    format!("/{position}/{field}/{edge}"),
                    target.as_str().unwrap(),
                    "objectives",
                );
            }
        }
    }
    for (position, resource) in resources.iter().enumerate() {
        if let Err(reason) = validate_relative_path(resource["path"].as_str().unwrap()) {
            errors.push(error(
                "resources",
                format!("/{position}/path"),
                "OSM_PATH",
                reason,
            ));
        }
    }
    for (position, assessment) in assessments.iter().enumerate() {
        if assessment["response"]["type"] == "single_select" {
            let mut options = BTreeSet::new();
            for (option, entry) in assessment["response"]["options"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                let id = entry["id"].as_str().unwrap();
                if !options.insert(id) {
                    errors.push(error(
                        "assessments",
                        format!("/{position}/response/options/{option}/id"),
                        "OSM_DUPLICATE_OPTION",
                        format!("duplicate option ID: {id}"),
                    ));
                }
            }
            if !options.contains(assessment["evaluation"]["answer"].as_str().unwrap()) {
                errors.push(error(
                    "assessments",
                    format!("/{position}/evaluation/answer"),
                    "OSM_ANSWER",
                    "correct answer must name an existing option",
                ));
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    // Kahn's algorithm avoids a recursive DFS stack overflow on a long chain.
    let mut remaining = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for concept in concepts {
        let id = concept["id"].as_str().unwrap();
        let requires = concept["requires"].as_array().unwrap();
        remaining.insert(id, requires.len());
        for required in requires {
            dependents
                .entry(required.as_str().unwrap())
                .or_default()
                .push(id);
        }
    }
    let mut ready: VecDeque<&str> = remaining
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut order = Vec::new();
    while let Some(id) = ready.pop_front() {
        order.push(id.to_owned());
        if let Some(children) = dependents.get(id) {
            for child in children {
                let count = remaining.get_mut(child).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.push_back(child);
                }
            }
        }
    }
    if order.len() != concepts.len() {
        return Err(vec![error(
            "concepts",
            String::new(),
            "OSM_CYCLE",
            "requires graph contains a cycle; Curriculum order is separate",
        )]);
    }
    Ok(PackageModel {
        documents,
        prerequisite_order: order,
    })
}
