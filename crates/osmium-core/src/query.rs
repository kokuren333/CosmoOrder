//! Read-only semantic views over a validated package model.
//!
//! These operations build the DTOs that CLI, Desktop IPC and future MCP clients
//! share. They never touch a filesystem, never re-run schema validation and
//! never decide package validity: only [`crate::validation::validate_package`]
//! constructs the model they consume.
//!
//! Every view is bounded. Package content is untrusted data: callers must not
//! treat returned Markdown or titles as instructions.

use crate::schema::Diagnostic;
use crate::validation::PackageModel;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Upper bound for a single `query` page.
pub const MAX_QUERY_LIMIT: usize = 256;
/// Maximum number of matches returned by a Package-local Concept search.
pub const MAX_CONCEPT_SEARCH_RESULTS: usize = 20;
/// Upper bound for the entity kinds a single `context` traversal may reach.
pub const MAX_CONTEXT_NODES: usize = 512;
/// Upper bound for the serialized entity bytes a `context` result may carry.
pub const MAX_CONTEXT_BYTES: usize = 256 * 1024;
/// Upper bound for the relation hops a `context` traversal may follow.
pub const MAX_CONTEXT_DEPTH: usize = 8;
/// Upper bound for the prerequisite ordering reported by `inspect`.
pub const MAX_PREREQUISITE_ORDER: usize = 64;

/// Package entity kinds a client can address. `manifest` is not an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Concept,
    Objective,
    Curriculum,
    Resource,
    Assessment,
}

impl EntityKind {
    pub const ALL: [Self; 5] = [
        Self::Concept,
        Self::Objective,
        Self::Curriculum,
        Self::Resource,
        Self::Assessment,
    ];

    /// Singular kind name, stable in machine output.
    pub fn name(self) -> &'static str {
        match self {
            Self::Concept => "concept",
            Self::Objective => "objective",
            Self::Curriculum => "curriculum",
            Self::Resource => "resource",
            Self::Assessment => "assessment",
        }
    }

    /// Entity document key used by the manifest and by diagnostics.
    pub fn document(self) -> &'static str {
        match self {
            Self::Concept => "concepts",
            Self::Objective => "objectives",
            Self::Curriculum => "curricula",
            Self::Resource => "resources",
            Self::Assessment => "assessments",
        }
    }

    /// Parse a user-supplied kind, accepting singular and plural spellings.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == value || kind.document() == value)
    }
}

/// A bounded per-entity summary. `title` falls back to the ID when an entity
/// kind has no title field, and `detail` carries only kind-specific counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntitySummary {
    pub id: String,
    pub kind: EntityKind,
    pub title: String,
    pub detail: Value,
}

/// Package-level metadata. Paths and payload hashes are not included because
/// they are not resolved by semantic validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestView {
    pub schema_version: String,
    pub package_id: String,
    pub package_version: String,
    pub title: String,
    pub language: String,
    pub capabilities: CapabilitiesView,
    pub entity_counts: BTreeMap<String, usize>,
    pub prerequisite_order: Vec<String>,
    pub prerequisite_order_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilitiesView {
    /// Declared capabilities. Format 0.1 implements none of them, so a
    /// non-empty `required` list is also a validation error.
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InspectView {
    pub manifest: ManifestView,
    pub total_entities: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryView {
    pub kind: EntityKind,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub returned: usize,
    pub truncated: bool,
    pub entities: Vec<EntitySummary>,
}

/// Bounded title search over Concepts in one Package. Results remain in the
/// author's document order; callers can narrow a broad query rather than
/// receiving the entire Concept collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConceptSearchView {
    pub query: String,
    pub total: usize,
    pub limit: usize,
    pub results: Vec<EntitySummary>,
    pub truncated: bool,
}

/// One entity reached by a context traversal, with its full untrusted payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextNode {
    pub id: String,
    pub kind: EntityKind,
    pub title: String,
    /// Shortest hop count from the traversal target. The target is 0.
    pub depth: usize,
    pub entity: Value,
}

