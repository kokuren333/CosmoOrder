use osmium_core::validation::{PackageDocuments, validate_package, validate_relative_path};
use serde_json::{Value, json};

fn example() -> PackageDocuments {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic");
    let read = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };
    PackageDocuments {
        manifest: read("osmium.json"),
        concepts: read("entities/concepts.json"),
        objectives: read("entities/objectives.json"),
        curricula: read("entities/curricula.json"),
        resources: read("entities/resources.json"),
        assessments: read("entities/assessments.json"),
    }
}

fn has_error(documents: PackageDocuments, code: &str, path: &str) {
    let errors = validate_package(documents).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == code && e.path == path),
        "{errors:?}"
    );
}

#[test]
fn valid_model_preserves_optional_extensions() {
    let mut docs = example();
    docs.manifest["capabilities"]["optional"] = json!(["org.example.future.v1"]);
    docs.manifest["extensions"]["org.example.future.v1"] = json!({"nested": [null, "保存", 23]});
    let expected = docs.manifest.clone();
    let model = validate_package(docs).unwrap();
    assert_eq!(model.documents().manifest, expected);
    assert_eq!(model.prerequisite_order(), ["addition"]);
}

#[test]
fn typed_references_do_not_accept_a_concept_as_an_objective() {
    let mut docs = example();
    docs.resources[0]["teaches"] = json!(["addition"]);
    has_error(docs, "OSM_REFERENCE", "/0/teaches/0");
    let mut docs = example();
    docs.objectives[0]["concept"] = json!("missing");
    has_error(docs, "OSM_REFERENCE", "/0/concept");
    let mut docs = example();
    docs.curricula[0]["objectives"] = json!(["missing"]);
    has_error(docs, "OSM_REFERENCE", "/0/objectives/0");
    let mut docs = example();
    docs.assessments[0]["measures"] = json!(["missing"]);
    has_error(docs, "OSM_REFERENCE", "/0/measures/0");
}

#[test]
fn duplicate_entity_and_option_ids_and_invalid_answer_are_rejected() {
    let mut docs = example();
    let mut duplicate = docs.concepts[0].clone();
    duplicate["title"] = json!("Different title, same identity");
    docs.concepts.as_array_mut().unwrap().push(duplicate);
    has_error(docs, "OSM_DUPLICATE_ID", "/1/id");
    let mut docs = example();
    docs.assessments[0]["response"]["options"][1]["id"] = json!("a");
    has_error(docs, "OSM_DUPLICATE_OPTION", "/0/response/options/1/id");
    let mut docs = example();
    docs.assessments[0]["evaluation"]["answer"] = json!("absent");
    has_error(docs, "OSM_ANSWER", "/0/evaluation/answer");
}

#[test]
fn unknown_required_capability_and_version_are_explicit() {
    let mut docs = example();
    docs.manifest["capabilities"]["required"] = json!(["org.example.future.v1"]);
    has_error(docs, "OSM_CAPABILITY", "/capabilities/required/0");
    let mut docs = example();
    docs.manifest["schema_version"] = json!("2.0");
    has_error(docs, "OSM_SCHEMA_VERSION", "/schema_version");
}

