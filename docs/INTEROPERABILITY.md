# Interoperability: QTI and SCORM boundaries

Status: documentation only. Osmium implements no QTI import or export, no QTI
XML reader or writer, no IMS content package handling, and no SCORM import,
export, launch or run-time support. This document exists so that a future
adapter has a stated target, and so that nobody mistakes the similarities for
compatibility.

## Position

Osmium's native model is not a QTI derivative.

```text
QTI (XML)  ──[future adapter]──▶  Osmium Assessment IR  ◀── native Osmium Package
```

The adapter is a *conversion* step, not the format. Osmium keeps its own model
because it optimizes for different things:

| Osmium prefers | QTI optimizes for |
| --- | --- |
| human-readable and agent-editable JSON | exhaustive interchange between delivery systems |
| a minimal field set that validates today | coverage of every assessment behaviour a platform may need |
| one meaning per field | a large, layered, backwards-compatible vocabulary |
| explicit absence (`license: null`, optional dates) | defaulting and inheritance rules |

Copying QTI's vocabulary into the native schema would make an Osmium package
unreadable to the authors it is for, and would import a compatibility burden
Osmium has not earned. So QTI stays at the edge.

## Mapping

Assessment-side:

| Osmium | QTI | Fidelity |
| --- | --- | --- |
| `assessments[].id` | `assessmentItem@identifier` | direct |
| `assessments[].revision` | `assessmentItem@title` / item metadata | **lossy** — QTI item versioning is weak and usually metadata, not identity |
| `assessments[].stimulus.markdown` | `itemBody` content | **lossy** — Osmium stimulus is Markdown reaching a typed Content IR; QTI item body is XML with its own markup vocabulary |
| `assessments[].response` (`single_select`) | `responseDeclaration` + `interaction` (`choiceInteraction`, `maxChoices=1`) | mostly direct |
| `assessments[].response` (`boolean`) | `choiceInteraction` with two options, or `trueFalseInteraction` depending on profile | **lossy** — a JSON boolean has no options, so a round trip invents option IDs |
| `response.options[].id` | `simpleChoice@identifier` | direct |
| `response.options[].text` | `simpleChoice` content | **lossy** — plain text out, XML content in |
| `assessments[].evaluation.answer` | `responseDeclaration/correctResponse` | direct for the shipped evaluator |
| `assessments[].evaluation` (`exact`, `org.osmium.exact.v1`) | `responseProcessing` | **lossy both ways** — QTI allows arbitrary processing logic; Osmium permits only a named deterministic evaluator, and QTI has no slot for an evaluator ID and version |
| `assessments[].feedback.markdown` | `modalFeedback` | **lossy** — QTI feedback is conditional on the outcome; Osmium feedback is static, so export loses the condition and import cannot express one |
| `assessments[].measures` (Objective IDs) | *no equivalent* | **lossy on export** — QTI links items to outcomes through a test/outcome model, not an item-level objective list |
| `assessments[].cognitive_level` | *no equivalent* | **lossy on export** — non-normative by design; QTI instruction metadata is a different mechanism |
| `assessments[].evidence_reference_ids` | *no equivalent* | **lossy on export** |
| Assessment order within a Curriculum | `assessmentSection` / `assessmentTest` | **lossy** — Osmium models sequencing as a Curriculum over Objectives, not as an item section |

Document-side:

| Osmium | QTI / IMS | Fidelity |
| --- | --- | --- |
| `osmium.json` manifest | `imsmanifest.xml` | **lossy** — different identity, capability and integrity models |
| `manifest.references` | *no equivalent* | **lossy on export** — QTI has no bibliographic evidence registry |
| `manifest.language` / Resource `language` | `xml:lang` | direct in intent |
| `concepts`, `objectives`, `curricula` | *no equivalent* | **lossy on export** — no concept graph, no prerequisite relation, no objective-level curriculum |
| Resource Markdown | `imscc` / content package resources | **lossy** — Osmium compiles Markdown to a typed IR and forbids raw markup; IMS packages carry arbitrary files |
| Learner `learning_event` | QTI results reporting (`assessmentResult`) | **lossy both ways** — see below |
| `learning_event.assessment_snapshot` | *no equivalent* | **lossy on export** — the snapshot is Osmium's answer to item-drift; QTI results do not embed the item as it was |
| `learning_event.concept_ids` | *no equivalent* | **lossy on export** |
| `learning_event.evaluator` (`id` + `version`) | *no equivalent* | **lossy on export** |
| `score` 0 or 1 | `outcomeVariable` (typically a real number) | **lossy both ways** — widening to a real loses the exactness, narrowing to 0/1 discards partial credit |
| `duration_ms`, `hints_used` | `assessmentResult` metadata | mostly direct |

## What a lossy mapping actually means

