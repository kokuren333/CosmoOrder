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
    let mut docs = documents();
    docs.resources[0].as_object_mut().unwrap().remove("creator");
    assert!(has(docs.clone(), metadata));
    docs.resources[0]["creator"] = json!("Osmium example authors");
    assert!(!has(docs, metadata));

    let optional = "OSM_LINT_OPTIONAL";
    assert!(!has(documents(), optional));
    let mut docs = documents();
    docs.manifest["capabilities"]["optional"] = json!(["org.osmium.media.v1"]);
    assert!(has(docs, optional));
}

#[test]
fn license_lint_requires_a_meaningful_known_license_not_field_presence() {
    let code = "OSM_LINT_LICENSE_UNKNOWN";
    // The arithmetic example states no reuse status at all.
    assert!(has(documents(), code));

    // A placeholder sentence satisfies the old field-presence rule and must
    // still be reported: the field exists but says nothing usable.
    let mut docs = documents();
    docs.resources[0]["license"] = json!("再利用条件は未設定。");
    assert!(has(docs.clone(), code));

    // `unknown` is an honest status, not a known license.
    docs.resources[0]["license"] = json!("CC0-1.0");
    docs.resources[0]["license_status"] = json!("unknown");
    assert!(has(docs.clone(), code));

    // Only a known status plus a real license name clears the finding.
    docs.resources[0]["license_status"] = json!("known");
    assert!(!has(docs, code));
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

/// Add a reference registry plus the matching Evidence relation to a copy.
fn with_evidence(reference: Value) -> PackageDocuments {
    let mut docs = documents();
    docs.manifest["references"] = json!([reference]);
    let id = docs.manifest["references"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    docs.resources[0]["evidence_reference_ids"] = json!([id]);
    docs
}

#[test]
fn reference_metadata_and_evidence_visibility_are_advisory_heuristics() {
    let metadata = "OSM_LINT_REFERENCE_METADATA";
    // A bare locator with no citation, publisher or date is hard to verify later.
    let bare = with_evidence(json!({
        "id":"bare","kind":"url","locator":"https://example.org/a","visibility":"public"
    }));
    assert!(has(bare.clone(), metadata));
    // A citation, or a publisher with a date, is enough to clear the heuristic.
    let cited = with_evidence(json!({
        "id":"cited","kind":"url","locator":"https://example.org/a","citation":"Example. Report.",
        "visibility":"public"
    }));
    assert!(!has(cited, metadata));
    let dated = with_evidence(json!({
        "id":"dated","kind":"url","locator":"https://example.org/a","publisher":"Example",
        "updated_at":"2026-01","visibility":"public"
    }));
    assert!(!has(dated, metadata));

    // Evidence that resolves only to private authoring provenance is reported,
    // because a learner can never see it.
    let private = with_evidence(json!({
        "id":"private-only","kind":"local_file","locator":"notes/a.md","visibility":"private"
    }));
    assert!(has(private, "OSM_LINT_SOURCE_VISIBILITY"));
}

#[test]
fn assessment_depth_and_reused_distractors_are_advisory_heuristics() {
    let level = "OSM_LINT_ASSESSMENT_COGNITIVE_LEVEL";
    assert!(has(documents(), level));
    let mut docs = documents();
    for assessment in docs.assessments.as_array_mut().unwrap() {
        assessment["cognitive_level"] = json!("recall");
    }
    assert!(!has(docs, level));

    // A short two-option item is reported; a longer or wider item is not.
    let shallow = "OSM_LINT_ASSESSMENT_SHALLOW";
    let mut docs = documents();
    docs.assessments[0]["stimulus"] = json!({"markdown":"短い問い"});
    assert!(has(docs.clone(), shallow));
    docs.assessments[0]["stimulus"] = json!({"markdown":"この教材の内容を踏まえ、二つの概念の違いを説明する記述として最も適切なものを選んでください。その理由も考えながら読みます。"});
    assert!(!has(docs, shallow));

    // Reused distractor text across items is reported once per owning item.
    let reused = "OSM_LINT_REUSED_DISTRACTOR";
    let mut docs = documents();
    let mut second = docs.assessments[0].clone();
    second["id"] = json!("addition.03");
    docs.assessments.as_array_mut().unwrap().push(second);
    assert!(has(docs, reused));
}
