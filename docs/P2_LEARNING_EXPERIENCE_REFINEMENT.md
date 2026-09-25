# P2 Learning Experience Refinement

Status: implemented as a runtime/UI prototype against the existing v0.1 package model. No stable package schema, Core primitive, or package format was added.

## Executive summary

P2 addressed one concrete Library failure and pressure-tested a more navigable Lesson against the 44-Concept cardiovascular package. Curriculum sections now mount their contents only while expanded, Resources are linked once per section, and the Lesson presents a compact author-ordered before/current/next context. Learners can jump from an explicit prerequisite to its Concept in the Package-local Atlas. Route remains available as an optional full-curriculum view.

Two learning loops were prototyped without changing package data: a single incorrect Assessment can route to a Resource that teaches its measured Objective and return to a clean retry; a fixture-allowlisted, runtime-owned pressure-volume interaction can be explored and retried. A separate cardiovascular fixture note demonstrates that a “related topic” such as heart failure and RAAS is different from a prerequisite. These are prototypes, not evidence for adding universal Activity, Relation, or Remediation schema yet.

The real Library uninstall issue was caused by strict validation of every remaining package after the selected payload had already been removed. A damaged unrelated package could make uninstall report failure after the target was gone. The Store now syncs only verified packages following uninstall while damaged entries remain in the management inventory. The UI clears selected learning/navigation state only after a confirmed successful removal and explains that append-only History remains.

## 1. Library uninstall: reproduced behavior and correction

`Runtime::uninstall` delegates payload removal to the Package Library and then refreshes Runtime package availability. Previously that refresh called `Library::packages()`, which strictly validates all installed distributions. If an unrelated version was damaged, the requested payload had already been removed but this later strict listing returned an error. The desktop surfaced an error and kept stale UI state, even though the payload was absent.

After removal, Runtime now syncs from `Library::valid_packages()`. This leaves damaged payloads in the bounded management inventory so they can still be removed, while they do not prevent valid packages from remaining usable. A Store regression test installs two versions, damages one, uninstalls the other, and checks that the damaged entry remains inventoried and History remains intact.

The desktop deletion path now:

- confirms the selected Package title and exact version, and says that History is retained;
- verifies the returned package ID/version against the selected version before treating the operation as success;
- clears Lesson, Resource, Assessment, progress, Route, and Atlas selection state for that package after success;
- refreshes the Library only on success, shows retained-history success feedback, and preserves state while reporting a failure;
- leaves append-only learning events and their readable snapshots available after the package payload is gone.

The end-to-end lifecycle test uses a fresh isolated data home and performs Library UI deletion, verifies the exact digest payload directory is gone, verifies Runtime and Library no longer list the package, restarts Desktop and checks absence again, confirms two History events still render without the payload, reinstalls the same `.osmium` distribution, and confirms the same digest restores the observed progress.

## 2. Lesson refinement

### Curriculum grouping and mounting

The Lesson derives its sections from the Package's authored Curriculum structure; this does not introduce a new Module model. The first Curriculum is initially open and other sections are collapsed. Closed sections do not mount their Concept/Objective contents. In the P1.5 cardiovascular fixture (44 Concepts, 44 Objectives, four Curricula), opening the Lesson mounts 10 Concepts/Objectives from the initial section instead of all 44. Expanding a section mounts its content then.

Resources are deduplicated at the section level. A shared Resource appears once with a concise list of supported Objectives behind a disclosure, rather than being repeated under each Objective. Assessments remain attached to their measured Objectives. Counts distinguish observed answers from unattempted assessed Objectives; non-assessed Objectives are not presented as unanswered questions. Resource de-duplication remains within the current Curriculum section: a Resource referenced in two distinct sections may still appear in both contexts so that each section remains understandable on its own.

### Author order and orientation

When an authored Curriculum sequence exists, the current focused Objective is shown with its immediate before/current/next neighbors in that author order. This is explicitly a curriculum position, not a claim about inferred mastery or a computed optimal path. If the Package has no Curriculum, the Lesson does not invent this navigation. The full Route stays optional and answers a larger question than the compact Lesson context: it exposes the ordered Objective route and why a learner is at a particular point.

### Prerequisites and Atlas

Explicit prerequisites are visually and behaviorally distinct from other topic relations. Each prerequisite title links directly to that Concept in the Package-local Atlas, providing a fast “why/what comes before this?” investigation without making the learner edit a graph. Atlas remains the explicit Package graph; no cross-Package mapping is introduced.

