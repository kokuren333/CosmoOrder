# AI-assisted package authoring

Osmium treats people and agents as authors of the same portable package format. A useful reviewable workflow is:

```text
open, licensed, or otherwise permitted sources
  → record what was read, and where it came from, in .osmium/ (authoring input)
  → read and analyze the material
  → extract concepts and decompose objectives
  → verify claims across independent sources
  → decide, per document, whether it is Evidence or only input
  → register References and attach them as Evidence
  → draft original resources and assessments
  → record creator, attribution, reuse policy and language
  → run osmium validate --json and osmium lint --json
  → inspect diagnostics and the built package
  → human review, revise, then build or publish
```

The CLI is the machine interface: package edits remain ordinary files, diagnostics have stable codes and deterministic ordering, and GUI access is not required. Extensions can carry namespaced provenance or review metadata while the schema is being pressure-tested.

Osmium records evidence and supports audit. It does not decide whether a use is lawful. Shortening or paraphrasing a source does not by itself establish permission. Authors must check source terms and obtain appropriate human review; quotation, adaptation, and attribution should be explicit and traceable.

## Authoring input is not Evidence

An agent that reads a PDF, a private institutional page, a search result set or its own earlier draft has produced *authoring input*. That is not medical or scholarly evidence, and it must not become a Reference. The two live in different places:

```text
.osmium/authoring.json …   local paths, private URLs, notes, agent workflow metadata
manifest.references[] …    the knowledge a learner may be shown
```

`.osmium/` is skipped by the package loader and gitignored. Nothing in it can reach a build, a runtime DTO or a Git commit by default. A document that an agent read *and* that the lesson actually rests on becomes a Reference through an explicit author decision; the fact that an agent fetched it is irrelevant to whether it is evidence.

`visibility: private` is not the authoring workspace. It is a legacy compatibility field for an old package that must stay loadable, and it still means a private record was authored inside the package directory — the exact situation the workspace exists to avoid.

Three operations exist because the authoring benchmark showed that a Reference and the Evidence relation pointing at it had to be edited in two files with nothing checking agreement until the whole package was re-read:

```sh
osmium reference add <path> --id nice-cg174 --kind url --type guideline \
  --title "…" --locator "https://…" --citation "…" --publisher NICE --version CG174
osmium reference attach <path> nice-cg174 --resource findings.lesson
osmium reference list <path> --json
```

See [`AUTHORING_WORKSPACE.md`](AUTHORING_WORKSPACE.md) and [`SOURCES_AND_PROVENANCE.md`](SOURCES_AND_PROVENANCE.md).

## Design motivation

Tools such as Obsidian help organize knowledge, and Anki helps with recall practice; neither category alone provides a structured learning runtime for reusable curricula, resources, assessments, and progress. Osmium explores an open infrastructure where individuals, educators, institutions, and specialist communities can build, inspect, share, or privately manage learning packages. Open packages can reduce access barriers for learners who cannot readily access tutoring or costly study materials, while organizations can use the same format for their own materials. The project does not require commercial education to be excluded.

AI may reduce the cost of organizing permitted knowledge sources into reusable learning material. Generated content still needs source tracking, cross-checking, and human review. Osmium does not include an AI generation service or automatic rights assessment.

## Schema pressure-test notes

The current Objective has one `concept` field, while an integrated clinical objective may span several concepts. Keep the current relation for this test and record the modeling pressure rather than silently duplicating or broadening it. Audio can currently be discussed in Markdown but there is no typed media asset reference. Tables and equations belong to renderer support rather than new educational entity types. Revisit multi-concept objectives, media references, and assessment response types after inspecting packages from multiple domains.

Two pressures were resolved in this increment and one was deliberately left open:

- **Reference identity vs material type.** `kind` (how a reader resolves a record) and `type` (what the material is) are now separate fields, because one enum was answering both questions and no value could be right for both.
- **One visibility enum, two questions.** `record_visibility` and `locator_visibility` now exist independently, with the legacy enum kept as a compatibility view.
- **Still open: one locator per Reference.** A `locators:` list would serve a DOI plus a landing page plus an archived copy, but no current fixture needs it. It is documented as a direction, not implemented.

## What "verifiable" means here

Two levels, which must not be conflated:

```text
Resource-level traceability  … implemented
Claim-level verifiability    … not implemented
```

