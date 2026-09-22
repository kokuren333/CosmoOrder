use osmium_core::schema::{DocumentKind, SCHEMA_JSON, schema_for, validate_document};
use serde_json::{Value, json};

fn read(relative: &str) -> Value {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    serde_json::from_str(&std::fs::read_to_string(root.join(relative)).unwrap()).unwrap()
}

#[test]
fn golden_source_and_event_validate() {
    let manifest = read("examples/arithmetic/osmium.json");
    assert!(validate_document(DocumentKind::Manifest, &manifest).is_empty());
    for kind in DocumentKind::ALL {
        let value = match kind {
            DocumentKind::Manifest | DocumentKind::DistributionManifest => continue,
            DocumentKind::LearningEvent => read("fixtures/valid/learning-event.json"),
            _ => read(&format!(
                "examples/arithmetic/entities/{}.json",
                kind.definition()
            )),
        };
        assert!(validate_document(kind, &value).is_empty(), "{kind:?}");
    }
}

#[test]
fn malformed_fixtures_are_rejected_at_semantic_json_paths() {
    for (file, kind, pointer) in [
        (
            "assessment-wrong-answer-type",
            DocumentKind::Assessments,
            "/0",
        ),
        ("concept-unknown-core-field", DocumentKind::Concepts, "/0"),
        ("objective-missing-concept", DocumentKind::Objectives, "/0"),
    ] {
        let value = read(&format!("fixtures/invalid/{file}.json"));
        let errors = validate_document(kind, &value);
        assert!(!errors.is_empty(), "{file}");
        assert!(errors.iter().any(|error| error.path.starts_with(pointer)));
        assert!(errors.iter().all(|error| error.line.is_none()));
    }
}

#[test]
fn all_schemas_are_valid_and_contain_only_local_refs() {
    fn inspect(value: &Value) {
        match value {
            Value::Object(map) => {
                if let Some(reference) = map.get("$ref") {
                    assert!(reference.as_str().unwrap().starts_with("#/$defs/"));
                }
                for nested in map.values() {
                    inspect(nested);
                }
            }
            Value::Array(values) => values.iter().for_each(inspect),
            _ => {}
        }
    }
    inspect(&serde_json::from_str(SCHEMA_JSON).unwrap());
    for kind in DocumentKind::ALL {
        let schema = schema_for(kind);
        assert!(jsonschema::meta::is_valid(&schema));
        assert!(jsonschema::draft202012::new(&schema).is_ok());
    }
}

#[test]
fn schema_versions_and_core_typos_fail_but_namespaced_data_survives() {
    let mut manifest = read("examples/arithmetic/osmium.json");
    manifest["schema_version"] = json!("1.0");
    assert!(!validate_document(DocumentKind::Manifest, &manifest).is_empty());
    manifest["schema_version"] = json!("0.1");
    manifest["extensions"] = json!({"org.example.notes.v1": {"future": [1, "保持", null]}});
    let original = manifest.clone();
    assert!(validate_document(DocumentKind::Manifest, &manifest).is_empty());
    let roundtrip: Value =
        serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
    assert_eq!(roundtrip, original);
    manifest["typo"] = json!(true);
    assert!(!validate_document(DocumentKind::Manifest, &manifest).is_empty());
}

#[test]
fn required_fields_and_response_discriminants_are_enforced() {
    for kind in DocumentKind::ALL {
        assert!(!validate_document(kind, &Value::Null).is_empty());
    }
    let mut assessments = read("examples/arithmetic/entities/assessments.json");
    assessments[0]["response"]["type"] = json!("javascript");
    assert!(!validate_document(DocumentKind::Assessments, &assessments).is_empty());
    let mut event = read("fixtures/valid/learning-event.json");
    event["timestamp"] = json!("not-a-date");
    assert!(!validate_document(DocumentKind::LearningEvent, &event).is_empty());
    event["timestamp"] = json!("2026-09-22T00:00:00Z");
    event["score"] = json!(0.7);
    assert!(!validate_document(DocumentKind::LearningEvent, &event).is_empty());
}

#[test]
fn identifier_and_extension_names_are_portable() {
    let mut concepts = read("examples/arithmetic/entities/concepts.json");
    for invalid in ["../escape", "a/b", "", "A", "a..b"] {
        concepts[0]["id"] = json!(invalid);
        assert!(!validate_document(DocumentKind::Concepts, &concepts).is_empty());
    }
    concepts[0]["id"] = json!("addition");
    concepts[0]["extensions"] = json!({"unqualified": {}});
    assert!(!validate_document(DocumentKind::Concepts, &concepts).is_empty());
}