#[test]
fn source_registry_checks_references_visibility_and_portability() {
    let mut docs = example();
    docs.manifest["sources"] = json!([
        {"id":"public-ref","kind":"url","locator":"https://example.org/guide","visibility":"public"},
        {"id":"private-ref","kind":"local_file","locator":"C:\\Users\\Example\\notes.pdf","visibility":"private"}
    ]);
    docs.resources[0]["source_ids"] = json!(["public-ref", "private-ref"]);
    docs.resources[0]["language"] = json!("ja-JP");
    assert!(validate_package(docs.clone()).is_ok());
    docs.resources[0]["source_ids"] = json!(["missing"]);
    has_error(docs, "OSM_SOURCE_REFERENCE", "/0/source_ids/0");

    for locator in [
        "C:/private/file.pdf",
        "/home/person/paper.pdf",
        "https://example.org/g?token=secret",
        "https://user:secret@example.org/g",
    ] {
        let mut docs = example();
        docs.manifest["sources"] =
            json!([{"id":"public-ref","kind":"url","locator":locator,"visibility":"public"}]);
        assert!(
            validate_package(docs)
                .unwrap_err()
                .iter()
                .any(|d| d.code == "OSM_SOURCE_PRIVACY"),
            "{locator}"
        );
    }
    let mut docs = example();
    docs.manifest["sources"] = json!([{"id":"asset-ref","kind":"package_asset","locator":"../outside.pdf","visibility":"public"}]);
    has_error(docs, "OSM_SOURCE_LOCATOR", "/sources/0/locator");
    let mut docs = example();
    docs.resources[0]["language"] = json!("not a language tag ???");
    assert!(
        validate_package(docs)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "OSM_LANGUAGE")
    );
}

#[test]
fn prerequisite_cycles_and_self_edges_fail() {
    let mut docs = example();
    docs.concepts[0]["requires"] = json!(["addition"]);
    has_error(docs, "OSM_CYCLE", "");
    let mut docs = example();
    docs.concepts = json!([
        {"id":"addition", "title":"A", "requires":["b"]},
        {"id":"b", "title":"B", "requires":["c"]},
        {"id":"c", "title":"C", "requires":["addition"]}
    ]);
    has_error(docs, "OSM_CYCLE", "");
}

#[test]
fn long_chain_is_iterative_and_prerequisites_precede_dependents() {
    let mut docs = example();
    let mut concepts = vec![json!({"id":"addition","title":"A","requires":[]})];
    for n in 1..2048 {
        let previous = if n == 1 {
            "addition".into()
        } else {
            format!("c{}", n - 1)
        };
        concepts.push(json!({"id":format!("c{n}"),"title":"Concept","requires":[previous]}));
    }
    concepts.reverse();
    docs.concepts = json!(concepts);
    let model = validate_package(docs).unwrap();
    assert_eq!(model.prerequisite_order().len(), 2048);
    assert_eq!(model.prerequisite_order()[0], "addition");
    assert_eq!(model.prerequisite_order()[2047], "c2047");
}

#[test]
fn curriculum_order_does_not_create_concept_edges() {
    let mut docs = example();
    docs.concepts
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"advanced","title":"Advanced","requires":["addition"]}));
    docs.objectives
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"advanced.basic","concept":"advanced","description":"Learn advanced"}));
    docs.curricula[0]["objectives"] = json!(["advanced.basic", "addition.basic"]);
    let model = validate_package(docs).unwrap();
    assert_eq!(model.prerequisite_order(), ["addition", "advanced"]);
    assert_eq!(
        model.documents().curricula[0]["objectives"][0],
        "advanced.basic"
    );
}

#[test]
fn portable_paths_reject_escape_and_windows_aliases_on_every_platform() {
    for path in [
        "",
        "/absolute",
        "../escape",
        "a/../escape",
        "a/./b",
        "a//b",
        "a/",
        "C:/escape",
        "\\\\host\\share",
        "a\\b",
        "a:b",
        "a/CON.txt",
        "nul",
        "COM1",
        "lpt².txt",
        "conin$",
        "a.",
        "a ",
        "a?b",
        "a\0b",
    ] {
        assert!(validate_relative_path(path).is_err(), "{path}");
    }
    for path in [
        "content/lesson.md",
        "教材/説明.md",
        "assets/com10.txt",
        "a-b/c_d.json",
    ] {
        assert!(validate_relative_path(path).is_ok(), "{path}");
    }
    let mut docs = example();
    docs.resources[0]["path"] = json!("../outside.md");
    has_error(docs, "OSM_PATH", "/0/path");
    let mut docs = example();
    docs.manifest["entities"]["concepts"] = json!("Entities/Objectives.json");
    has_error(docs, "OSM_PATH_COLLISION", "/entities/objectives");
}