The cardiovascular fixture also has a temporary, hardcoded prototype callout on the heart-failure Concept saying it is related to RAAS but is not a prerequisite. Its button navigates to RAAS in the Atlas. This is test fixture UI, not package-authored relation support and not a Core semantic contract.

## 3. Route decision

Keep Route as an optional view. A compact before/current/next strip is faster for a learner who only needs nearby context. Route continues to make a larger authored sequence inspectable and supports orientation to the current goal. Neither view replaces the other, and Route is not required to answer a question or open the next Assessment. An E2E pressure test still opens Route and validates its four authored Objectives in the exam-oriented mathematics Package.

Do not turn Route into a recommendation algorithm in this phase. Current navigation is based on explicit authored sequence and references only.

## 4. Remediation prototype

On an incorrect answer, Assessment can show a retry route to a Resource whose `teaches` references overlap the Assessment's `measures` Objective. Returning from that Resource restores the same Assessment but clears the previous attempt selection, allowing a fresh answer. The E2E path records an incorrect preload response, opens the linked Resource, returns, submits the correction, and finds two separate answer events in History.

This demonstrates that the existing `Assessment.measures` + `Resource.teaches` composition is sufficient for one useful remediation loop. It does not yet support authored branching, item banks, retry policy, remediation sequencing, mastery criteria, or evidence beyond the existing answer event. A generic remediation graph is not justified by one case.

## 5. Interactive learning prototype

`PressureVolumePrototype.tsx` is a small Runtime-owned React interaction embedded beside the pressure-volume Resource. It uses built-in buttons/radios, displays a schematic loop and observation, checks one answer, returns feedback, and permits retry. The renderer is enabled only for the exact cardiovascular validation Package ID and Resource ID. No Package-provided JavaScript is loaded or evaluated, and the prototype does not record learning events or alter mastery/progress. The Resource text remains available as the fallback.

This shows that a read-only Resource plus a simple quiz may not be enough for every instructional experience. It does **not** establish a stable Activity schema or a universal interaction runtime. One cardiovascular interaction is not evidence that all learning domains can be served by one hardcoded Core primitive.

If similar needs recur in independent Packages, prefer a small declarative request describing a known interaction capability and typed inputs/data, with Runtime-owned, allowlisted renderers. Unknown capabilities should degrade to the Resource fallback. Arbitrary package scripts, eval, or a general plugin loader are explicitly out of scope.

## 6. No new stable Relation, Activity, or Remediation schema

The existing schema and typed authoring model remain unchanged in this work. In particular:

- prerequisite relations continue to use current explicit Concept relation fields and current Core validation;
- curriculum order continues to come from Curriculum data;
- instructional attachment continues through Objective references to Resource and Assessment;
- the related-topic callout is fixture-only;
- the remediation route is a Runtime inference over existing Objective links, not persisted policy;
- the PV interaction is an exact fixture allowlist, not a Package extension or Core capability.

A distinct typed non-prerequisite relation may be warranted once a second Package has a concrete authoring need and provenance/validation/UI semantics can be agreed. Any such relation must not be interpreted as pedagogical ordering. Similarly, a future Activity proposal should start with repeated cross-domain needs and a safe declarative capability boundary, not a catalog of hardcoded activity kinds.

## 7. Authoring implications

For Level 1 authors, the immediate model remains title, Resource, Assessment, and authored Curriculum order; a Resource-to-Objective relation is reused for the review loop. Lesson grouping is generated from existing Curricula. The current interaction prototype is not authorable by ordinary teachers.

For Level 2 learning designers, current explicit prerequisites and Atlas references are usable, but there is no general GUI for non-prerequisite relations, remediation policy, or interactive capability configuration. The prototype surfaces should not be mistaken for authoring support.

For Level 3 developers/agents, current structured Package files and Core validation remain the source of truth. No new JSON contract was added. This preserves the existing distinction: internal representation may be explicit, while learner-facing screens remain simple.

Before promoting related relations, Activity, or remediation into package data, validate author workflow on at least two unrelated domains and establish lint/validation, localization, accessibility, fallback, offline behavior, and provenance semantics.

## 8. A–K review answers

### A. Is the uninstall fixed?

The reproduced failure is fixed and the actual desktop flow is covered. The test validates deletion of the selected on-disk payload, restart behavior, retained readable History, and same-digest reinstall/progress restoration. The independent damaged-version regression protects the original false-error path.

### B. Can Lesson handle the real 44-Concept package?

The first screen now mounts only 10 Concepts/Objectives, one section's unique Resource set, and its Assessments. Remaining sections mount on expansion. This addresses the tested package shape; it does not prove suitability for arbitrarily large curriculum counts or deeply nested routes.

