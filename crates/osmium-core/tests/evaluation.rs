use osmium_core::{
    evaluation::evaluate,
    validation::{PackageDocuments, PackageModel, validate_package},
};
use serde_json::{Value, json};

fn model() -> PackageModel {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic");
    let read =
        |path: &str| serde_json::from_slice(&std::fs::read(root.join(path)).unwrap()).unwrap();
    validate_package(PackageDocuments {
        manifest: read("osmium.json"),
        concepts: read("entities/concepts.json"),
        objectives: read("entities/objectives.json"),
        curricula: read("entities/curricula.json"),
        resources: read("entities/resources.json"),
        assessments: read("entities/assessments.json"),
    })
    .unwrap()
}

#[test]
fn all_golden_items_grade_deterministically_with_snapshot_and_feedback() {
    let model = model();
    let before = model.documents().assessments.clone();
    for item in before.as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        let answer = &item["evaluation"]["answer"];
        let result = evaluate(&model, id, answer).unwrap();
        assert_eq!(result, evaluate(&model, id, answer).unwrap());
        assert_eq!(result.score, 1);
        assert!(result.correct);
        assert_eq!(result.assessment_snapshot, *item);
        assert_eq!(result.feedback, item["feedback"]);
        assert_eq!(result.objective_ids, ["addition.basic"]);
        assert_eq!(result.concept_ids, ["addition"]);
        let wrong = if answer.is_boolean() {
            json!(!answer.as_bool().unwrap())
        } else {
            item["response"]["options"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["id"] != *answer)
                .unwrap()["id"]
                .clone()
        };
        assert_eq!(evaluate(&model, id, &wrong).unwrap().score, 0);
    }
    assert_eq!(model.documents().assessments, before);
}

#[test]
fn malformed_responses_are_rejected_instead_of_recorded_as_incorrect() {
    let model = model();
    for item in model.documents().assessments.as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        for invalid in [
            Value::Null,
            json!([]),
            json!({}),
            json!(1),
            json!("not-an-option"),
        ] {
            assert_eq!(
                evaluate(&model, id, &invalid).unwrap_err()[0].code,
                "OSM_RESPONSE"
            );
        }
        let wrong_type = if item["response"]["type"] == "boolean" {
            json!("true")
        } else {
            json!(true)
        };
        assert!(evaluate(&model, id, &wrong_type).is_err());
    }
    assert_eq!(
        evaluate(&model, "missing", &json!(true)).unwrap_err()[0].code,
        "OSM_ASSESSMENT"
    );
}
