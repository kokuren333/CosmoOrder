# AI-assisted package authoring

Osmium treats people and agents as authors of the same portable package format. A useful reviewable workflow is:

```text
open, licensed, or otherwise permitted sources
  → read and analyze the material
  → extract concepts and decompose objectives
  → verify claims across independent sources
  → draft original resources and assessments
  → record consulted sources, creator, license, attribution, and provenance
  → run osmium validate --json and osmium lint --json
  → inspect diagnostics and the built package
  → human review, revise, then build or publish
```

The CLI is the machine interface: package edits remain ordinary files, diagnostics have stable codes and deterministic ordering, and GUI access is not required. Extensions can carry namespaced provenance or review metadata while the schema is being pressure-tested.

Osmium records evidence and supports audit. It does not decide whether a use is lawful. Shortening or paraphrasing a source does not by itself establish permission. Authors must check source terms and obtain appropriate human review; quotation, adaptation, and attribution should be explicit and traceable.

## Design motivation

Tools such as Obsidian help organize knowledge, and Anki helps with recall practice; neither category alone provides a structured learning runtime for reusable curricula, resources, assessments, and progress. Osmium explores an open infrastructure where individuals, educators, institutions, and specialist communities can build, inspect, share, or privately manage learning packages. Open packages can reduce access barriers for learners who cannot readily access tutoring or costly study materials, while organizations can use the same format for their own materials. The project does not require commercial education to be excluded.

AI may reduce the cost of organizing permitted knowledge sources into reusable learning material. Generated content still needs source tracking, cross-checking, and human review. Osmium does not include an AI generation service or automatic rights assessment.

## Schema pressure-test notes

The current Objective has one `concept` field, while an integrated clinical objective may span several concepts. Keep the current relation for this test and record the modeling pressure rather than silently duplicating or broadening it. Audio can currently be discussed in Markdown but there is no typed media asset reference. Tables and equations belong to renderer support rather than new educational entity types. Revisit multi-concept objectives, media references, and assessment response types after inspecting packages from multiple domains.

## Current capability audit (before this authoring increment)

| Capability | Initial status | Evidence / gap |
|---|---|---|
| Package initialization | Supported | `osmium init` creates a minimal valid source. |
| Metadata editing | Partially supported | Plain JSON/YAML edits; no structured edit command. |
| Content language | Partially supported | Required manifest language and BCP 47 validation; Resource overrides absent. |
| Concept creation | Partially supported | Entity files can be edited; no entity-create operation. |
| Curriculum creation | Partially supported | Entity files can be edited; no structured operation. |
| Resource creation | Partially supported | Markdown and entity files; no typed source registry. |
| Assessment creation | Partially supported | JSON authoring supported; single-select/boolean exact scoring only. |
| Prerequisite editing | Partially supported | `Concept.requires` exists; edit is manual. |
| Source registration | Partially supported | Legacy free-text Resource `source`; no typed registry. |
| Provenance | Partially supported | Unspecified Resource object, no semantics. |
| Source visibility | Unsupported | No visibility or distribution filter. |
| Validation | Supported | JSON Schema, semantic refs/graph, filesystem/path checks. |
| Lint | Supported | Coverage/metadata advisories; no language/source/body heuristics. |
| Build | Supported | Reproducible normalized distribution and archive verification. |
| Package inspection | Supported | JSON inspect/query/context. |
| Agent-friendly structured operations | Partially supported | Deterministic JSON CLI exists; editing/source CRUD and shared MCP adapter absent. |

A shell agent can initialize, validate, inspect, lint and build, but must hand-edit files to add Concepts, Resources, Assessments and source metadata. Sources were not typed, linked to Resources, or filtered at distribution time. CLI and MCP were architectural intentions, not an exposed shared operation API.

## Resource quality audit

Before the fixture update, arithmetic and cross-domain fixtures were compact renderer/schema probes rather than standalone lessons. Language and programming samples include useful focused examples but do not consistently teach concepts from first principles. The medical fixture was especially thin: three short English notes, one table, and an assessment case; foundational explanation, distinctions, counterexamples, and citations were mostly missing. The update expands the medical notes in Japanese. It does not assert expert clinical review or replace current local guidance.

## Capability boundaries and authoring quality

- **CLI/MCP = capabilities**: deterministic machine operations over shared Core/Package contracts.
- **Skills = workflows/policies**: ordering, evidence judgement, privacy constraints, quality heuristics and output contract.
- **Model = reasoning**: decomposes concepts, evaluates evidence and drafts content.
- **Agent capability != Package semantics**: search/browser/shell availability affects evidence acquisition, never package meaning or format.

Hard constraints (schema validity, resolvable source IDs, valid language tags, portable locators, no private data in distributions) are separate from quality heuristics (sufficient explanation, useful examples, relevant misconceptions). No fixed resource word count or question count is a correctness rule. Authoring quality evaluation should be fixture-based and generation-model independent, covering concept coverage, completeness, factual grounding, language consistency, assessment quality, redundancy, traceability, privacy leakage, schema validity and lint. Current fixtures are seeds, not a benchmark harness.
