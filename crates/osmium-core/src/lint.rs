//! Advisory static analysis of the authored package graph.
use crate::{schema::Diagnostic, validation::PackageModel};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn ids<'a>(entity: &'a Value, field: &str) -> impl Iterator<Item = &'a str> {
    entity[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// Findings have a stable order independent of authored array order.
pub fn lint(model: &PackageModel) -> Vec<Diagnostic> {
    let docs = model.documents();
    let concepts = docs.concepts.as_array().unwrap();
    let objectives = docs.objectives.as_array().unwrap();
    let curricula = docs.curricula.as_array().unwrap();
    let resources = docs.resources.as_array().unwrap();
    let assessments = docs.assessments.as_array().unwrap();
    let mut findings = Vec::new();
    let mut add =
        |document: &str, index: usize, entity_type: &str, id: &str, code: &str, message: String| {
            findings.push(Diagnostic {
                code: code.into(),
                severity: "warning".into(),
                entity_type: Some(entity_type.into()),
                entity_id: Some(id.into()),
                file: docs.manifest["entities"][document]
                    .as_str()
                    .map(str::to_owned),
                line: None,
                column: None,
                path: format!("/{index}/id"),
                message,
                suggestions: vec![],
            });
        };
    if docs
        .manifest
        .get("language")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        add(
            "manifest",
            0,
            "manifest",
            "language",
            "OSM_LINT_LANGUAGE",
            "package content language is unspecified".into(),
        );
    }
    let taught: BTreeSet<&str> = resources
        .iter()
        .flat_map(|item| ids(item, "teaches"))
        .collect();
    let measured: BTreeSet<&str> = assessments
        .iter()
        .flat_map(|item| ids(item, "measures"))
        .collect();
    let scheduled: BTreeSet<&str> = curricula
        .iter()
        .flat_map(|item| ids(item, "objectives"))
        .collect();
    let referenced_concepts: BTreeSet<&str> = objectives
        .iter()
        .filter_map(|item| item["concept"].as_str())
        .chain(concepts.iter().flat_map(|item| ids(item, "requires")))
        .collect();
    for (index, concept) in concepts.iter().enumerate() {
        let id = concept["id"].as_str().unwrap();
        if !referenced_concepts.contains(id) {
            add(
                "concepts",
                index,
                "concept",
                id,
                "OSM_LINT_ORPHAN_CONCEPT",
                format!("concept {id} is not referenced by any objective or prerequisite"),
            );
        }
    }
    for (index, objective) in objectives.iter().enumerate() {
        let id = objective["id"].as_str().unwrap();
        if !taught.contains(id) {
            add(
                "objectives",
                index,
                "objective",
                id,
                "OSM_LINT_OBJECTIVE_NO_RESOURCE",
                format!("objective {id} has no teaching resource"),
            );
        }
        if !measured.contains(id) {
            add(
                "objectives",
                index,
                "objective",
                id,
                "OSM_LINT_OBJECTIVE_NO_ASSESSMENT",
                format!("objective {id} has no assessment"),
            );
        }
        if !curricula.is_empty() && !scheduled.contains(id) {
            add(
                "objectives",
                index,
                "objective",
                id,
                "OSM_LINT_OBJECTIVE_NOT_IN_CURRICULUM",
                format!("objective {id} is not reachable from any curriculum"),
            );
        }
    }
    // Compare only objectives explicitly ordered in the same curriculum.
    let objective_concepts: BTreeMap<&str, &str> = objectives
        .iter()
        .map(|o| (o["id"].as_str().unwrap(), o["concept"].as_str().unwrap()))
        .collect();
    let concept_requires: BTreeMap<&str, Vec<&str>> = concepts
        .iter()
        .map(|c| (c["id"].as_str().unwrap(), ids(c, "requires").collect()))
        .collect();
    for (index, curriculum) in curricula.iter().enumerate() {
        let curriculum_id = curriculum["id"].as_str().unwrap();
        let ordered: Vec<&str> = ids(curriculum, "objectives").collect();
        let mut earliest: BTreeMap<&str, usize> = BTreeMap::new();
        for (position, id) in ordered.iter().enumerate() {
            if let Some(&concept) = objective_concepts.get(id) {
                earliest
                    .entry(concept)
                    .and_modify(|old| *old = (*old).min(position))
                    .or_insert(position);
            }
        }
        for (dependent, requirements) in &concept_requires {
            let Some(&dependent_position) = earliest.get(dependent) else {
                continue;
            };
            for required in requirements {
                if let Some(&required_position) = earliest.get(required) {
                    if required_position > dependent_position {
                        add(
                            "curricula",
                            index,
                            "curriculum",
                            curriculum_id,
                            "OSM_LINT_PREREQUISITE_ORDER",
                            format!(
                                "concept {required} is required before {dependent} but appears later in curriculum {curriculum_id}"
                            ),
                        );
                    }
                }
            }
        }
    }
    for (index, resource) in resources.iter().enumerate() {
        let id = resource["id"].as_str().unwrap();
        for field in ["creator", "license", "attribution"] {
            if resource
                .get(field)
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                add(
                    "resources",
                    index,
                    "resource",
                    id,
                    "OSM_LINT_METADATA",
                    format!("resource {id} lacks {field}"),
                );
            }
        }
        if resource.get("language").and_then(Value::as_str).is_none()
            && docs
                .manifest
                .get("language")
                .and_then(Value::as_str)
                .is_none()
        {
            add(
                "resources",
                index,
                "resource",
                id,
                "OSM_LINT_LANGUAGE",
                format!("resource {id} has no effective content language"),
            );
        }
        if let Some(sources) = docs.manifest.get("sources").and_then(Value::as_array) {
            let by_id: BTreeMap<&str, &Value> = sources
                .iter()
                .filter_map(|s| Some((s["id"].as_str()?, s)))
                .collect();
            let refs = ids(resource, "source_ids").collect::<Vec<_>>();
            if !refs.is_empty()
                && refs.iter().all(|source_id| {
                    by_id
                        .get(source_id)
                        .is_some_and(|s| s["visibility"] == "private")
                })
            {
                add(
                    "resources",
                    index,
                    "resource",
                    id,
                    "OSM_LINT_SOURCE_VISIBILITY",
                    format!(
                        "resource {id} has only private provenance; no learner-attributable source is declared"
                    ),
                );
            }
        }
    }
    for (index, capability) in ids(&docs.manifest["capabilities"], "optional").enumerate() {
        findings.push(Diagnostic {
            code: "OSM_LINT_OPTIONAL".into(),
            severity: "warning".into(),
            entity_type: Some("manifest".into()),
            entity_id: Some(capability.into()),
            file: Some("manifest".into()),
            line: None,
            column: None,
            path: format!("/capabilities/optional/{index}"),
            message: format!("optional capability {capability} is preserved but not implemented"),
            suggestions: vec![],
        });
    }
    findings.sort_by(|a, b| {
        (&a.file, &a.entity_id, &a.code, &a.message).cmp(&(
            &b.file,
            &b.entity_id,
            &b.code,
            &b.message,
        ))
    });
    findings
}
