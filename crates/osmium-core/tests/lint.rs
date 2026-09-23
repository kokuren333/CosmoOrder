use osmium_core::lint::lint;
use osmium_core::validation::{PackageDocuments, validate_package};
use serde_json::{Value, json};

fn documents() -> PackageDocuments {
    PackageDocuments {
        manifest: serde_json::from_str(include_str!("../../../examples/arithmetic/osmium.json"))
            .unwrap(),
        concepts: serde_json::from_str(include_str!(
            "../../../examples/arithmetic/entities/concepts.json"
        ))
        .unwrap(),
        objectives: serde_json::from_str(include_str!(
            "../../../examples/arithmetic/entities/objectives.json"
        ))
        .unwrap(),
        curricula: serde_json::from_str(include_str!(
            "../../../examples/arithmetic/entities/curricula.json"
        ))
        .unwrap(),
        resources: serde_json::from_str(include_str!(
            "../../../examples/arithmetic/entities/resources.json"
        ))
        .unwrap(),
        assessments: serde_json::from_str(include_str!(
            "../../../examples/arithmetic/entities/assessments.json"
        ))
        .unwrap(),
    }
}

fn has(docs: PackageDocuments, code: &str) -> bool {
    lint(&validate_package(docs).unwrap())
        .iter()
        .any(|d| d.code == code)
}

#[test]
fn objective_resource_coverage() {
    let code = "OSM_LINT_OBJECTIVE_NO_RESOURCE";
    assert!(!has(documents(), code));
    let mut docs = documents();
    docs.resources = json!([]);
    assert!(has(docs, code));
}

#[test]
fn objective_assessment_coverage() {
    let code = "OSM_LINT_OBJECTIVE_NO_ASSESSMENT";
    assert!(!has(documents(), code));
    let mut docs = documents();
    docs.assessments = json!([]);
    assert!(has(docs, code));
}

#[test]
fn orphan_concept_is_advisory() {
    let code = "OSM_LINT_ORPHAN_CONCEPT";
    assert!(!has(documents(), code));
    let mut docs = documents();
    docs.concepts
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unused", "title":"Unused", "requires":[]}));
    assert!(has(docs, code));
    let mut docs = documents();
    docs.concepts
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"foundation", "title":"Foundation", "requires":[]}));
    docs.concepts[0]["requires"] = json!(["foundation"]);
    assert!(!has(docs, code));
}

#[test]
fn curriculum_coverage_skips_packages_without_curricula() {
    let code = "OSM_LINT_OBJECTIVE_NOT_IN_CURRICULUM";
    assert!(!has(documents(), code));
    let mut docs = documents();
    docs.curricula = json!([]);
    assert!(!has(docs, code));
    let mut docs = documents();
    docs.objectives
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"addition.extra", "concept":"addition", "description":"Extra"}));
    assert!(has(docs, code));
}

#[test]
fn prerequisite_order_requires_both_concepts_in_same_curriculum() {
    let code = "OSM_LINT_PREREQUISITE_ORDER";
    assert!(!has(documents(), code));
    let mut docs = documents();
    docs.concepts
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"carry", "title":"Carry", "requires":["addition"]}));
    docs.objectives
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"carry.basic", "concept":"carry", "description":"Carry"}));
    docs.curricula[0]["objectives"] = json!(["carry.basic", "addition.basic"]);
    assert!(has(docs.clone(), code));
    docs.curricula[0]["objectives"] = json!(["addition.basic", "carry.basic"]);
    assert!(!has(docs, code));
}

#[test]
fn resource_metadata_and_optional_capability_are_advisory() {
    let metadata = "OSM_LINT_METADATA";
    assert!(has(documents(), metadata));
    let mut docs = documents();
    docs.resources[0]["license"] = json!("CC0-1.0");
    assert!(!has(docs, metadata));

    let optional = "OSM_LINT_OPTIONAL";
    assert!(!has(documents(), optional));
    let mut docs = documents();
    docs.manifest["capabilities"]["optional"] = json!(["org.osmium.media.v1"]);
    assert!(has(docs, optional));
}

#[test]
fn findings_are_structured_and_ordered_deterministically() {
    let mut docs = documents();
    docs.resources = json!([]);
    docs.assessments = json!([]);
    docs.objectives
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"addition.extra", "concept":"addition", "description":"Extra"}));
    docs.concepts
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unused", "title":"Unused", "requires":[]}));
    let model = validate_package(docs).unwrap();
    let findings = lint(&model);
    assert_eq!(findings, lint(&model));
    assert!(findings.len() >= 4);
    for pair in findings.windows(2) {
        assert!(
            (
                &pair[0].file,
                &pair[0].entity_id,
                &pair[0].code,
                &pair[0].message
            ) <= (
                &pair[1].file,
                &pair[1].entity_id,
                &pair[1].code,
                &pair[1].message
            )
        );
    }
    for finding in findings {
        assert_eq!(finding.severity, "warning");
        assert!(finding.entity_type.is_some());
        assert!(finding.entity_id.is_some());
        let value: Value = serde_json::to_value(&finding).unwrap();
        assert!(value["code"].is_string());
        assert!(value["entity_id"].is_string());
    }
}
