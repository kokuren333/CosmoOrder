//! Advisory checks. Structural coverage is not an educational quality score.
use crate::{schema::Diagnostic, validation::PackageModel};
use std::collections::BTreeSet;

pub fn lint(model: &PackageModel) -> Vec<Diagnostic> {
    let docs = model.documents();
    let mut findings = Vec::new();
    let mut add = |kind: &str, path: String, code: &str, message: String| {
        findings.push(Diagnostic {
            code: code.into(),
            severity: "warning".into(),
            file: Some(
                docs.manifest["entities"][kind]
                    .as_str()
                    .unwrap_or("manifest")
                    .into(),
            ),
            line: None,
            column: None,
            path,
            message,
            suggestions: vec![],
        });
    };
    for (index, resource) in docs.resources.as_array().unwrap().iter().enumerate() {
        for field in ["creator", "license", "attribution"] {
            if resource
                .get(field)
                .and_then(|v| v.as_str())
                .is_none_or(|s| s.trim().is_empty())
            {
                add(
                    "resources",
                    format!("/{index}/{field}"),
                    "OSM_LINT_METADATA",
                    format!(
                        "resource {} lacks {field}",
                        resource["id"].as_str().unwrap()
                    ),
                );
            }
        }
    }
    for (kind, field, document, code) in [
        ("resource", "teaches", &docs.resources, "OSM_LINT_UNTAUGHT"),
        (
            "assessment",
            "measures",
            &docs.assessments,
            "OSM_LINT_UNMEASURED",
        ),
        (
            "curriculum",
            "objectives",
            &docs.curricula,
            "OSM_LINT_UNSCHEDULED",
        ),
    ] {
        let covered: BTreeSet<&str> = document
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|e| e[field].as_array().unwrap())
            .map(|id| id.as_str().unwrap())
            .collect();
        for (index, objective) in docs.objectives.as_array().unwrap().iter().enumerate() {
            let id = objective["id"].as_str().unwrap();
            if !covered.contains(id) {
                add(
                    "objectives",
                    format!("/{index}/id"),
                    code,
                    format!("objective {id} has no associated {kind}"),
                );
            }
        }
    }
    for (index, capability) in docs.manifest["capabilities"]["optional"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        add(
            "manifest",
            format!("/capabilities/optional/{index}"),
            "OSM_LINT_OPTIONAL",
            format!(
                "optional capability {} is preserved but not implemented",
                capability.as_str().unwrap()
            ),
        );
    }
    findings
}
