//! Typed, round-trip-preserving authoring operations for package Sources.
//!
//! The renderer edits these domain DTOs, never raw package JSON. Stable IDs for
//! newly added entities are assigned here; existing JSON objects are mutated
//! in place so fields the current editor does not expose survive a save.

use crate::{LoadedSource, load_source};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use super::diagnostic;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "origin", content = "value", rename_all = "snake_case")]
pub enum DraftKey {
    Existing(String),
    New(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptDraft {
    pub key: DraftKey,
    pub title: String,
    #[serde(default)]
    pub requires: Vec<DraftKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveDraft {
    pub key: DraftKey,
    pub concept: DraftKey,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDraft {
    pub key: DraftKey,
    pub title: String,
    pub markdown: String,
    pub teaches: Vec<DraftKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumDraft {
    pub key: DraftKey,
    pub title: String,
    pub objectives: Vec<DraftKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionDraft {
    pub key: DraftKey,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssessmentResponseDraft {
    SingleSelect {
        options: Vec<OptionDraft>,
        answer: DraftKey,
    },
    Boolean {
        answer: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentDraft {
    pub key: DraftKey,
    pub measures: Vec<DraftKey>,
    pub stimulus: String,
    pub feedback: String,
    pub response: AssessmentResponseDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringWorkspace {
    pub source_directory: String,
    pub package_id: String,
    pub editable: bool,
    pub title: String,
    pub language: String,
    pub concepts: Vec<ConceptDraft>,
    pub objectives: Vec<ObjectiveDraft>,
    pub resources: Vec<ResourceDraft>,
    pub curricula: Vec<CurriculumDraft>,
    pub assessments: Vec<AssessmentDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEdits {
    pub source_directory: String,
    pub title: String,
    pub language: String,
    pub concepts: Vec<ConceptDraft>,
    pub objectives: Vec<ObjectiveDraft>,
    pub resources: Vec<ResourceDraft>,
    pub curricula: Vec<CurriculumDraft>,
    pub assessments: Vec<AssessmentDraft>,
}

fn mapped_key(
    key: &DraftKey,
    generated: &BTreeMap<String, String>,
) -> Result<String, Vec<osmium_core::schema::Diagnostic>> {
    match key {
        DraftKey::Existing(id) => Ok(id.clone()),
        DraftKey::New(key) => generated.get(key).cloned().ok_or_else(|| {
            diagnostic(
                "OSM_INIT_INVALID",
                format!("new entity reference is missing: {key}"),
                None,
            )
        }),
    }
}

fn allocate_key(
    key: &DraftKey,
    prefix: &str,
    generated: &mut BTreeMap<String, String>,
) -> Result<String, Vec<osmium_core::schema::Diagnostic>> {
    match key {
        DraftKey::Existing(id) => Ok(id.clone()),
        DraftKey::New(client_key) => {
            if generated.contains_key(client_key) {
                return Err(diagnostic(
                    "OSM_INIT_INVALID",
                    "new entity keys must be unique",
                    None,
                ));
            }
            if client_key.is_empty() || client_key.len() > 128 {
                return Err(diagnostic(
                    "OSM_INIT_INVALID",
                    "new entity key is invalid",
                    None,
                ));
            }
            let id = format!("{prefix}-{}", uuid::Uuid::new_v4().simple());
            generated.insert(client_key.clone(), id.clone());
            Ok(id)
        }
    }
}

fn entity_path(manifest: &Value, name: &str) -> String {
    manifest["entities"][name]
        .as_str()
        .unwrap_or(match name {
            "concepts" => "entities/concepts.json",
            "objectives" => "entities/objectives.json",
            "resources" => "entities/resources.json",
            "curricula" => "entities/curricula.json",
            _ => "entities/assessments.json",
        })
        .to_owned()
}

fn read_array(
    loaded: &LoadedSource,
    name: &str,
) -> Result<(String, Vec<Value>), Vec<osmium_core::schema::Diagnostic>> {
    let path = entity_path(&loaded.model.documents().manifest, name);
    let bytes = loaded.files.get(&path).ok_or_else(|| {
        diagnostic(
            "OSM_IO",
            format!("entity document is unavailable: {path}"),
            Some(path.clone()),
        )
    })?;
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| diagnostic("OSM_JSON", error, Some(path.clone())))?;
    let array = value.as_array().cloned().ok_or_else(|| {
        diagnostic(
            "OSM_JSON",
            format!("entity document must be an array: {path}"),
            Some(path.clone()),
        )
    })?;
    Ok((path, array))
}

fn text(value: &Value, field: &str, fallback: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

fn id(value: &Value) -> String {
    text(value, "id", "")
}

fn workspace_from_loaded(
    root: &Path,
    loaded: &LoadedSource,
) -> Result<AuthoringWorkspace, Vec<osmium_core::schema::Diagnostic>> {
    let manifest = &loaded.model.documents().manifest;
    let (_, concepts) = read_array(loaded, "concepts")?;
    let (_, objectives) = read_array(loaded, "objectives")?;
    let (_, resources) = read_array(loaded, "resources")?;
    let (_, curricula) = read_array(loaded, "curricula")?;
    let (_, assessments) = read_array(loaded, "assessments")?;
    let concepts = concepts
        .into_iter()
        .map(|item| ConceptDraft {
            key: DraftKey::Existing(id(&item)),
            title: text(&item, "title", ""),
            requires: item
                .get("requires")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|value| DraftKey::Existing(value.to_owned()))
                .collect(),
        })
        .collect();
    let objectives = objectives
        .into_iter()
        .map(|item| ObjectiveDraft {
            key: DraftKey::Existing(id(&item)),
            concept: DraftKey::Existing(text(&item, "concept", "")),
            description: text(&item, "description", ""),
        })
        .collect();
    let resources = resources
        .into_iter()
        .map(|item| {
            let path = text(&item, "path", "");
            let markdown = loaded
                .files
                .get(&path)
                .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
                .unwrap_or_default();
            ResourceDraft {
                key: DraftKey::Existing(id(&item)),
                title: text(&item, "title", ""),
                markdown,
                teaches: item
                    .get("teaches")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(|value| DraftKey::Existing(value.to_owned()))
                    .collect(),
            }
        })
        .collect();
    let curricula = curricula
        .into_iter()
        .map(|item| CurriculumDraft {
            key: DraftKey::Existing(id(&item)),
            title: text(&item, "title", ""),
            objectives: item
                .get("objectives")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|value| DraftKey::Existing(value.to_owned()))
                .collect(),
        })
        .collect();
    let assessments = assessments
        .into_iter()
        .map(|item| {
            let response = &item["response"];
            let response = match response["type"].as_str().unwrap_or_default() {
                "single_select" => {
                    let options = response["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|option| OptionDraft {
                            key: DraftKey::Existing(id(option)),
                            text: text(option, "text", ""),
                        })
                        .collect();
                    AssessmentResponseDraft::SingleSelect {
                        options,
                        answer: DraftKey::Existing(text(&item["evaluation"], "answer", "")),
                    }
                }
                _ => AssessmentResponseDraft::Boolean {
                    answer: item["evaluation"]["answer"].as_bool().unwrap_or(false),
                },
            };
            AssessmentDraft {
                key: DraftKey::Existing(id(&item)),
                measures: item
                    .get("measures")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(|value| DraftKey::Existing(value.to_owned()))
                    .collect(),
                stimulus: text(&item["stimulus"], "markdown", ""),
                feedback: text(&item["feedback"], "markdown", ""),
                response,
            }
        })
        .collect();
    Ok(AuthoringWorkspace {
        source_directory: root.to_string_lossy().into_owned(),
        package_id: text(manifest, "package_id", ""),
        editable: root.join("osmium.json").is_file(),
        title: text(manifest, "title", ""),
        language: text(manifest, "language", "en"),
        concepts,
        objectives,
        resources,
        curricula,
        assessments,
    })
}

pub fn open_workspace(
    path: impl AsRef<Path>,
) -> Result<AuthoringWorkspace, Vec<osmium_core::schema::Diagnostic>> {
    let root = path.as_ref();
    let loaded = load_source(root)?;
    workspace_from_loaded(root, &loaded)
}

fn put(object: &mut Value, field: &str, value: Value) {
    if !object.is_object() {
        *object = Value::Object(Map::new());
    }
    object[field] = value;
}

fn by_id(items: &[Value]) -> BTreeMap<String, Value> {
    items
        .iter()
        .cloned()
        .map(|item| (id(&item), item))
        .collect()
}

fn target_value(old: &BTreeMap<String, Value>, stable_id: &str) -> Value {
    old.get(stable_id)
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()))
}

/// Apply one typed workspace snapshot through Package/Core. New stable IDs are
/// generated here, existing object fields not exposed by the editor survive,
/// and the candidate Source is Core-validated before any source file changes.
pub fn save_workspace(
    edits: WorkspaceEdits,
) -> Result<AuthoringWorkspace, Vec<osmium_core::schema::Diagnostic>> {
    let input_root = PathBuf::from(&edits.source_directory);
    let loaded = load_source(&input_root)?;
    let root = input_root
        .canonicalize()
        .map_err(|error| diagnostic("OSM_IO", error, Some(edits.source_directory.clone())))?;
    if !root.join("osmium.json").is_file() {
        return Err(diagnostic(
            "OSM_EDITOR_FORMAT",
            "editing YAML manifests is unavailable because no Core YAML serializer exists",
            Some("osmium.yaml".into()),
        ));
    }
    let manifest_path = "osmium.json".to_owned();
    let mut manifest: Value =
        serde_json::from_slice(loaded.files.get(&manifest_path).ok_or_else(|| {
            diagnostic(
                "OSM_IO",
                "JSON manifest bytes are unavailable",
                Some(manifest_path.clone()),
            )
        })?)
        .map_err(|error| diagnostic("OSM_JSON", error, Some(manifest_path.clone())))?;
    put(&mut manifest, "title", Value::String(edits.title));
    put(&mut manifest, "language", Value::String(edits.language));
    let (concept_path, old_concepts) = read_array(&loaded, "concepts")?;
    let (objective_path, old_objectives) = read_array(&loaded, "objectives")?;
    let (resource_path, old_resources) = read_array(&loaded, "resources")?;
    let (curriculum_path, old_curricula) = read_array(&loaded, "curricula")?;
    let (assessment_path, old_assessments) = read_array(&loaded, "assessments")?;
    let old_concepts = by_id(&old_concepts);
    let old_objectives = by_id(&old_objectives);
    let old_resources = by_id(&old_resources);
    let old_curricula = by_id(&old_curricula);
    let old_assessments = by_id(&old_assessments);
    let mut generated = BTreeMap::new();
    for item in &edits.concepts {
        allocate_key(&item.key, "concept", &mut generated)?;
    }
    for item in &edits.objectives {
        allocate_key(&item.key, "objective", &mut generated)?;
    }
    for item in &edits.resources {
        allocate_key(&item.key, "resource", &mut generated)?;
    }
    for item in &edits.curricula {
        allocate_key(&item.key, "curriculum", &mut generated)?;
    }
    for item in &edits.assessments {
        allocate_key(&item.key, "assessment", &mut generated)?;
        if let AssessmentResponseDraft::SingleSelect { options, .. } = &item.response {
            for option in options {
                allocate_key(&option.key, "option", &mut generated)?;
            }
        }
    }
    let mut concepts = Vec::new();
    for item in &edits.concepts {
        let stable_id = mapped_key(&item.key, &generated)?;
        let mut value = target_value(&old_concepts, &stable_id);
        put(&mut value, "id", Value::String(stable_id));
        put(&mut value, "title", Value::String(item.title.clone()));
        put(
            &mut value,
            "requires",
            Value::Array(
                item.requires
                    .iter()
                    .map(|key| mapped_key(key, &generated).map(Value::String))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );
        concepts.push(value);
    }
    let mut objectives = Vec::new();
    for item in &edits.objectives {
        let stable_id = mapped_key(&item.key, &generated)?;
        let mut value = target_value(&old_objectives, &stable_id);
        put(&mut value, "id", Value::String(stable_id));
        put(
            &mut value,
            "concept",
            Value::String(mapped_key(&item.concept, &generated)?),
        );
        put(
            &mut value,
            "description",
            Value::String(item.description.clone()),
        );
        objectives.push(value);
    }
    let mut resources = Vec::new();
    let mut content_writes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for item in &edits.resources {
        let stable_id = mapped_key(&item.key, &generated)?;
        let mut value = target_value(&old_resources, &stable_id);
        let content_path = value["path"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("content/{stable_id}.md"));
        put(&mut value, "id", Value::String(stable_id));
        put(&mut value, "type", Value::String("markdown".into()));
        put(&mut value, "title", Value::String(item.title.clone()));
        put(&mut value, "path", Value::String(content_path.clone()));
        put(
            &mut value,
            "teaches",
            Value::Array(
                item.teaches
                    .iter()
                    .map(|key| mapped_key(key, &generated).map(Value::String))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );
        content_writes.insert(content_path, item.markdown.as_bytes().to_vec());
        resources.push(value);
    }
    let mut curricula = Vec::new();
    for item in &edits.curricula {
        let stable_id = mapped_key(&item.key, &generated)?;
        let mut value = target_value(&old_curricula, &stable_id);
        put(&mut value, "id", Value::String(stable_id));
        put(&mut value, "title", Value::String(item.title.clone()));
        put(
            &mut value,
            "objectives",
            Value::Array(
                item.objectives
                    .iter()
                    .map(|key| mapped_key(key, &generated).map(Value::String))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );
        curricula.push(value);
    }
    let mut assessments = Vec::new();
    for item in &edits.assessments {
        let stable_id = mapped_key(&item.key, &generated)?;
        let mut value = target_value(&old_assessments, &stable_id);
        put(&mut value, "id", Value::String(stable_id));
        if value.get("revision").is_none() {
            put(&mut value, "revision", Value::String("1".into()));
        }
        put(
            &mut value,
            "measures",
            Value::Array(
                item.measures
                    .iter()
                    .map(|key| mapped_key(key, &generated).map(Value::String))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );
        put(&mut value, "stimulus", json!({"markdown": item.stimulus}));
        put(&mut value, "feedback", json!({"markdown": item.feedback}));
        let (response, answer) = match &item.response {
            AssessmentResponseDraft::SingleSelect { options, answer } => {
                let values = options
                    .iter()
                    .map(|option| {
                        Ok(json!({"id": mapped_key(&option.key, &generated)?, "text": option.text}))
                    })
                    .collect::<Result<Vec<_>, Vec<osmium_core::schema::Diagnostic>>>()?;
                (
                    json!({"type":"single_select","options":values}),
                    Value::String(mapped_key(answer, &generated)?),
                )
            }
            AssessmentResponseDraft::Boolean { answer } => {
                (json!({"type":"boolean"}), Value::Bool(*answer))
            }
        };
        put(&mut value, "response", response);
        put(
            &mut value,
            "evaluation",
            json!({"type":"exact","answer":answer}),
        );
        assessments.push(value);
    }
    let mut candidate_files = loaded.files.clone();
    candidate_files.insert(
        manifest_path.clone(),
        serde_json::to_vec_pretty(&manifest)
            .map_err(|error| diagnostic("OSM_JSON", error, Some(manifest_path.clone())))?,
    );
    for (path, values) in [
        (&concept_path, &concepts),
        (&objective_path, &objectives),
        (&resource_path, &resources),
        (&curriculum_path, &curricula),
        (&assessment_path, &assessments),
    ] {
        candidate_files.insert(
            path.clone(),
            serde_json::to_vec_pretty(values)
                .map_err(|error| diagnostic("OSM_JSON", error, Some(path.clone())))?,
        );
    }
    candidate_files.extend(content_writes.clone());
    let candidate = tempfile::tempdir().map_err(|error| diagnostic("OSM_IO", error, None))?;
    for (name, bytes) in &candidate_files {
        let target = candidate.path().join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| diagnostic("OSM_IO", error, Some(parent.display().to_string())))?;
        }
        fs::write(&target, bytes)
            .map_err(|error| diagnostic("OSM_IO", error, Some(target.display().to_string())))?;
    }
    load_source(candidate.path())?;
    let mut writes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    writes.insert(
        manifest_path,
        serde_json::to_vec_pretty(&manifest)
            .map_err(|error| diagnostic("OSM_JSON", error, None))?,
    );
    writes.insert(
        concept_path,
        serde_json::to_vec_pretty(&concepts)
            .map_err(|error| diagnostic("OSM_JSON", error, None))?,
    );
    writes.insert(
        objective_path,
        serde_json::to_vec_pretty(&objectives)
            .map_err(|error| diagnostic("OSM_JSON", error, None))?,
    );
    writes.insert(
        resource_path,
        serde_json::to_vec_pretty(&resources)
            .map_err(|error| diagnostic("OSM_JSON", error, None))?,
    );
    writes.insert(
        curriculum_path,
        serde_json::to_vec_pretty(&curricula)
            .map_err(|error| diagnostic("OSM_JSON", error, None))?,
    );
    writes.insert(
        assessment_path,
        serde_json::to_vec_pretty(&assessments)
            .map_err(|error| diagnostic("OSM_JSON", error, None))?,
    );
    writes.extend(content_writes);
    for (name, bytes) in writes {
        let target = root.join(&name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| diagnostic("OSM_IO", error, Some(parent.display().to_string())))?;
        }
        fs::write(&target, bytes)
            .map_err(|error| diagnostic("OSM_IO", error, Some(target.display().to_string())))?;
    }
    open_workspace(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().expect("temporary root");
        let root = temp.path().join("course");
        crate::init::init_source(&crate::init::InitRequest {
            directory: root.clone(),
            package_id: "org.example/course".into(),
            title: Some("Course".into()),
            language: "en".into(),
        })
        .expect("scaffold");
        (temp, root)
    }

    fn existing(id: &str) -> DraftKey {
        DraftKey::Existing(id.into())
    }
    fn new(key: &str) -> DraftKey {
        DraftKey::New(key.into())
    }

    #[test]
    fn multi_entity_edits_round_trip_relations_and_curriculum_order() {
        let (_temp, root) = source();
        let mut workspace = open_workspace(&root).expect("open workspace");
        workspace.concepts.push(ConceptDraft {
            key: new("concept-two"),
            title: "Subtraction".into(),
            requires: vec![existing("example.concept")],
        });
        workspace.objectives.push(ObjectiveDraft {
            key: new("objective-two"),
            concept: new("concept-two"),
            description: "Subtract small integers".into(),
        });
        workspace.resources.push(ResourceDraft {
            key: new("resource-two"),
            title: "Subtraction lesson".into(),
            markdown: "# Subtraction\n".into(),
            teaches: vec![new("objective-two")],
        });
        workspace.curricula[0].objectives =
            vec![new("objective-two"), existing("example.objective")];
        workspace.assessments.push(AssessmentDraft {
            key: new("assessment-two"),
            measures: vec![new("objective-two")],
            stimulus: "3 - 1 = ?".into(),
            feedback: "Subtract one at a time.".into(),
            response: AssessmentResponseDraft::SingleSelect {
                options: vec![
                    OptionDraft {
                        key: new("option-one"),
                        text: "1".into(),
                    },
                    OptionDraft {
                        key: new("option-two"),
                        text: "2".into(),
                    },
                ],
                answer: new("option-two"),
            },
        });
        workspace.assessments.push(AssessmentDraft {
            key: new("assessment-three"),
            measures: vec![new("objective-two")],
            stimulus: "3 - 1 is 2.".into(),
            feedback: "Yes.".into(),
            response: AssessmentResponseDraft::Boolean { answer: true },
        });
        let saved = save_workspace(WorkspaceEdits {
            source_directory: workspace.source_directory,
            title: workspace.title,
            language: workspace.language,
            concepts: workspace.concepts,
            objectives: workspace.objectives,
            resources: workspace.resources,
            curricula: workspace.curricula,
            assessments: workspace.assessments,
        })
        .expect("save multi entity workspace");
        assert_eq!(saved.concepts.len(), 2);
        assert_eq!(saved.objectives.len(), 2);
        assert_eq!(saved.resources.len(), 2);
        assert_eq!(saved.curricula[0].objectives[0], saved.objectives[1].key);
        assert_eq!(saved.resources[1].teaches[0], saved.objectives[1].key);
        assert!(matches!(
            saved.assessments[1].response,
            AssessmentResponseDraft::SingleSelect { .. }
        ));
        assert!(matches!(
            saved.assessments[2].response,
            AssessmentResponseDraft::Boolean { answer: true }
        ));
        let loaded = crate::load_source(&root).expect("saved source still validates");
        assert_eq!(
            loaded.model.documents().resources.as_array().unwrap()[1]["teaches"][0],
            loaded.model.documents().objectives.as_array().unwrap()[1]["id"]
        );
    }

    #[test]
    fn edits_preserve_unexposed_fields_and_reject_dangling_or_cyclic_graphs() {
        let (_temp, root) = source();
        let mut raw: Value =
            serde_json::from_slice(&fs::read(root.join("entities/resources.json")).unwrap())
                .unwrap();
        raw[0]["extensions"] = json!({"org.example.editor.v1":{"keep":true}});
        fs::write(
            root.join("entities/resources.json"),
            serde_json::to_vec_pretty(&raw).unwrap(),
        )
        .unwrap();
        let mut workspace = open_workspace(&root).expect("open source");
        workspace.resources[0].title = "Changed title".into();
        let saved = save_workspace(WorkspaceEdits {
            source_directory: workspace.source_directory.clone(),
            title: workspace.title.clone(),
            language: workspace.language.clone(),
            concepts: workspace.concepts.clone(),
            objectives: workspace.objectives.clone(),
            resources: workspace.resources.clone(),
            curricula: workspace.curricula.clone(),
            assessments: workspace.assessments.clone(),
        })
        .expect("save known fields");
        let raw: Value =
            serde_json::from_slice(&fs::read(root.join("entities/resources.json")).unwrap())
                .unwrap();
        assert_eq!(raw[0]["extensions"]["org.example.editor.v1"]["keep"], true);
        assert_eq!(saved.resources[0].title, "Changed title");

        let before = fs::read(root.join("entities/concepts.json")).unwrap();
        let mut dangling = open_workspace(&root).expect("open source");
        dangling
            .concepts
            .retain(|item| item.key != existing("example.concept"));
        assert!(
            save_workspace(WorkspaceEdits {
                source_directory: dangling.source_directory,
                title: dangling.title,
                language: dangling.language,
                concepts: dangling.concepts,
                objectives: dangling.objectives,
                resources: dangling.resources,
                curricula: dangling.curricula,
                assessments: dangling.assessments,
            })
            .is_err()
        );
        assert_eq!(
            fs::read(root.join("entities/concepts.json")).unwrap(),
            before
        );

        let mut cyclic = open_workspace(&root).expect("open source");
        cyclic.concepts[0].requires = vec![existing("example.concept")];
        assert!(
            save_workspace(WorkspaceEdits {
                source_directory: cyclic.source_directory,
                title: cyclic.title,
                language: cyclic.language,
                concepts: cyclic.concepts,
                objectives: cyclic.objectives,
                resources: cyclic.resources,
                curricula: cyclic.curricula,
                assessments: cyclic.assessments,
            })
            .is_err()
        );
        assert_eq!(
            fs::read(root.join("entities/concepts.json")).unwrap(),
            before
        );
    }

    #[test]
    fn invalid_assessment_edits_are_rejected_before_source_writes() {
        let (_temp, root) = source();
        let workspace = open_workspace(&root).expect("open source");
        let before = fs::read(root.join("entities/assessments.json")).unwrap();
        let mut edits = WorkspaceEdits {
            source_directory: workspace.source_directory,
            title: workspace.title,
            language: workspace.language,
            concepts: workspace.concepts,
            objectives: workspace.objectives,
            resources: workspace.resources,
            curricula: workspace.curricula,
            assessments: workspace.assessments,
        };
        edits.assessments.push(AssessmentDraft {
            key: new("invalid-assessment"),
            measures: vec![existing("example.objective")],
            stimulus: "Choose one".into(),
            feedback: "Try again".into(),
            response: AssessmentResponseDraft::SingleSelect {
                options: vec![OptionDraft {
                    key: new("only-option"),
                    text: "One".into(),
                }],
                answer: new("only-option"),
            },
        });
        let errors = save_workspace(edits).expect_err("Core must reject the invalid item");
        assert!(
            !errors.is_empty(),
            "Core must return a structured diagnostic"
        );
        assert_eq!(
            fs::read(root.join("entities/assessments.json")).unwrap(),
            before
        );
    }
}
