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