/// One typed relationship between two entities in the traversal result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextRelation {
    pub relation: String,
    pub from_kind: EntityKind,
    pub from_id: String,
    pub to_kind: EntityKind,
    pub to_id: String,
    /// True when the relationship is expressed on the `to` entity's field,
    /// for example `addition` is required by `carry`.
    pub incoming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextView {
    pub target: EntitySummary,
    pub depth: usize,
    pub nodes: Vec<ContextNode>,
    pub relations: Vec<ContextRelation>,
    pub truncated: bool,
    /// Number of Concepts in the transitive `requires` closure below the
    /// target, for a Concept target only. `0` means no prerequisite.
    pub prerequisite_depth: Option<usize>,
    /// Always true: callers must treat payload text as data, not instructions.
    pub content_is_untrusted: bool,
}

/// A deterministic key for one entity inside a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NodeKey<'a> {
    kind: EntityKind,
    id: &'a str,
}

/// A directed edge. `relation` is the field name that expresses it, and
/// `incoming` states whether the payload field belongs to the other entity.
struct Edge<'a> {
    relation: &'static str,
    other: NodeKey<'a>,
    incoming: bool,
}

struct Graph<'a> {
    adjacency: BTreeMap<NodeKey<'a>, Vec<Edge<'a>>>,
}

impl<'a> Graph<'a> {
    fn link(&mut self, from: NodeKey<'a>, to: NodeKey<'a>, relation: &'static str) {
        self.adjacency.entry(from).or_default().push(Edge {
            relation,
            other: to,
            incoming: false,
        });
        self.adjacency.entry(to).or_default().push(Edge {
            relation,
            other: from,
            incoming: true,
        });
    }

    fn neighbors(
        &self,
        key: NodeKey<'a>,
        relation: &'static str,
        incoming: bool,
    ) -> Vec<NodeKey<'a>> {
        self.adjacency
            .get(&key)
            .into_iter()
            .flatten()
            .filter(|edge| edge.relation == relation && edge.incoming == incoming)
            .map(|edge| edge.other)
            .collect()
    }
}

fn entities(model: &PackageModel, kind: EntityKind) -> &[Value] {
    let documents = model.documents();
    let value = match kind {
        EntityKind::Concept => &documents.concepts,
        EntityKind::Objective => &documents.objectives,
        EntityKind::Curriculum => &documents.curricula,
        EntityKind::Resource => &documents.resources,
        EntityKind::Assessment => &documents.assessments,
    };
    value
        .as_array()
        .expect("a validated model holds entity arrays")
}

fn entity_index(model: &PackageModel, kind: EntityKind) -> BTreeMap<&str, &Value> {
    entities(model, kind)
        .iter()
        .map(|entity| {
            (
                entity["id"]
                    .as_str()
                    .expect("a validated entity has a string ID"),
                entity,
            )
        })
        .collect()
}

/// Resolve a stable entity ID without a declared kind.
///
/// IDs are unique per kind, not per package, so an ambiguous ID is an error
/// rather than an arbitrary choice.
pub fn resolve_target<'a>(
    model: &'a PackageModel,
    id: &str,
) -> Result<(EntityKind, &'a Value), Vec<Diagnostic>> {
    let matches: Vec<(EntityKind, &Value)> = EntityKind::ALL
        .into_iter()
        .filter_map(|kind| {
            entity_index(model, kind)
                .get(id)
                .map(|entity| (kind, *entity))
        })
        .collect();
    match matches.as_slice() {
        [] => Err(vec![error(
            "package",
            "OSM_UNKNOWN_ENTITY",
            format!("no entity has the ID: {id}"),
        )]),
        [(kind, entity)] => Ok((*kind, entity)),
        many => {
            let kinds: Vec<&str> = many.iter().map(|(kind, _)| kind.name()).collect();
            Err(vec![error(
                "package",
                "OSM_AMBIGUOUS_ENTITY",
                format!(
                    "ID {id} exists as more than one kind ({}); declare --kind",
                    kinds.join(", ")
                ),
            )])
        }
    }
}