A Resource or Assessment names the References it draws on, and a learner can read them. Nothing ties a specific sentence to a specific Reference, so an uncited claim can sit between two good References. Do not describe generated or reviewed content as "claim-verified" or "sourced sentence by sentence". The intended future mechanism is a typed Citation in the Content IR — see [`SOURCES_AND_PROVENANCE.md`](SOURCES_AND_PROVENANCE.md#resource-level-traceability-and-claim-level-verifiability).

## Current capability audit

| Capability | Status | Evidence / gap |
|---|---|---|
| Package initialization | Supported | `osmium init` creates a minimal valid source. |
| Metadata editing | Partially supported | Plain JSON/YAML edits; no structured edit command. |
| Content language | Supported | Required manifest language, BCP 47 validation, Resource override. |
| Concept / Curriculum creation | Partially supported | Entity files can be edited; no entity-create operation. |
| Resource creation | Partially supported | Markdown and entity files; no resource-create operation. |
| Assessment creation | Partially supported | JSON authoring; `single_select`/`boolean` with exact scoring; `cognitive_level` optional. |
| Prerequisite editing | Partially supported | `Concept.requires` exists; edit is manual. |
| Reference registration / Evidence | Partially supported | Typed `references` registry, `evidence_reference_ids`, and `osmium reference add/attach/list`. |
| Authoring provenance | Supported | `.osmium/` workspace, skipped by the loader and gitignored. |
| Reference visibility | Supported | `record_visibility` / `locator_visibility` enforced at build, distribution-read and runtime-DTO boundaries. |
| Validation | Supported | JSON Schema, semantic refs/graph, filesystem/path and URL-privacy checks. |
| Lint | Supported | Coverage, metadata, language, license, evidence and assessment heuristics. |
| Build | Supported | Reproducible normalized distribution and archive verification. |
| Package inspection | Supported | JSON inspect/query/context. |
| Agent-friendly structured operations | Partially supported | Deterministic JSON CLI plus three Reference verbs; general entity CRUD and a shared MCP adapter are absent. |

A shell agent can initialize, register References, attach Evidence, validate, inspect, lint and build, but must still hand-edit files to add Concepts, Resources and Assessments.

## Resource quality audit

Before the fixture update, arithmetic and cross-domain fixtures were compact renderer/schema probes rather than standalone lessons. Language and programming samples included useful focused examples but did not consistently teach concepts from first principles. The medical fixture was especially thin: three short English notes, one table, and an assessment case; foundational explanation, distinctions, counterexamples, and citations were mostly missing. The later increments expanded the medical notes in Japanese and rebuilt the language, mathematics and programming fixtures around a single teaching arc each. None of this asserts expert clinical review or replaces current local guidance.

## Assessment quality audit

An early assessment pattern was a near-restatement of the lesson sentence with two options, one of which was obviously wrong, and a "review" item that asked whether the learner should use the material. Those items measured recognition of phrasing and nothing else, and the expanded arithmetic fixture still shows the pattern (`OSM_LINT_ASSESSMENT_SHALLOW`, `OSM_LINT_REUSED_DISTRACTOR`).

The medicine fixture now carries four items that move through the levels deliberately: distinguishing extracellular from intracellular compartments, applying the concentration-versus-content distinction to a case, interpreting two findings without over-claiming, and integrating several findings from a described case while stating what remains unknown. Every item names the Objective it measures and its `cognitive_level`; every item carries `evidence_reference_ids`. The point is not difficulty — a harder item can still measure nothing — but whether a wrong answer is informative.

## Capability boundaries and authoring quality

- **CLI/MCP = capabilities**: deterministic machine operations over shared Core/Package contracts.
- **Skills = workflows/policies**: ordering, evidence judgement, privacy constraints, quality heuristics and output contract.
- **Model = reasoning**: decomposes concepts, evaluates evidence and drafts content.
- **Agent capability != Package semantics**: search/browser/shell availability affects evidence acquisition, never package meaning or format.

Hard constraints (schema validity, resolvable Reference IDs, valid language tags, portable locators, no authoring input in distributions) are separate from quality heuristics (sufficient explanation, useful examples, relevant misconceptions, meaningful license metadata, item depth). No fixed resource word count or question count is a correctness rule. Authoring quality evaluation should be fixture-based and generation-model independent, covering concept coverage, completeness, factual grounding, language consistency, assessment quality, redundancy, traceability, privacy leakage, schema validity and lint. Current fixtures are seeds, not a benchmark harness.