#[test]
fn malformed_structure_returns_diagnostics_instead_of_panicking() {
    let mut docs = example();
    docs.concepts = json!({"not":"an array"});
    assert!(validate_package(docs).is_err());
    let mut docs = example();
    docs.manifest = Value::Null;
    assert!(validate_package(docs).is_err());
}

#[test]
fn canonical_reference_registry_accepts_evidence_and_bibliographic_metadata() {
    let mut docs = example();
    docs.manifest["references"] = json!([{
        "id": "who-guideline",
        "kind": "url",
        "type": "guideline",
        "title": "WHO guideline",
        "locator": "https://www.who.int/publications/example?lang=en",
        "citation": "World Health Organization. Example guideline.",
        "publisher": "World Health Organization",
        "authors": ["World Health Organization"],
        "published_at": "2024-01",
        "updated_at": "2025-06",
        "accessed_at": "2026-01-15",
        "edition": "2nd",
        "version": "v2.1",
        "identifiers": {"doi": "10.1000/example"},
        "visibility": "public",
        "record_visibility": "public",
        "locator_visibility": "public"
    }]);
    docs.resources[0]["evidence_reference_ids"] = json!(["who-guideline"]);
    let model = validate_package(docs.clone()).unwrap();
    // The canonical registry name is preserved verbatim, not normalised.
    assert!(model.documents().manifest.get("sources").is_none());

    docs.resources[0]["evidence_reference_ids"] = json!(["absent"]);
    has_error(docs, "OSM_SOURCE_REFERENCE", "/0/evidence_reference_ids/0");

    let mut docs = example();
    docs.manifest["references"] =
        json!([{"id":"ref","kind":"url","locator":"https://example.org/a","visibility":"public"}]);
    docs.assessments[0]["evidence_reference_ids"] = json!(["absent"]);
    has_error(docs, "OSM_SOURCE_REFERENCE", "/0/evidence_reference_ids/0");
}

#[test]
fn query_string_policy_rejects_credentials_but_allows_publication_queries() {
    for locator in [
        "https://example.org/g?token=secret",
        "https://example.org/g?access_token=abc",
        "https://example.org/g?access%5Ftoken=abc",
        "https://example.org/g?api_key=abc",
        "https://example.org/g?apikey=abc",
        "https://example.org/g?secret=abc",
        "https://example.org/g?signature=abc",
        "https://example.org/g?sig=abc",
        "https://example.org/g?X-Amz-Signature=abc",
        "https://example.org/g?X-Amz-Credential=abc",
        "https://example.org/g?X-Amz-Security-Token=abc",
        "https://example.org/g?client_secret=abc",
        "https://example.org/g?password=abc",
        "https://user:secret@example.org/g",
        "https://user@example.org/g",
    ] {
        let mut docs = example();
        docs.manifest["references"] =
            json!([{"id":"ref","kind":"url","locator":locator,"visibility":"public"}]);
        let errors = validate_package(docs).unwrap_err();
        assert!(
            errors.iter().any(|d| d.code == "OSM_SOURCE_PRIVACY"),
            "{locator}: {errors:?}"
        );
    }
    // Ordinary publication queries are not credentials and must stay valid.
    for locator in [
        "https://example.org/article?id=123",
        "https://example.org/article?id=123&lang=ja",
        "https://example.org/search?q=fluid+balance",
        "https://example.org/a?article=foo#section-2",
        "https://example.org/a?format=pdf",
        "https://example.org/a?id=1&page=4",
        "https://example.org/search?email=person@example.net",
    ] {
        let mut docs = example();
        docs.manifest["references"] =
            json!([{"id":"ref","kind":"url","locator":locator,"visibility":"public"}]);
        assert!(validate_package(docs).is_ok(), "{locator}");
    }
}

