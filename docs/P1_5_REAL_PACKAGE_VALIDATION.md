# P1.5 — Real Learning Package Validation

Date: 2026-09-25  
Scope: learner UX and current v0.1 Package primitives only. No schema, Store, Personal Atlas, embedding, or new learning primitive was added.

## 1. Packages used

| Package | Contents | What it pressures |
|---|---|---|
| `examples/cardiovascular-atlas-validation` (`org.example/cardiovascular-atlas-validation`) | New, deliberately shallow Japanese validation Package: 44 Concepts and 44 Objectives, four 10–12-objective Curricula, four module Resources, eight single-select Assessments, and multi-level `requires` branches. | Atlas fan-out/depth/search, Lesson length, Objective/Resource repetition, uneven assessment coverage, and a concrete non-prerequisite relation case. |
| `examples/mathematics-pressure-test` (`org.osmium.pressure/mathematics`, v0.2.0) | Existing exam-study Package with four Concepts in a linear probability → events → conditional probability → Bayes chain, four ordered Objectives, Resources, and Assessments. | Whether the same learner path works outside medicine; package-local search isolation and a narrow exam-preparation sequence. |

The cardiovascular fixture is a UX/load probe, not a complete course or medical reference. Its `requires` edges are authored pedagogical sequencing assumptions, not a canonical causal network. The resource bodies are short orientation summaries and are not suitable for clinical decisions or treatment. Their reference metadata points to [OpenStax cardiac cycle](https://openstax.org/books/anatomy-and-physiology-2e/pages/19-3-cardiac-cycle), [OpenStax cardiac physiology](https://openstax.org/books/anatomy-and-physiology-2e/pages/19-4-cardiac-physiology), and the [2026 ESC heart-failure guideline](https://www.escardio.org/guidelines/clinical-practice-guidelines/all-esc-practice-guidelines/heart-failure/). This scope intentionally avoids writing a full medical textbook.

CLI validation accepted the fixture as v0.1 and read 10 files / 104 total entities. Lint returned 43 warnings: three brief overview Resources, four fixture Resources with unknown reuse licenses, and 36 Objectives without a linked Assessment. Those are deliberate fixture limits and are useful for testing “not assessed” versus “not attempted”; lint itself succeeded.

## 2. Actual learner scenarios

| Scenario | Finding | Classification |
|---|---|---|
| A — Before study, get the whole-package picture | Lesson names all four authored modules and exposes 44 Concepts. Route communicates the active module’s ordered Objectives. Atlas focuses on one Concept and its bounded neighborhood; it is not a whole-package map. A learner wanting a structural overview across all modules still lacks a compact overview. | UI is insufficient for whole-package orientation; Route is enough for course order. |
| B — During a Lesson, return to an unfamiliar prerequisite | The Concept card lists prerequisite titles; Route provides clickable prerequisite pills that open the Atlas centered on that prerequisite. The path works, but from Lesson it takes a detour through Route. | Atlas is useful for the local prerequisite lookup; Lesson alone supplies names but not exploration. Direct Lesson → Atlas remains a navigation opportunity. |
| C — Find what this Concept connects to next | The selected Concept’s incoming/outgoing explicit prerequisites make local dependency visible; search/recenter and “study this topic” return to content. The seven-child cardiac-structure hub tests the cap. | Atlas is useful for declared prerequisites, not general conceptual association. |
| D — Review untouched / attempted areas | Route and Atlas display Objective answer-attempt counts. This is not Concept coverage or mastery. Thirty-five validation Objectives have no Assessment, so “0 attempts” cannot mean “unseen,” “weak,” or even “ready to assess.” The current UI does not distinguish an Objective with no Assessment from an assessable Objective never attempted. | UI is insufficient; no mastery/coverage claim is justified. |
| E — Search directly for a Concept | Package-scoped title search returns a candidate; selecting it recenters the Atlas. In the math Package, searching for a medical term returns no results, while Bayes is discoverable. | Atlas is needed for direct Concept search/recenter; current title-only search is sufficient for this fixture. |

Package boundary pressure test: a math learner did not express a need to traverse from the probability Package into the medicine Package. The UI correctly kept its search Package-local. Installing and testing two packages is not evidence of demand for a cross-Package Atlas.

## 3. Lesson findings

- The default entry still works as a quiet Lesson with an optional Route action. Route/Atlas do not block reading the first Resource or answering a question.
- Four broad modules form a readable authored outline. A 44-Concept Lesson, however, renders 44 Objective entries, repeats its shared module Resource once per linked Objective (44 links), and shows only eight Assessment links. The full learner page is consequently a long repeated list rather than a concise “current Lesson.”
- The Objective layer is semantically useful: Resources teach and Assessments measure Objectives, while Objectives connect to Concepts. But exposing every intermediate Objective in the default Lesson makes the package model visible at scale. A collapsed module-first Lesson should be tested before changing the model.
- A module Resource linked to 10–12 Objectives is repeated under every one of those Objectives. It remains correct and clickable, but the same action/title dominates the module. This is a concrete resource-linkage/UI density issue, not a reason to change `Resource.teaches` yet.
- Resource → Assessment is still mostly “read a summary, then answer a single-select check.” The assessment questions include basic distinctions and a conceptual integration prompt, but the Runtime does not stage practice inside a resource or provide a multi-step case workflow.

## 4. Route findings

- Keep Route as an optional independent screen for now. It separates author-defined Curriculum order from `Concept.requires`, and shows before/current/next without pretending to infer an adaptive route. In the 10-objective foundation module, the linear ordered list is long but understandable as a course outline.
- The active Objective’s neighboring steps answer “what comes before/next?”; the prerequisite section answers a different “what does the author declare I need first?” question. Separate headings/legend prevent semantic conflation, though both structures can still feel redundant when they repeat near-identical sequences.
- Route has concrete value when a learner asks “why this topic now?” or seeks a prerequisite. For simply opening the current article and answering one quiz, Route is unnecessary.
- Route uses Curriculum when present. It is currently a module-sized sequence rather than an arbitrary learner goal, branching route, or remediation plan. Keep describing it as author-defined order.
- A compact current/before/next strip inside Lesson may eventually reduce navigation steps, but removing the independent Route now would lose the only dedicated place that makes order and prerequisite semantics legible together.

## 5. Atlas findings

- Atlas was most useful for Scenario B/C: inspect an explicit prerequisite or find a named Concept and move back to learning. It was not needed to read a Resource or complete a simple Assessment.
- Six visible prerequisite/dependent Concept buttons is a reasonable provisional cap on this real branch: `心臓の構造` has seven direct dependents, so the UI showed six and explicitly disclosed one more outside the cap. Six was dense but scannable in the list layout; changing the cap on one example would be premature. Keep testing wider fan-out and localized labels.
- Context depth 3 returned 47 typed nodes, including 28 Concepts, in the selected hub neighborhood. The UI still lists at most six direct incoming/outgoing Concepts instead of drawing every returned node. Depth 3 appears adequate for one or two explanatory layers; this fixture does not establish that all Atlas tasks need three hops.
- Objective, Resource, Assessment, and Curriculum-position detail for one Concept is a useful return-to-study layer. A module Resource that teaches many Objectives is shown for each selected Concept, so the detail remains local even though the Lesson list repeats it globally. Four small detail sections are tolerable for one Concept but should not be repeated as a full-graph panel.
- Curriculum position is helpful when a learner asks “where in this module?” but is secondary to prerequisites in Atlas. Retain as a detail layer, not another edge overlay.
- Atlas does not provide whole-Package semantic zoom, modules-as-nodes, or an “unseen” overlay. Do not add a graph canvas based on this test; the bounded list view is sufficient for current explicit relations.

## 6. Authoring / schema friction

The new fixture was authored using the existing stable v0.1 entities and Core validation; no new fields or relation kind were introduced.

| Category | Concrete case and current limitation | Conclusion |
|---|---|---|
| Concept granularity | One Objective must name one Concept. A broad “integrate heart failure and kidney response” outcome would need a chosen home Concept or multiple Objectives. | Current example can be decomposed. Revisit if authors repeatedly need a cross-cutting Objective independent of one Concept. |
| Prerequisite | To make `RAAS` visibly connected to the “heart failure syndrome” Concept, current `requires` would assert a learning prerequisite. In this course a learner can understand the syndrome before studying RAAS; it is a meaningful association / mechanism, not a necessary order. The fixture instead reaches the syndrome via renal perfusion → RAAS/ADH → fluid retention → congestion, which risks presenting one pedagogical route as canonical. | This is a concrete missing non-prerequisite link, but one fixture is not yet enough evidence to add a generic Relation to stable schema. Keep the present path explicitly pedagogical. |
| Curriculum | Four ordered Curriculum lists can express module sequencing; there is no separate module/group object. | Adequate for this validation course; nested units / optional branches have not been shown necessary. |
| Objective | Objective is a good join point for current `Concept → Resource / Assessment`. It becomes visually heavy when 44 are all expanded. | Keep model; reduce default UI exposure before revising schema. |
| Resource linkage | One overview Resource teaches a whole module (10–12 Objectives), and Lesson repeats its link under each. | Current schema can express the association; UI should avoid repeated global presentation. |
| Assessment | Eight exact single-select checks can validate vocabulary and some integration, but cannot capture constructed reasoning, rubric-scored case explanation, or a sequence of intermediate decisions. Thirty-six Objectives have no assessment in this deliberately shallow fixture. | The latter is visible incompleteness, not proof all 44 need a question. Exact evaluation is insufficient for open/case responses. |
| Remediation | A wrong answer gives fixed feedback. It cannot direct the learner to a selected prerequisite or alternate explanation and then return to retry. | Concrete gap for a missed preload/afterload distinction; test one curriculum-level remediation sequence before adding a primitive. |
| Non-quiz activity | A learner should vary preload/afterload and observe a pressure-volume loop change, or annotate a loop and justify which change caused it. Markdown plus exact multiple choice cannot represent the manipulate → observe → explain → retry interaction. | Concrete Activity-like need for simulation/derivation. Prototype outside Core (e.g. static guided task or external capable Resource) first; do not add generic Activity schema now. |

## 7. Concrete missing relations and activities

**Relation case:** the heart-failure syndrome is related to RAAS activation and fluid retention, but RAAS is not a prerequisite for every learner to understand the syndrome. Current `requires` must be interpreted as “learn first / prerequisite.” Using it to mean “pathophysiologically related” overstates the edge. Atlas cannot show this association separately, with its provenance, while preserving that meaning. Do not turn the current directed chain into a canonical causal graph.

**Activity case:** manipulate preload or afterload in a pressure-volume representation, observe a changed loop, then explain the change. A static Resource describes the concepts and Assessment supports a single-select answer only; neither stores the manipulation sequence or gives step-specific feedback. A second candidate is a multi-step case in which the learner identifies a congestion clue, selects a physiological explanation, and revises after feedback. Current Assessment can ask one exact question but does not model a reusable remediation loop.

Both are specific failures of current representation. They justify prototyping interaction shape and collecting teacher/learner evidence, not immediately implementing a universal Relation/Activity/Evidence schema.

## 8. Local observation and performance

No network analytics or product telemetry was added. The existing Desktop E2E harness can time a navigation action with `performance.now()` around real button clicks, Core IPC, and a rendered selector; its pass log records Lesson, Route, Atlas, search, and recenter timing. This is adequate for local comparative QA and leaves learning history untouched. For moderated usability sessions, add an opt-in local debug observer with event names and package digest only if the current E2E hooks prove insufficient; do not persist learner-level click history by default.

E2E measurements use the running Windows WebView in a local debug build. They are wall-clock UI observations from action start until the expected DOM condition, not microbenchmarks; search includes the intentional 180 ms debounce. The cardiovascular flow ran twice while correcting the math test's retained current Objective selection; the table gives both medicine runs as a range. The final complete E2E run passed **329 checks**. The existing 390/1280 px × 100/150/200% text × light/dark presentation matrix also passed for Route and Atlas; the 44-Concept Lesson had no horizontal overflow at 390 px. The four-Concept math Atlas returned the expected Package-local chain, and searching “心不全” produced no math result before Bayes was found.

| Action | Cardiovascular Package | Mathematics Package | Interpretation |
|---|---:|---:|---|
| Lesson open | 569–828 ms | — | Includes click, package IPC, and Lesson render. |
| Route open | 36–40 ms | 54 ms | Medicine's active Curriculum has 10 ordered items; math has four. |
| Atlas open | 260–263 ms | Not separately timed | Local neighborhood, not all 44 Concepts rendered. |
| Search | 511–514 ms | 256 ms | Includes the intentional 180 ms debounce. |
| Recenter | 266–274 ms | Not separately timed | Medicine timing includes click-to-focused-node DOM update. |
| Context bounds | 3 hops / 47 nodes | — | Atlas UI reports requested depth and returned node count. |

## 9. Bundle and mobile notes

The normal Desktop bundle remains 778.90 kB minified JavaScript / 235.55 kB gzip and 51.01 kB CSS / 13.67 kB gzip. E2E-enabled JavaScript is 779.16 kB / 235.63 kB gzip. No dependency was added; Vite still warns that the main chunk exceeds 500 kB. This P1.5 work adds test/package/docs rather than product code and does not materially change that bundle.

Code-splitting is plausible but was not implemented: `App.tsx` imports Authoring, AuthoringReview, AtlasPanel, RoutePanel, Reader, and other panels statically. Authoring/review are the clearest user-action-only lazy chunks; Route/Atlas are smaller but also candidates. KaTeX and syntax-highlighting are statically imported from `main.tsx` / Markdown rendering and may dominate more than Route. Measure per-chunk output before moving imports.

The real-package responsive check is Desktop WebView emulation at 390 px, not Android/iOS certification. Atlas uses buttons/lists and local Core query, which is mobile-friendly in shape; the very long default Lesson and its repeated Resource links are the mobile risk. No ONNX, vector storage, model load, or native mobile identifier was exercised or added.

## 10. UX problems and unnecessary elements

- The 44-Concept Lesson exposes the entity hierarchy and repeats module Resources, so “Lesson” becomes a package-wide catalog. A compact/collapsible module view is a stronger next UX experiment than graph expansion.
- Route is long when it lists all ten objectives for one module; its value is orientation, not rapid article launch.
- Atlas requires Lesson → Route → Atlas to discover a prerequisite during reading; one direct “view prerequisite” link from the Lesson may remove friction without introducing concepts.
- Atlas answer counts are not coverage. Concepts without Assessments cannot be classified as unseen/mastered; labels should remain observations.
- The whole-package Atlas overview, generic Relation, Activity/Evidence schema, Personal Atlas, embedding, cross-package mapping, adaptive path, and graph renderer looked unnecessary for these scenarios.

## 11. Changes not justified yet

Do not change stable v0.1 Package schema or Store, add arbitrary `related` edges, infer mastery from attempt count, transfer state across Packages, add Personal Atlas/embedding/HNSW, turn Route into a recommender, or draw every Concept at once. The fixture reveals an association use case and a simulation use case, but neither has been observed with real learners/authors, repeated across Packages, or compared against a simpler existing-primitive workaround.

## 12. Recommendation for next phase

Run a small moderated author-and-learner study with the existing Lesson → optional Route → local Atlas flow and a real domain author. First prototype the smallest UX change: collapse each Curriculum module by default in Lesson and show a current/before/next summary with a direct prerequisite Atlas action. Keep Atlas bounded. Then test one concrete remediation sequence and one pressure-volume guided interaction as external/task-level prototypes. Add model semantics only if these fail repeatedly and cannot be represented with existing Curriculum / Resource / Assessment links.

Do not proceed to Personal Atlas or embeddings. The math and medical Packages remain correctly isolated, and this exercise produced no learner-reported cross-Package problem.

## Required answers

- **A — Keep an independent Route?** Yes, for now, as an optional orientation screen. It explains the difference between author order and prerequisites. Reconsider after learner observation and test a compact in-Lesson summary.
- **B — Atlas’s highest-value case?** Find/recenter on a specific prerequisite or dependent Concept and inspect its local teaching material; the seven-dependent heart-structure hub tests disclosure of the six-item cap.
- **C — When was Atlas unnecessary?** Reading a Resource, answering its simple quiz, or following an already clear short Curriculum list.
- **D — Did `requires` cause trouble?** Yes, if Atlas is expected to show “heart failure is related to RAAS” without claiming RAAS is a necessary prerequisite. `requires` cannot encode that association separately.
- **E — Concrete need for Activity?** Yes: manipulate preload/afterload and inspect/annotate a pressure-volume loop with step-specific feedback. Current read + exact quiz cannot express the task loop.
- **F — Is Objective appropriate as center?** It remains appropriate as the join point for Concept, Resource, Assessment, and progress in current simple courses. Exposing dozens of Objectives expanded is not an appropriate default Lesson presentation.
- **G — Can Level 1 Authoring create good material without graph input?** The underlying schema can omit edges and let authors write resources/questions, but this validation does not prove the current GUI Level 1 flow can create a strong multi-Concept curriculum without manually constructing graph structure. Need a teacher task study; do not equate typed API capability with Level 1 UX.
- **H — Did Package-local use create a need to cross Package boundaries?** No. The four-Concept math and 44-Concept medical scopes remained understandable independently. Package co-installation alone is not evidence of a personal cross-Package graph problem.
- **I — Evidence for Personal Atlas / embeddings?** No. There is no demonstrated cross-Package learning task or user demand. Keep deferred.
- **J — Highest-value next change?** Prototype a compact/collapsible module-first Lesson with a current/before/next summary and direct prerequisite Atlas action, then observe real learners before changing schema.
