//! Embedded, offline JSON Schema validation. Callers cannot supply schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

pub const SCHEMA_VERSION: &str = "0.1";
pub const SCHEMA_JSON: &str = include_str!("../../../spec/v0.1/package.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Manifest,
    Concepts,
    Objectives,
    Curricula,
    Resources,
    Assessments,
    LearningEvent,
}

impl DocumentKind {
    pub const ALL: [Self; 7] = [
        Self::Manifest,
        Self::Concepts,
        Self::Objectives,
        Self::Curricula,
        Self::Resources,
        Self::Assessments,
        Self::LearningEvent,
    ];

    pub fn definition(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Concepts => "concepts",
            Self::Objectives => "objectives",
            Self::Curricula => "curricula",
            Self::Resources => "resources",
            Self::Assessments => "assessments",
            Self::LearningEvent => "learning_event",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub path: String,
    pub message: String,
    pub suggestions: Vec<String>,
}

/// Produce a standalone schema selecting one definition from the fixed bundle.
pub fn schema_for(kind: DocumentKind) -> Value {
    let mut schema: Value =
        serde_json::from_str(SCHEMA_JSON).expect("embedded schema is valid JSON");
    schema["$ref"] = Value::String(format!("#/$defs/{}", kind.definition()));
    schema
}

static VALIDATORS: LazyLock<Vec<(DocumentKind, jsonschema::Validator)>> = LazyLock::new(|| {
    DocumentKind::ALL
        .into_iter()
        .map(|kind| {
            let validator = jsonschema::draft202012::options()
                .should_validate_formats(true)
                .build(&schema_for(kind))
                .expect("embedded schemas pass conformance tests");
            (kind, validator)
        })
        .collect()
});

/// Validate structure without mutating input or loading external resources.
/// Locations refer to JSON pointers; source coordinates require a parser map.
pub fn validate_document(kind: DocumentKind, document: &Value) -> Vec<Diagnostic> {
    let validator = &VALIDATORS.iter().find(|(k, _)| *k == kind).unwrap().1;
    validator
        .iter_errors(document)
        .take(100)
        .map(|error| Diagnostic {
            code: "OSM_SCHEMA".into(),
            severity: "error".into(),
            file: None,
            line: None,
            column: None,
            path: error.instance_path().to_string(),
            message: error.to_string(),
            suggestions: Vec::new(),
        })
        .collect()
}
