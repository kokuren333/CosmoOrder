# Assessment model

Status: development contract for Package format 0.1. The responsibility
boundaries below are the contract. Only part of the intended model is
implemented, and each section says which part.

## Why QTI is an influence and not a dependency

QTI already separates an item's meaning from how it is delivered and from what a
learner answered. That idea is right and Osmium adopts it. QTI itself is not
adopted: its native form is XML written for interchange between learning
platforms, and Osmium's native form is a small, human-readable, agent-editable
JSON document.

Osmium does not become "QTI XML serialized to JSON". See
[`INTEROPERABILITY.md`](INTEROPERABILITY.md) for the mapping and for what is
lost in each direction.

The separation Osmium holds to:

```text
meaning model
  != serialization
  != runtime presentation
  != learner result
```

## Responsibilities

| Responsibility | Where it lives today | Implemented |
| --- | --- | --- |
| **Assessment item** — the authored unit | `assessments[].{id, revision, measures, cognitive_level}` | yes |
| **Interaction** — what the learner does | `response` (`single_select`, `boolean`) | yes, two kinds |
| **Response model** — what counts as a valid answer | implicit in the interaction; validated before grading | yes, not separable |
| **Scoring / evaluation** — how an answer becomes a score | `evaluation`, run by `org.osmium.exact.v1` | yes, exact match only |
| **Feedback** — what the learner reads after answering | `feedback.markdown` | yes, static |
| **Result** — what happened for this learner | `learning_event` in the store | yes, separate document |
| **Presentation** — how any of it looks | Desktop components | yes, separate layer |

The rule that keeps these apart: an Assessment definition is authored, versioned
and distributed. A learner result is *observed*, append-only, and never written
back into a package.

## Interaction

`single_select` and `boolean` are the two shipped interactions. The stimulus is
compiled Markdown, so an item can carry a table, an equation or a code fence the
same way a lesson does.

The interaction declares *what kind of answer exists*, not how it is drawn.
`single_select` means "exactly one option is chosen". It does not mean a radio
button, a card, or a list on a phone. The Desktop currently renders radio cards
because that is a reasonable desktop presentation; a mobile renderer may render
the same item as a list without touching the package. The Desktop test suite
asserts `type="radio"` — that assertion belongs to the Desktop, not to the
package contract, and the package schema contains no markup hint of any kind.

Adding an interaction means adding a response shape, a validation rule and an
evaluator. It does not mean adding a presentation.

## Response model

Each interaction defines what a syntactically valid response is. `evaluate`
rejects a response that is not an available option ID or not a JSON boolean
*before* grading, with `OSM_RESPONSE`. A malformed answer is an error, never
silently recorded as incorrect — recording a client bug as a wrong answer would
corrupt every statistic derived from the event log.

There is currently no separate response-model document: validity is implied by
the interaction plus the validation pass. A future revision could split it out
so that partial credit or a response-specific constraint can be expressed
without changing the interaction.

## Scoring and evaluation

`evaluation` names the evaluator and its answer. The shipped evaluator
`org.osmium.exact.v1` version `1` is deterministic, side-effect-free, and returns
`score` 0 or 1. Its identity is recorded on every result, so a future evaluator
(partial credit, manual grading, rubrics) can coexist with it in the same event
log without reinterpreting old events.

Evaluation is separated from the item so the *rule* can change without the item
changing. It is intentionally not a general expression language: a scoring
engine that can run arbitrary logic is arbitrary code in a package, which
Osmium forbids.

## Feedback

Feedback is Markdown compiled to the same Content IR as lesson bodies. It is
static today: the same feedback is shown whatever the learner answered. Adaptive
feedback would need a condition on the response, which does not exist yet.

## Assessment and Objective

`measures` is required and non-empty. It is the Assessment → Objective link and
is validated: every ID must resolve to an Objective in the package
(`OSM_REFERENCE`), and lint reports an Objective that no Assessment measures
(`OSM_LINT_OBJECTIVE_NO_ASSESSMENT`). The transactional direction — Resource
`teaches`, Curriculum `objectives`, Assessment `measures` — is the whole
traceability story.