fn error(file: &str, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file: Some(file.into()),
        line: None,
        column: None,
        path: String::new(),
        message: message.into(),
        suggestions: Vec::new(),
    }
}

fn required_array<'a>(entity: &'a Value, field: &str) -> &'a [Value] {
    entity[field]
        .as_array()
        .map(Vec::as_slice)
        .expect("a validated entity holds the declared array field")
}

fn required_str<'a>(entity: &'a Value, field: &str) -> &'a str {
    entity[field]
        .as_str()
        .expect("a validated entity holds the declared string field")
}

fn string_list(entity: &Value, field: &str) -> Vec<String> {
    string_refs(entity, field)
        .iter()
        .map(|entry| (*entry).to_owned())
        .collect()
}

/// Borrowed string entries of a validated string array. The returned slices
/// point into the model, so they are usable as graph keys.
fn string_refs<'a>(entity: &'a Value, field: &str) -> Vec<&'a str> {
    required_array(entity, field)
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .expect("a validated string array holds only strings")
        })
        .collect()
}

/// A human-facing label for one entity. Objectives describe a capability
/// rather than carrying a title, and Assessments have no text field at all, so
/// the stable ID is the honest fallback.
fn title_of(kind: EntityKind, entity: &Value) -> String {
    match kind {
        EntityKind::Objective => required_str(entity, "description").to_owned(),
        EntityKind::Assessment => required_str(entity, "id").to_owned(),
        _ => required_str(entity, "title").to_owned(),
    }
}

fn summary(kind: EntityKind, entity: &Value) -> EntitySummary {
    let detail = match kind {
        EntityKind::Concept => serde_json::json!({
            "requires": string_list(entity, "requires"),
        }),
        EntityKind::Objective => serde_json::json!({
            "concept": required_str(entity, "concept"),
        }),
        EntityKind::Curriculum => serde_json::json!({
            "objective_count": required_array(entity, "objectives").len(),
        }),
        EntityKind::Resource => serde_json::json!({
            "path": required_str(entity, "path"),
            "teaches": string_list(entity, "teaches"),
        }),
        EntityKind::Assessment => serde_json::json!({
            "revision": required_str(entity, "revision"),
            "response_type": required_str(&entity["response"], "type"),
            "measures": string_list(entity, "measures"),
        }),
    };
    EntitySummary {
        id: required_str(entity, "id").to_owned(),
        kind,
        title: title_of(kind, entity),
        detail,
    }
}

fn manifest_view(model: &PackageModel, prerequisite_limit: usize) -> ManifestView {
    let manifest = &model.documents().manifest;
    let order = model.prerequisite_order();
    let limit = prerequisite_limit.min(MAX_PREREQUISITE_ORDER);
    ManifestView {
        schema_version: required_str(manifest, "schema_version").to_owned(),
        package_id: required_str(manifest, "package_id").to_owned(),
        package_version: required_str(manifest, "package_version").to_owned(),
        title: required_str(manifest, "title").to_owned(),
        language: required_str(manifest, "language").to_owned(),
        capabilities: CapabilitiesView {
            required: string_list(&manifest["capabilities"], "required"),
            optional: string_list(&manifest["capabilities"], "optional"),
        },
        entity_counts: EntityKind::ALL
            .into_iter()
            .map(|kind| (kind.document().to_owned(), entities(model, kind).len()))
            .collect(),
        prerequisite_order: order.iter().take(limit).cloned().collect(),
        prerequisite_order_truncated: order.len() > limit,
    }
}

/// Package metadata and bounded entity counts.
pub fn inspect(model: &PackageModel, prerequisite_limit: usize) -> InspectView {
    let manifest = manifest_view(model, prerequisite_limit);
    let total_entities = manifest.entity_counts.values().sum();
    InspectView {
        manifest,
        total_entities,
    }
}