#[test]
fn hidden_locator_records_keep_a_citation_without_a_distributable_locator() {
    let mut docs = example();
    docs.manifest["references"] = json!([{
        "id": "internal",
        "kind": "url",
        "title": "Internal document",
        "citation": "Example Organization. Internal document.",
        "locator": "https://intranet.example.org/draft",
        "visibility": "attribution_only",
        "record_visibility": "public",
        "locator_visibility": "hidden"
    }]);
    docs.resources[0]["evidence_reference_ids"] = json!(["internal"]);
    // A locator that never distributes is not a public-locator privacy defect,
    // but removing the locator entirely must still validate.
    assert!(validate_package(docs.clone()).is_ok());
    docs.manifest["references"][0]
        .as_object_mut()
        .unwrap()
        .remove("locator");
    assert!(validate_package(docs).is_ok());
}

#[test]
fn hidden_and_private_locators_cannot_store_recognizable_credentials() {
    for visibility in ["attribution_only", "private"] {
        let mut docs = example();
        docs.manifest["references"] = json!([{
            "id": "sensitive",
            "kind": "url",
            "title": "Hidden document",
            "citation": "Example Organization. Internal document.",
            "locator": "https://example.org/draft?access_token=credential",
            "visibility": visibility
        }]);
        has_error(docs, "OSM_SOURCE_PRIVACY", "/references/0/locator");
    }
}

#[test]
fn declared_evidence_without_a_registry_is_rejected() {
    let mut docs = example();
    docs.resources[0]["evidence_reference_ids"] = json!(["anything"]);
    has_error(docs, "OSM_SOURCE_REFERENCE", "/0/evidence_reference_ids");
}

#[test]
fn canonical_and_legacy_reference_aliases_cannot_be_mixed() {
    let mut docs = example();
    docs.manifest["references"] = json!([{
        "id": "public-reference",
        "kind": "url",
        "locator": "https://example.org/public",
        "visibility": "public"
    }]);
    docs.manifest["sources"] = json!([{
        "id": "private-reference",
        "kind": "url",
        "locator": "https://example.org/private?token=secret",
        "visibility": "private"
    }]);
    has_error(docs, "OSM_SOURCE_REFERENCE", "/references");

    let mut docs = example();
    docs.manifest["references"] = json!([{
        "id": "reference",
        "kind": "url",
        "locator": "https://example.org/public",
        "visibility": "public"
    }]);
    docs.resources[0]["evidence_reference_ids"] = json!([]);
    docs.resources[0]["source_ids"] = json!(["reference"]);
    has_error(docs, "OSM_SOURCE_REFERENCE", "/0/evidence_reference_ids");
}

#[test]
fn learner_result_fields_cannot_be_written_into_an_assessment_definition() {
    // The definition/result boundary is structural, not a convention: a result
    // field on a definition is a schema error, and so is a definition field on
    // a learning event that does not belong there.
    for (field, value) in [
        ("score", json!(1)),
        ("response", json!("a")),
        ("correct", json!(true)),
        ("attempt", json!(1)),
        ("timestamp", json!("2026-01-15T00:00:00Z")),
        ("duration_ms", json!(1200)),
        ("hints_used", json!(0)),
    ] {
        let mut docs = example();
        docs.assessments[0][field] = value;
        let errors = validate_package(docs).unwrap_err();
        assert!(
            errors.iter().any(|d| d.code == "OSM_SCHEMA"),
            "{field} must not be accepted on an assessment definition"
        );
    }
    // The definition is what an event snapshots; the event never gains the
    // definition's own required fields by accident.
    let mut docs = example();
    docs.assessments[0]["stimulus"] = json!({"markdown": "問題"});
    assert!(validate_package(docs).is_ok());
}

#[test]
fn an_assessment_must_name_at_least_one_objective() {
    // `measures` is the Assessment -> Objective link and is required, so an item
    // that measures nothing cannot be authored at all.
    let mut docs = example();
    docs.assessments[0]["measures"] = json!([]);
    assert!(validate_package(docs).is_err());
    let mut docs = example();
    docs.assessments[0]["measures"] = json!(["missing.objective"]);
    has_error(docs, "OSM_REFERENCE", "/0/measures/0");
}
