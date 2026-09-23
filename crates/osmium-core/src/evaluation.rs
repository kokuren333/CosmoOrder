//! Deterministic, side-effect-free assessment evaluation. No package code runs.

use crate::{schema::Diagnostic, validation::PackageModel};
use serde::Serialize;
use serde_json::Value;

pub const EVALUATOR_ID: &str = "org.osmium.exact.v1";
pub const EVALUATOR_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Evaluation {
    pub assessment_id: String,
    pub assessment_revision: String,
    pub objective_ids: Vec<String>,
    pub concept_ids: Vec<String>,
    pub assessment_snapshot: Value,
    pub response: Value,
    pub score: u8,
    pub correct: bool,
    pub feedback: Value,
    pub evaluator: Evaluator,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Evaluator {
    pub id: &'static str,
    pub version: &'static str,
}

fn failure(code: &str, message: &str) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: code.into(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file: None,
        line: None,
        column: None,
        path: "/response".into(),
        message: message.into(),
        suggestions: Vec::new(),
    }]
}

pub fn evaluate(
    model: &PackageModel,
    assessment_id: &str,
    response: &Value,
) -> Result<Evaluation, Vec<Diagnostic>> {
    let documents = model.documents();
    let assessment = documents
        .assessments
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == assessment_id)
        .ok_or_else(|| failure("OSM_ASSESSMENT", "assessment does not exist"))?;
    match assessment["response"]["type"].as_str() {
        Some("single_select") => {
            if !response.is_string()
                || !assessment["response"]["options"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|option| option["id"] == *response)
            {
                return Err(failure(
                    "OSM_RESPONSE",
                    "response must be an available option ID",
                ));
            }
        }
        Some("boolean") if response.is_boolean() => {}
        Some("boolean") => return Err(failure("OSM_RESPONSE", "response must be a JSON boolean")),
        _ => {
            return Err(failure(
                "OSM_CAPABILITY",
                "unsupported assessment response type",
            ));
        }
    }
    let objective_ids: Vec<String> = assessment["measures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    let mut concept_ids = Vec::new();
    for objective_id in &objective_ids {
        let objective = documents
            .objectives
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == *objective_id)
            .unwrap();
        let concept = objective["concept"].as_str().unwrap().to_owned();
        if !concept_ids.contains(&concept) {
            concept_ids.push(concept);
        }
    }
    let correct = *response == assessment["evaluation"]["answer"];
    Ok(Evaluation {
        assessment_id: assessment_id.into(),
        assessment_revision: assessment["revision"].as_str().unwrap().into(),
        objective_ids,
        concept_ids,
        assessment_snapshot: assessment.clone(),
        response: response.clone(),
        score: u8::from(correct),
        correct,
        feedback: assessment["feedback"].clone(),
        evaluator: Evaluator {
            id: EVALUATOR_ID,
            version: EVALUATOR_VERSION,
        },
    })
}