Read the table as: *converting an Osmium package to QTI would drop information,
and converting back would not restore it.* The dropped information is not
incidental — `evidence_reference_ids`, the concept graph, the Curriculum and the
result snapshot are the parts of Osmium that make a package auditable. That is
the honest framing, and it is why no round-trip fidelity is claimed anywhere.

The reverse direction is not free either. Importing a QTI item would have to
invent an Objective for `measures`, because QTI has nothing to map it from. An
invented Objective is not traceability, so an importer must either require the
author to name an Objective or leave `measures` empty — and `measures` is
required and non-empty. Any importer therefore needs an author decision, which is
exactly the kind of thing a thin adapter should surface rather than paper over.

## SCORM packaging and runtime boundary

SCORM solves a different interoperability problem from QTI. Its Content
Aggregation Model describes how learning resources are organized and packaged;
its Run-Time Environment defines how a launched Sharable Content Object (SCO)
communicates with a Learning Management System; and its Sequencing and
Navigation model governs delivery through the organization. The ADL programmer
guide describes the package manifest as the structure that organizes content
and tells an LMS what to deliver. The SCORM 2004 conformance requirements treat
package import, SCO launch and API exposure, runtime data, and sequencing as
separate LMS responsibilities.

Osmium's `.osmium` archive is **not** a SCORM content package. It has no
`imsmanifest.xml`, SCO launch contract, SCORM JavaScript API, `cmi` data model,
or SCORM sequencing engine. The runtime records Osmium learning events in its
own store; it does not emulate SCORM's runtime communication model.

| Osmium | SCORM concept | Boundary |
| --- | --- | --- |
| Package manifest and declared payload files | Content package manifest and resources | A future exporter could emit `imsmanifest.xml` and map package identity, metadata, and portable files. Osmium's archive is not directly importable by a SCORM LMS. |
| Curriculum ordering over Objectives | Organization, items, and sequencing rules | A Curriculum is not a SCORM activity tree. A future adapter must ask an author how to map objective coverage and prerequisites to launch order and sequencing; it must not infer SCORM behavior from the Concept graph. |
| Markdown Resource | Asset or SCO resource | An asset can be exported as a static resource. A SCO requires a launchable web application that implements the SCORM API and runtime contract; an Osmium Markdown file alone is not one. |
| Osmium learning event and mastery projection | SCORM runtime data model and completion/success status | Both record learner activity, but the fields, lifecycle, and semantics differ. Mapping would be lossy and would require an explicit policy for attempts, score, completion, and mastery. |
| Assessment definition and Evidence links | Assessment content and package metadata | SCORM packaging does not supply Osmium's Objective, Reference, Evidence, or assessment-definition model. Those relations need an extension or a separate mapping record and cannot be claimed as native SCORM semantics. |

SCORM's sequencing ideas are useful when thinking about launch and delivery
boundaries, just as its manifest and resource organization are useful when
thinking about packaging. They do not justify importing SCORM's runtime or
sequencing model into Osmium's native package schema. A future adapter would
translate at the edge, report lossy mappings, and leave the Osmium Runtime and
learning-event store in control of Osmium behavior.

## Adapter boundary

If a QTI adapter is ever built, it belongs beside `osmium-core` as a separate
crate that produces and consumes the native model:

```text
qti  ──▶ Osmium Assessment IR ──▶ native Package schema
                    ▲
                    └── what a renderer and an adapter both read
```

The boundary rules, so the adapter cannot become a second source of truth:

- The adapter never writes a schema. It emits documents that the existing
  `validate_package` pass must accept, or it fails loudly.
- The adapter never re-implements evaluation. It maps onto the named evaluator
  or refuses the item.
- The adapter never invents an Objective, a Reference or a cognitive level.
  Anything QTI cannot supply is an author decision and is reported as one.
- The native model remains the source of truth. A QTI file is an import input,
  exactly like a Markdown file.

No part of this is implemented.

## Related

- [`ASSESSMENT_MODEL.md`](ASSESSMENT_MODEL.md) — the responsibilities being
  mapped.
- [`SOURCES_AND_PROVENANCE.md`](SOURCES_AND_PROVENANCE.md) — the Reference model
  with no QTI equivalent.

## Primary references

- [1EdTech QTI 3.0 overview](https://developers.imsglobal.org/spec/qti/v3p0/oview) and [QTI 3.0 implementation guide](https://developers.imsglobal.org/spec/qti/v3p0/impl).
- [ADL SCORM Best Practices Guide for Programmers](https://adlnet.gov/assets/uploads/SCORM_Users_Guide_for_Programmers.pdf) and [SCORM 2004 4th Edition testing requirements](https://adlnet.gov/assets/uploads/SCORM_2004_4ED_v1_1_TR_20090814.pdf).
