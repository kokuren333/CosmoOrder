# P1 Learning Navigation Implementation

Date: 2026-09-25  
Scope: learner-side Lesson → Route → Package-local Atlas vertical slice. The v0.1 Package schema and machine identifiers remain unchanged.

## 1. UX objective

Keep Lesson as the ordinary, quiet starting screen, with one optional “View learning route” action. Route explains the author-defined curriculum position and declared Concept prerequisites. Atlas zooms into one selected Concept and offers a bounded local neighborhood, related learning material, and a path back to study. This is an interaction experiment, not evidence that graph views improve learning outcomes.

## 2. Lesson / Route / Atlas architecture

- **Lesson** remains the existing `LessonView` projection, organized around Concepts, Objectives, Resources, Assessments, and Curriculum. Its original first-Resource action remains available; the only new learner entry is an optional Route action.
- **Route** uses the opened `LessonView` and observed `ObjectiveProgress`. It displays only explicit `Curriculum.objectives` ordering and `Concept.requires`. It does not produce recommendations.
- **Atlas** calls the existing `package_context` read API for the selected Concept with depth 3 and node limit 64. It renders at most six prerequisite and six dependent Concept buttons. Resource/Objective/Assessment/Curriculum details appear as layers below the selected Concept rather than as an all-package graph.
- **Concept search** calls the Core-backed `package_concept_search` command. Each request is package/version-scoped and returns at most 20 title matches. The UI debounces typing; it does not build an independent frontend knowledge index.
- Opening a Resource from Atlas and returning restores the selected Atlas Concept; learners can then return to Route or Lesson.

## 3. Changed files

- `crates/osmium-core/src/query.rs`: bounded `search_concepts` result and test; existing `context` contract remains the local Atlas read model.
- `crates/osmium-store/src/runtime.rs`: version-scoped Runtime search adapter.
- `apps/desktop/src-tauri/src/commands.rs`, `apps/desktop/src-tauri/src/lib.rs`: `package_concept_search` command registration.
- `apps/desktop/src/client.ts`, `apps/desktop/src/types.ts`: typed Desktop adapter.
- `apps/desktop/src/learningNavigation.ts`: explicit Route projection helpers.
- `apps/desktop/src/App.tsx`: panel navigation and context/resource return state.
- `apps/desktop/src/components/Lesson.tsx`: optional Route action.
- `apps/desktop/src/components/RoutePanel.tsx`: ordered Route plus separate prerequisite section.
- `apps/desktop/src/components/AtlasPanel.tsx`: bounded local Atlas, search, related material, and return actions.
- `apps/desktop/src/styles/app.css`: responsive, button/list-based layouts; no hover-only operation.
- `apps/desktop/tests/learning-navigation.test.ts`, `apps/desktop/tests/presentation.test.ts`: Route and Atlas semantics/presentation coverage.
- `apps/desktop/e2e/desktop-e2e.mjs`: temp-home-only data guard and learner vertical-slice coverage.

No graph visualization dependency was added. The frontend dependency set still uses React, Tauri APIs, and existing Lucide icons; the Atlas is semantic HTML lists and buttons rather than a canvas graph.

## 4. Reused P0 / Core primitives

Reused `PackageContextView`, `package_context`, `Runtime::context`, and Core `context`. These already expose bounded nodes and typed relation records for `requires`, `concept`, `teaches`, `measures`, and `orders`; entity payloads retain the authored `Curriculum.objectives` sequence. This is sufficient for the P1 Concept neighborhood and its learning-detail layers. Search adds only a small bounded read endpoint because the existing page query does not search titles.

## 5. Route semantics

- `Curriculum.objectives` is the author’s ordered list. Route shows before/current/next from this list only.
- `Concept.requires` is a package-local knowledge prerequisite. Route labels it separately and uses a dashed panel and explicit text legend.
- Neither relation is synthesized from the other. When no Curriculum includes the current Objective, before/after stay empty and the UI says order is not defined.
- State is the stored attempt count per Objective. Labels say “not attempted” or “N attempts”; the Route does not claim mastery, understanding, weak prerequisites, or adaptive recommendations.
- “Why next” text cites the actual Curriculum order and named declared prerequisites.

## 6. Atlas rendering strategy

The authoritative graph boundary is the currently open Package. The selected Concept is centered in a three-lane neighborhood: required-before, selected topic, and required-by. The Context result bounds total input (Core caps include maximum depth, node count, and serialized bytes); each side additionally displays no more than six neighbor buttons. Search recenters on a selected Concept rather than expanding the whole graph. Objectives, Resources, Assessments, and Curriculum placement are details/layers, not a second inferred graph. `context.truncated` is disclosed to the learner.

## 7. Learning-state presentation

The only overlay is observed answer count by Objective. It is rendered separately from Package-authored relation labels and text. Correctness percentages or mastery are not calculated in Atlas/Route. Existing Progress remains its current observed-state projection.

## 8. Empty / large Package behavior

A Concept with no incoming or outgoing `requires` edges shows a calm empty-state message and still shows any objective/resource layers returned by Context. An absent Curriculum does not imply sequence. Atlas neighbor lanes are capped at six per side; Context itself is bounded at 64 nodes for this UI request. Search results are capped at 20. The E2E fixture covers both a one-Concept/no-prerequisite Package and the five-Concept `arithmetic-expanded` Package. No claim is made for 100k-Concept performance; the current search is a bounded-response linear title scan inside Core, so this should be profiled before supporting very large packages.