/// Page through one entity kind in document order.
///
/// A limit above [`MAX_QUERY_LIMIT`] is rejected instead of silently clamped so
/// callers cannot believe they received an unbounded result.
pub fn query(
    model: &PackageModel,
    kind: EntityKind,
    offset: usize,
    limit: usize,
) -> Result<QueryView, Vec<Diagnostic>> {
    if limit == 0 || limit > MAX_QUERY_LIMIT {
        return Err(vec![error(
            "cli",
            "OSM_QUERY_LIMIT",
            format!("limit must be between 1 and {MAX_QUERY_LIMIT}"),
        )]);
    }
    let all = entities(model, kind);
    let page: Vec<EntitySummary> = all
        .iter()
        .skip(offset)
        .take(limit)
        .map(|entity| summary(kind, entity))
        .collect();
    let returned = page.len();
    Ok(QueryView {
        kind,
        offset,
        limit,
        total: all.len(),
        returned,
        truncated: offset + returned < all.len(),
        entities: page,
    })
}

/// Search Concept titles within one validated Package, returning a small
/// deterministic result set suitable for recentering a local Atlas view.
pub fn search_concepts(
    model: &PackageModel,
    needle: &str,
    limit: usize,
) -> Result<ConceptSearchView, Vec<Diagnostic>> {
    if limit == 0 || limit > MAX_CONCEPT_SEARCH_RESULTS || needle.chars().count() > 128 {
        return Err(vec![error(
            "cli",
            "OSM_QUERY_LIMIT",
            format!(
                "search requires 1..={MAX_CONCEPT_SEARCH_RESULTS} results and at most 128 query characters"
            ),
        )]);
    }
    let query = needle.trim();
    if query.is_empty() {
        return Ok(ConceptSearchView {
            query: String::new(),
            total: 0,
            limit,
            results: Vec::new(),
            truncated: false,
        });
    }
    let normalized = query.to_lowercase();
    let matches = entities(model, EntityKind::Concept)
        .iter()
        .filter(|entity| {
            required_str(entity, "title")
                .to_lowercase()
                .contains(&normalized)
        });
    let mut total = 0;
    let mut results = Vec::new();
    for entity in matches {
        total += 1;
        if results.len() < limit {
            results.push(summary(EntityKind::Concept, entity));
        }
    }
    Ok(ConceptSearchView {
        query: query.to_owned(),
        total,
        limit,
        truncated: total > results.len(),
        results,
    })
}

fn build_graph(model: &PackageModel) -> Graph<'_> {
    let mut graph = Graph {
        adjacency: BTreeMap::new(),
    };
    for concept in entities(model, EntityKind::Concept) {
        let id = required_str(concept, "id");
        // Every relation points from the entity whose field declares it to the
        // entity that field names. A Concept therefore points at the Concepts
        // it requires.
        for required in string_refs(concept, "requires") {
            graph.link(
                NodeKey {
                    kind: EntityKind::Concept,
                    id,
                },
                NodeKey {
                    kind: EntityKind::Concept,
                    id: required,
                },
                "requires",
            );
        }
    }
    for objective in entities(model, EntityKind::Objective) {
        graph.link(
            NodeKey {
                kind: EntityKind::Objective,
                id: required_str(objective, "id"),
            },
            NodeKey {
                kind: EntityKind::Concept,
                id: required_str(objective, "concept"),
            },
            "concept",
        );
    }
    for resource in entities(model, EntityKind::Resource) {
        for objective in string_refs(resource, "teaches") {
            graph.link(
                NodeKey {
                    kind: EntityKind::Resource,
                    id: required_str(resource, "id"),
                },
                NodeKey {
                    kind: EntityKind::Objective,
                    id: objective,
                },
                "teaches",
            );
        }
    }
    for assessment in entities(model, EntityKind::Assessment) {
        for objective in string_refs(assessment, "measures") {
            graph.link(
                NodeKey {
                    kind: EntityKind::Assessment,
                    id: required_str(assessment, "id"),
                },
                NodeKey {
                    kind: EntityKind::Objective,
                    id: objective,
                },
                "measures",
            );
        }
    }
    for curriculum in entities(model, EntityKind::Curriculum) {
        for objective in string_refs(curriculum, "objectives") {
            graph.link(
                NodeKey {
                    kind: EntityKind::Curriculum,
                    id: required_str(curriculum, "id"),
                },
                NodeKey {
                    kind: EntityKind::Objective,
                    id: objective,
                },
                "orders",
            );
        }
    }
    graph
}