`cognitive_level` is an optional, free-form, non-normative label for what the
item asks of a learner:

```json
{ "cognitive_level": "integration" }
```

Osmium does not own a taxonomy and does not validate the value. The documented
default vocabulary is `recall`, `discrimination`, `application` and
`integration`, borrowed from the four kinds of understanding that a medical
lesson is expected to build, but an author may use any short label from any
framework they already follow. The schema enforces only length and control
characters; lint warns when the field is absent
(`OSM_LINT_ASSESSMENT_COGNITIVE_LEVEL`). Hard-coding Bloom levels or any other
numbered taxonomy into the schema would force every author into one framework for
no validation benefit, so it is deliberately not done.

## Result and definition

The definition and the result are different documents with different lifetimes.

```text
Assessment definition          Learner result (learning_event)
  id, revision                   event_id, device_id, request_id
  measures                       package_id, package_version, package_digest
  cognitive_level                assessment_id, assessment_revision, assessment_hash
  stimulus                       assessment_snapshot
  response                       response
  evaluation                     score, correct, evaluator
  feedback                       timestamp, duration_ms, hints_used
  evidence_reference_ids         objective_ids, concept_ids
```

The event embeds `assessment_snapshot`: a copy of the definition *as it was
when the learner answered*. That is deliberate and it is the only direction in
which a definition appears inside a result. A package may be updated; an old
attempt must still be explainable against the item that produced it, which is
also why `assessment_revision` and `assessment_hash` are recorded.

Nothing flows the other way. No attempt, response, score, correctness,
timestamp, duration, hint count or objective progress may be written into an
Assessment definition, and the schema enforces that structurally:
`additionalProperties: false` on the assessment definition means `score`,
`response`, `correct`, `attempt`, `timestamp`, `duration_ms` or `hints_used` on a
definition is a schema error.
`crates/osmium-core/tests/validation.rs` asserts exactly that.

Future additions and where they would land:

| Addition | Where it goes | Why |
| --- | --- | --- |
| multiple attempts | already representable — one event per attempt | the log is append-only |
| partial credit | a new evaluator ID + version, and a wider score range | score is already evaluator-attributed |
| manual grading | a later event that scores an earlier one | results stay append-only |
| confidence | a field on the result | it is an observation, not a definition |
| latency | `duration_ms` already exists | same |

## Presentation boundary

Desktop receives the typed definition and a separate attempt DTO. It never
receives a package path to read, never re-parses Markdown, and never derives
grading: `evaluate` has already run in Core. The components render
`AssessmentResponse` as a discriminated union, so a new interaction kind is a
TypeScript exhaustiveness error rather than a silent fallback.

## Assessment IR

Content already has `Markdown → Content IR → Runtime`, so no renderer
re-parses Markdown. Assessment does not yet have the equivalent
`Package schema → Assessment IR → Desktop/Web/Mobile` layer.

Auditing the current coupling: the Desktop types `Assessment` structurally, so
it depends on package JSON shape directly, and `AssessmentView` switches on
`response.type`. That is one renderer and two interactions today, which is
survivable; it will not be at four interactions and three renderers. The
intended direction is an Assessment IR that exposes resolved meaning —
interaction kind, options with stable IDs, whether an attempt is expected —
with the package JSON as one of its inputs. It is not implemented, and the
Desktop type is the thing that would change first.

## Accessibility

No accessible-content work is implemented. The structure does not obstruct it:

- the stimulus and feedback are Content IR, so alt text, semantic role and
  language travel with the content rather than with a markup string;
- the item's language comes from the package or the Resource, and a BCP 47 tag
  is already validated;
- nothing in the schema names a visual presentation, so an accessible
  alternative is not fighting a fixed layout;
- per-learner accommodation metadata is not modelled at all. When it is added it
  belongs with the result or the session, never inside the distributed
  definition, because accommodations are properties of a learner and a
  definition is shared by everyone.

The Desktop renders a native `details` disclosure, labelled radio inputs and
`meter` elements, and marks one current navigation item; accessibility extension
is a presentation concern with room to grow.
