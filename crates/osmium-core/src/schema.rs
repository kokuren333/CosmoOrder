//! Embedded, offline JSON Schema validation. Callers cannot supply schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

pub const SCHEMA_VERSION: &str = "0.1";
pub const SCHEMA_JSON: &str = include_str!("../../../spec/v0.1/package.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Manifest,
    DistributionManifest,
    Concepts,
    Objectives,
    Curricula,
    Resources,
    Assessments,
    LearningEvent,
}

impl DocumentKind {
    pub const ALL: [Self; 8] = [
        Self::Manifest,
        Self::DistributionManifest,
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
            Self::DistributionManifest => "distribution_manifest",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
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

/// Validator for a single entity ID, so authoring tools reject a bad identifier
/// before writing a file instead of producing an unloadable package.
static ID_VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let mut schema: Value =
        serde_json::from_str(SCHEMA_JSON).expect("embedded schema is valid JSON");
    schema["$ref"] = Value::String("#/$defs/id".into());
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("the ID definition is valid")
});

/// Whether a string is a syntactically valid entity or Reference ID.
pub fn is_valid_id(id: &str) -> bool {
    ID_VALIDATOR.is_valid(&Value::String(id.to_owned()))
}

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
            entity_type: None,
            entity_id: None,
            file: None,
            line: None,
            column: None,
            path: error.instance_path().to_string(),
            message: error.to_string(),
            suggestions: Vec::new(),
        })
        .collect()
}