fn entity_for<'a>(model: &'a PackageModel, key: NodeKey<'_>) -> &'a Value {
    entity_index(model, key.kind)[key.id]
}

/// Bounded breadth-first traversal from one entity.
///
/// Traversal is deterministic: entities are expanded in the order they are
/// stored. The target node is always present; once the node or byte budget is
/// exhausted the result is marked `truncated` and no further entity is added.
pub fn context(
    model: &PackageModel,
    kind: EntityKind,
    id: &str,
    depth: usize,
    node_limit: usize,
) -> Result<ContextView, Vec<Diagnostic>> {
    if depth == 0 || depth > MAX_CONTEXT_DEPTH {
        return Err(vec![error(
            "cli",
            "OSM_CONTEXT_DEPTH",
            format!("depth must be between 1 and {MAX_CONTEXT_DEPTH}"),
        )]);
    }
    if node_limit == 0 || node_limit > MAX_CONTEXT_NODES {
        return Err(vec![error(
            "cli",
            "OSM_CONTEXT_LIMIT",
            format!("node limit must be between 1 and {MAX_CONTEXT_NODES}"),
        )]);
    }
    let target = entity_index(model, kind).get(id).copied().ok_or_else(|| {
        vec![error(
            "package",
            "OSM_UNKNOWN_ENTITY",
            format!("no {} has the ID: {id}", kind.name()),
        )]
    })?;
    if target.to_string().len() > MAX_CONTEXT_BYTES {
        return Err(vec![error(
            "package",
            "OSM_CONTEXT_BYTES",
            "target exceeds context byte limit; inspect or query a summary instead",
        )]);
    }
    let graph = build_graph(model);
    let root = NodeKey { kind, id };
    let mut nodes = vec![ContextNode {
        id: id.to_owned(),
        kind,
        title: title_of(kind, target),
        depth: 0,
        entity: target.clone(),
    }];
    let mut relations = Vec::new();
    let mut visited: BTreeSet<NodeKey<'_>> = BTreeSet::from([root]);
    let mut queue: VecDeque<(NodeKey<'_>, usize)> = VecDeque::from([(root, 0)]);
    let mut bytes = target.to_string().len();
    let mut truncated = false;
    while let Some((current, current_depth)) = queue.pop_front() {
        if let Some(edges) = graph.adjacency.get(&current) {
            for edge in edges {
                relations.push(ContextRelation {
                    relation: edge.relation.to_owned(),
                    from_kind: current.kind,
                    from_id: current.id.to_owned(),
                    to_kind: edge.other.kind,
                    to_id: edge.other.id.to_owned(),
                    incoming: edge.incoming,
                });
            }
        }
        if current_depth == depth {
            continue;
        }
        let mut neighbors = Vec::new();
        for relation in ["requires", "concept", "teaches", "measures", "orders"] {
            for incoming in [false, true] {
                neighbors.extend(graph.neighbors(current, relation, incoming));
            }
        }
        for neighbor in neighbors {
            if !visited.insert(neighbor) {
                continue;
            }
            let entity = entity_for(model, neighbor);
            let size = entity.to_string().len();
            if nodes.len() >= node_limit || bytes + size > MAX_CONTEXT_BYTES {
                truncated = true;
                break;
            }
            bytes += size;
            nodes.push(ContextNode {
                id: neighbor.id.to_owned(),
                kind: neighbor.kind,
                title: title_of(neighbor.kind, entity),
                depth: current_depth + 1,
                entity: entity.clone(),
            });
            queue.push_back((neighbor, current_depth + 1));
        }
        if truncated {
            break;
        }
    }
    // How many Concepts a learner must already hold before this one: the size
    // of the transitive `requires` closure below the target. The length of the
    // closure rather than its depth, because a branch of two prerequisites
    // means two prerequisites.
    let prerequisite_depth = if kind == EntityKind::Concept {
        let mut reached: BTreeSet<&str> = BTreeSet::from([id]);
        let mut frontier: Vec<&str> = vec![id];
        let mut count = 0usize;
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for concept in &frontier {
                for required in graph.neighbors(
                    NodeKey {
                        kind: EntityKind::Concept,
                        id: concept,
                    },
                    "requires",
                    false,
                ) {
                    if reached.insert(required.id) {
                        count += 1;
                        next.push(required.id);
                    }
                }
            }
            frontier = next;
        }
        Some(count)
    } else {
        None
    };
    let view = ContextView {
        target: summary(kind, target),
        depth,
        nodes,
        relations,
        truncated,
        prerequisite_depth,
        content_is_untrusted: true,
    };
    if serde_json::to_vec(&view)
        .expect("context DTO is JSON serializable")
        .len()
        > MAX_CONTEXT_BYTES
    {
        return Err(vec![error(
            "package",
            "OSM_CONTEXT_BYTES",
            "context exceeds byte limit; request a smaller depth or limit",
        )]);
    }
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{PackageDocuments, validate_package};
    use serde_json::json;

    #[test]
    fn context_rejects_an_oversized_target_extension() {
        let mut docs = model().documents().clone();
        let id = docs.concepts[0]["id"].as_str().unwrap().to_owned();
        docs.concepts[0]["extensions"] =
            json!({"org.example.large.v1": "x".repeat(MAX_CONTEXT_BYTES)});
        let model = validate_package(docs).unwrap();
        assert_eq!(
            context(&model, EntityKind::Concept, &id, 1, 1).unwrap_err()[0].code,
            "OSM_CONTEXT_BYTES"
        );
    }

    fn model() -> PackageModel {
        validate_package(PackageDocuments {
            manifest: json!({
                "schema_version": "0.1",
                "package_id": "org.example/arithmetic",
                "package_version": "0.1.0",
                "title": "足し算の基礎",
                "language": "ja-JP",
                "capabilities": { "required": [], "optional": ["org.osmium.media.v1"] },
                "entities": {
                    "concepts": "entities/concepts.json",
                    "objectives": "entities/objectives.json",
                    "curricula": "entities/curricula.json",
                    "resources": "entities/resources.json",
                    "assessments": "entities/assessments.json"
                },
                "extensions": {}
            }),
            concepts: json!([
                { "id": "addition", "title": "足し算", "requires": [] },
                { "id": "carry", "title": "繰り上がり", "requires": ["addition"] }
            ]),
            objectives: json!([
                { "id": "addition.basic", "concept": "addition", "description": "1桁を足せる" },
                { "id": "carry.basic", "concept": "carry", "description": "繰り上がりを扱える" }
            ]),
            curricula: json!([
                { "id": "intro", "title": "入門", "objectives": ["addition.basic", "carry.basic"] }
            ]),
            resources: json!([
                {
                    "id": "addition.lesson",
                    "type": "markdown",
                    "title": "1と1を合わせる",
                    "path": "content/introduction.md",
                    "teaches": ["addition.basic"]
                }
            ]),
            assessments: json!([
                {
                    "id": "addition.01",
                    "revision": "1",
                    "measures": ["addition.basic"],
                    "stimulus": { "markdown": "1 + 1 はいくつ？" },
                    "response": {
                        "type": "single_select",
                        "options": [
                            { "id": "a", "text": "1" },
                            { "id": "b", "text": "2" }
                        ]
                    },
                    "evaluation": { "type": "exact", "answer": "b" },
                    "feedback": { "markdown": "1に1を足すと2です。" }
                }
            ]),
        })
        .unwrap()
    }

    #[test]
    fn inspect_counts_entities_and_bounds_prerequisite_order() {
        let view = inspect(&model(), 1);
        assert_eq!(view.total_entities, 7);
        assert_eq!(view.manifest.entity_counts["concepts"], 2);
        assert_eq!(view.manifest.entity_counts["assessments"], 1);
        assert_eq!(view.manifest.prerequisite_order, ["addition"]);
        assert!(view.manifest.prerequisite_order_truncated);
        assert_eq!(view.manifest.capabilities.optional, ["org.osmium.media.v1"]);
    }

    #[test]
    fn query_pages_in_document_order_and_rejects_unbounded_limits() {
        // The second of two Concepts: the page reaches the end of the kind.
        let view = query(&model(), EntityKind::Concept, 1, 1).unwrap();
        assert_eq!(view.total, 2);
        assert_eq!(view.returned, 1);
        assert_eq!(view.entities[0].id, "carry");
        assert!(!view.truncated);
        // The first page of a two-item kind leaves more to read.
        assert!(
            query(&model(), EntityKind::Concept, 0, 1)
                .unwrap()
                .truncated
        );
        assert!(query(&model(), EntityKind::Concept, 0, MAX_QUERY_LIMIT + 1).is_err());
        assert!(query(&model(), EntityKind::Concept, 0, 0).is_err());
    }

    #[test]
    fn concept_search_is_case_insensitive_package_local_and_bounded() {
        let mut documents = model().documents().clone();
        documents.concepts.as_array_mut().unwrap().push(json!({
            "id": "addition.extra",
            "title": "足し算の応用",
            "requires": ["addition"]
        }));
        let model = validate_package(documents).unwrap();
        let view = search_concepts(&model, "  足し  ", 1).unwrap();
        assert_eq!(view.query, "足し");
        assert_eq!(view.total, 2);
        assert_eq!(view.results.len(), 1);
        assert_eq!(view.results[0].id, "addition");
        assert!(view.truncated);
        assert!(search_concepts(&model, "  ", 1).unwrap().results.is_empty());
        assert!(search_concepts(&model, "足し", 0).is_err());
        assert!(search_concepts(&model, "足し", MAX_CONCEPT_SEARCH_RESULTS + 1).is_err());
        assert!(search_concepts(&model, &"x".repeat(129), 1).is_err());
    }

    #[test]
    fn query_accepts_plural_kind_spelling() {
        assert_eq!(EntityKind::parse("concepts"), Some(EntityKind::Concept));
        assert_eq!(
            EntityKind::parse("assessment"),
            Some(EntityKind::Assessment)
        );
        assert_eq!(EntityKind::parse("nope"), None);
    }

    #[test]
    fn every_entity_kind_has_a_summary_and_a_label() {
        // Assessments carry no title field, so a label must fall back to the ID
        // instead of assuming a field that the schema does not require.
        for kind in EntityKind::ALL {
            let view = query(&model(), kind, 0, 8).unwrap();
            assert_eq!(view.returned, view.total, "{}", kind.name());
            assert!(!view.entities.is_empty(), "{}", kind.name());
            assert!(
                view.entities.iter().all(|entity| !entity.title.is_empty()),
                "{}",
                kind.name()
            );
        }
        let assessment = query(&model(), EntityKind::Assessment, 0, 8).unwrap();
        assert_eq!(assessment.entities[0].title, "addition.01");
        assert_eq!(
            assessment.entities[0].detail["response_type"],
            "single_select"
        );
    }

    #[test]
    fn context_links_objectives_resources_and_assessments() {
        // Two hops from an Objective reach its Concept, its Resources, its
        // Assessments, the Curriculum that orders it, and the sibling
        // Objective that shares its Concept.
        let view = context(&model(), EntityKind::Objective, "addition.basic", 2, 64).unwrap();
        let ids: BTreeSet<&str> = view.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(
            ids,
            BTreeSet::from([
                "addition.basic",
                "addition",
                "addition.lesson",
                "addition.01",
                "intro",
                "carry",
                "carry.basic"
            ])
        );
        assert!(view.content_is_untrusted);
        assert!(!view.truncated);
    }

    #[test]
    fn concept_context_is_atlas_ready_and_keeps_curriculum_order_separate() {
        let view = context(&model(), EntityKind::Concept, "addition", 2, 64)
            .expect("bounded Concept neighborhood");
        let ids: BTreeSet<&str> = view.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(
            ids,
            BTreeSet::from([
                "addition",
                "carry",
                "addition.basic",
                "carry.basic",
                "addition.lesson",
                "addition.01",
                "intro",
            ])
        );
        assert!(!view.truncated);

        let relation_kinds: BTreeSet<&str> = view
            .relations
            .iter()
            .map(|relation| relation.relation.as_str())
            .collect();
        assert_eq!(
            relation_kinds,
            BTreeSet::from(["concept", "measures", "orders", "requires", "teaches"])
        );
        assert!(view.relations.iter().any(|relation| {
            relation.relation == "requires"
                && relation.from_id == "addition"
                && relation.to_id == "carry"
                && relation.incoming
        }));
        assert!(view.relations.iter().any(|relation| {
            relation.relation == "orders"
                && relation.from_id == "addition.basic"
                && relation.to_id == "intro"
                && relation.incoming
        }));

        let curriculum = view
            .nodes
            .iter()
            .find(|node| node.kind == EntityKind::Curriculum && node.id == "intro")
            .expect("the related Curriculum is available as a detail node");
        assert_eq!(
            curriculum.entity["objectives"],
            json!(["addition.basic", "carry.basic"]),
            "Curriculum order is the author's ordered objective list, not a requires edge"
        );
    }

    #[test]
    fn prerequisite_depth_counts_the_chain_below_the_target() {
        // `addition` has no prerequisites; `carry` needs `addition`.
        assert_eq!(
            context(&model(), EntityKind::Concept, "addition", 1, 64)
                .unwrap()
                .prerequisite_depth,
            Some(0)
        );
        assert_eq!(
            context(&model(), EntityKind::Concept, "carry", 1, 64)
                .unwrap()
                .prerequisite_depth,
            Some(1)
        );
    }

    #[test]
    fn context_follows_prerequisites_in_the_declared_direction() {
        let view = context(&model(), EntityKind::Concept, "carry", 1, 64).unwrap();
        let relation = view
            .relations
            .iter()
            .find(|relation| relation.relation == "requires")
            .unwrap();
        // `carry` declares the requirement, so the relation points from `carry`
        // to `addition` and is outgoing for the target.
        assert_eq!(relation.from_id, "carry");
        assert_eq!(relation.to_id, "addition");
        assert!(!relation.incoming);
    }

    #[test]
    fn context_stops_at_the_node_limit() {
        let view = context(&model(), EntityKind::Concept, "addition", 8, 2).unwrap();
        assert_eq!(view.nodes.len(), 2);
        assert!(view.truncated);
    }

    #[test]
    fn context_rejects_zero_depth_and_unknown_ids() {
        assert!(context(&model(), EntityKind::Concept, "addition", 0, 8).is_err());
        assert!(
            context(
                &model(),
                EntityKind::Concept,
                "addition",
                MAX_CONTEXT_DEPTH + 1,
                8
            )
            .is_err()
        );
        assert!(
            context(
                &model(),
                EntityKind::Concept,
                "addition",
                1,
                MAX_CONTEXT_NODES + 1
            )
            .is_err()
        );
        assert!(context(&model(), EntityKind::Concept, "missing", 1, 8).is_err());
    }

    #[test]
    fn resolve_target_reports_ambiguity_instead_of_guessing() {
        let model = validate_package(PackageDocuments {
            manifest: model().documents().manifest.clone(),
            concepts: json!([{ "id": "shared", "title": "概念", "requires": [] }]),
            objectives: json!([
                { "id": "shared", "concept": "shared", "description": "目標" }
            ]),
            curricula: json!([]),
            resources: json!([]),
            assessments: json!([]),
        })
        .unwrap();
        let errors = resolve_target(&model, "shared").unwrap_err();
        assert_eq!(errors[0].code, "OSM_AMBIGUOUS_ENTITY");
        assert_eq!(
            resolve_target(&model, "present").unwrap_err()[0].code,
            "OSM_UNKNOWN_ENTITY"
        );
    }
}