## 9. Mobile considerations

The interface uses standard buttons, lists, labels, and responsive CSS. It has no hover requirement, right-click interaction, or fixed canvas. At narrow widths the neighborhood and detail columns stack, with comfortable button heights. This keeps the interaction path portable, but does not certify a Tauri Android/iOS build or full mobile visual QA.

## 10. Bundle / performance impact

No dependency was added. Final normal production build after P1: **778.90 kB minified JavaScript / 235.55 kB gzip**, CSS **51.01 kB / 13.67 kB gzip**. P0 implementation notes recorded a 755.70 kB JavaScript / 229.85 kB gzip baseline: approximately **+23.20 kB minified and +5.70 kB gzip**. This remains over Vite’s 500 kB advisory threshold; P1 did not worsen it through a graph package, but it should remain a future chunking task.

Desktop interaction measurements from the final E2E run: Route open **90 ms**, Atlas open **274 ms**, prerequisite recenter **9 ms**, and backend Concept search **269 ms**. Search timing includes the intentional 180 ms typing debounce and IPC/read-model latency. Values are machine/build-specific observations, not benchmark targets. The preceding run measured 43 / 266 / 10 / 261 ms; the difference is ordinary local run variance, not a performance regression signal.

## 11. Tests

- Route helper tests cover current position, before/after, explicit prerequisites, absent Curriculum behavior, and observed attempt counts.
- Presentation tests cover current step, separation labels/legend, selected Atlas Concept, prerequisite and dependent neighbors, Objective/Resource/Assessment/Curriculum layers, sparse graph state, and six-item visible-neighbor cap.
- Desktop E2E extends the existing package/import/history flow with the expanded graph navigation path and a sparse Package path. It requires `--home` to be a dedicated child directory under the OS temporary directory; generated authoring source and screenshots are inside that home.
- `cargo test --workspace`: pass (all unit, integration, desktop, and doc test binaries; 0 failures).
- `npm run typecheck`, `npm run lint`, `npm test`: pass (29 Desktop unit/presentation tests).
- `npm run build`: pass; final normal build was 778.90 kB / 235.55 kB gzip; E2E-enabled build was 779.16 kB / 235.63 kB gzip.
- Desktop E2E: **302 checks passed**, including rich/sparse Atlas paths; Route/Atlas layout at 390 and 1280 px, 100/150/200% text, and light/dark themes; the existing uninstall/history scenario; and established authoring/import/restart/renderer coverage.
- E2E data home was a fresh `%TEMP%\cosmoorder-p1-*` child; all generated authoring sources, archives, user data, and screenshots were inside that temporary home. E2E refuses a home outside the OS temp root.

## 12. Discovered architecture gaps

1. Lesson and Route currently receive the full package learner projection; Atlas correctly uses bounded Context. This is not a graph renderer leak, but a future large-package reader may want paged Lesson sections.
2. Concept search is deterministic and package-scoped, but currently scans titles linearly. Measure before introducing an index/ANN structure.
3. A Resource is attached to Objectives, and Objectives to Concepts. A Concept without linked Objectives consequently has no derived direct Resource action. This is honest to the current schema; authoring better links may matter more than a generic Relation schema.
4. This slice shows explicit package structure and observed attempts but does not prove that Route or Atlas changes learning outcomes. Usability research is needed before Personal Atlas investment.

## 13. Deliberately not implemented

No Personal Atlas, cross-Package mappings, embeddings, mastery model, recommendations, global ontology, new schema primitive, generic Relation, Activity/Evidence model, graph editor/canvas, module grouping, or graph dependency was added. No package format, persistence, crate, executable, bundle ID, or data path was renamed.

## 14. Recommendation for P2

Do not begin Personal Atlas or embeddings yet. First use the Package-local flow with actual authors/learners and refine the Route/Atlas wording and transitions. If people repeatedly need to compare Packages, prototype a separate read-only derived mapping view, with provenance and no source mutation; do not transfer mastery through a Concept match. Keep the current Package-local Atlas if it helps answer “why this lesson / what depends on this”; prefer a linear outline when users do not need the extra zoom level.

## Answers after implementation

- **A — Is Lesson → Route → Atlas natural?** The interaction is coherent in the implemented slice and keeps the default Lesson action first. The E2E result is the final check; “natural” still needs learner observation.
- **B — Orientation or decoration?** The Atlas is action-oriented in this implementation because it names explicit prerequisites, exposes related resources, and returns to study. Its learning value remains unproven.
- **C — How useful are current primitives?** Sufficient for author-defined sequence, prerequisite explanation, local neighbors, objective/resource/assessment association, and Package-local orientation. They do not express arbitrary relation types or dynamic sequencing.
- **D — Is a Relation schema extension needed?** No evidence from this vertical slice. Existing `requires` and curriculum ordering suffice for the narrow use case; collect a concrete authoring case before expanding.
- **E — Does this justify Personal Atlas?** Not yet. Package-local value needs learner validation first.
- **F — What feels CosmoOrder-specific?** Moving between the learner’s current objective, its author-defined route, and an explicit package knowledge prerequisite while retaining saved attempts and a return-to-study action.
- **G — Personal Atlas/embedding or polish?** Polish and validate Package-local UX first; defer cross-Package mapping and embedding.