### C. Should Route be removed?

No. Keep it optional: Lesson gives immediate local context; Route exposes the full authored learning sequence and goal.

### D. Is prerequisite-to-Atlas navigation useful?

Yes for direct inspection of an explicit prerequisite. The E2E flow verifies it opens the expected Concept center. It does not turn prerequisites into a recommendation or mastery model.

### E. Is `related` a new stable relation now?

No. The heart-failure/RAAS fixture callout tests the learner-facing distinction, but one domain example is too little to define a stable cross-domain relation vocabulary and authoring semantics.

### F. Is generic remediation needed?

Not yet. One retry loop can be composed from current Assessment/Objectives/Resource references. Branching policy and repeated author needs are not yet demonstrated.

### G. Is a stable Activity primitive needed?

Not proven. The PV interaction indicates a real gap in the text-plus-quiz ceiling, but the current exact fixture allowlist is only a behavior prototype. Seek a second independent use case before standardizing an Activity contract.

### H. How should package-provided behavior work?

No arbitrary code execution. A future declarative capability/data request may be rendered by known Runtime components. Unknown capability must safely fall back to Resource content. That contract is a future investigation, not implemented here.

### I. What is missing for authoring Levels 1/2/3?

Level 1 can use existing authoring for basic Resource/Assessment/Curriculum composition but cannot author the PV interaction. Level 2 lacks GUI for general non-prerequisite relation, remediation, or interaction policy. Level 3 has structured source and validation but no new contract from P2. Verify the user cost before expanding the authoring model.

### J. What should not be built now?

Do not add stable `related` edges, a universal Activity catalog, generic remediation graphs, arbitrary plugin/script execution, mastery-transfer rules, or a new schema version based on these prototypes.

### K. What is the next valuable validation?

Observe novice learners on Lesson → Route → Atlas and a remediation retry, and observe an ordinary teacher author the same Resource/Assessment sequence. Seek a second interaction/remediation need in an unrelated Package. Only then decide whether to introduce declarative Activity or Relation authoring.

## 9. Performance, bundle, and limitations

### Lesson mounting

The P1.5 validation fixture has 44 Concepts/44 Objectives. The expanded initial section mounts 10; collapsed Curriculum sections mount no Concept/Objective cards until opened. The E2E suite measures Lesson opening, section expansion, direct Atlas opening, Route/Atlas navigation, Atlas search/recenter, and the remediation/interaction feedback steps. These are UI timing samples from the local E2E environment, not a cross-device benchmark.

### Frontend production bundle

Normal `npm run build` after P2 reports:

| Artifact | P1.5 reported build | P2 build | Difference |
|---|---:|---:|---:|
| JavaScript, minified | 778.90 kB | 788.61 kB | +9.71 kB |
| JavaScript, gzip | 235.55 kB | 238.46 kB | +2.91 kB |
| CSS, minified | 51.01 kB | 53.67 kB | +2.66 kB |
| CSS, gzip | 13.67 kB | 14.22 kB | +0.55 kB |

No dependency was added. Vite still warns that the main JavaScript chunk exceeds 500 kB; this P2 work did not split the existing application bundle. The old and new numbers are build-output comparisons from the reported P1.5 baseline, not a controlled microbenchmark.

### Known limits

- The non-prerequisite relation UI and PV interaction are hardcoded to the validation fixture IDs.
- Interaction and remediation outcomes are not persisted as new event types; only the normal Assessment answer is in History.
- The sample is one large flat-ish four-Curriculum Package, not a 100k Concept workload.
- No new mobile performance benchmark was run. The prototype uses standard DOM/SVG controls and no GPU/model dependency, but this alone does not validate the broader product on Android/iOS.
- Route and Atlas UI timing varies with the local app and data home; report timings only with the E2E run output and environment.

## 10. Validation

Completed checks:

- `npm run lint` — pass.
- `npm run typecheck` — pass.
- `npm test` — pass, 31 tests.
- `cargo test --workspace -j 1` — pass, including the damaged-sibling uninstall regression.
- isolated `cargo build -p osmium-desktop -p osmium-cli -j 1` — pass.
- Desktop E2E — pass, 347 checks. This includes the UI-delete / payload removal / restart / retained History / same-digest reinstall lifecycle.
- normal `npm run build` — pass; 788.61 kB JavaScript and 53.67 kB CSS minified, with the existing >500 kB chunk warning.
- `git diff --check` — pass; Git emitted only existing working-tree LF-to-CRLF conversion notices.

P2 does not alter stable Package schema, Package extension, Runtime identity semantics, persisted event schema, or database migrations.
