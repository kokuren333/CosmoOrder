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

Note-taking tools help organize knowledge, and flashcard tools help with recall practice; neither category alone provides a structured learning runtime for reusable curricula, resources, assessments, and progress. Osmium explores an open infrastructure where individuals, educators, institutions, and specialist communities can build, inspect, share, or privately manage learning packages. Open packages can reduce access barriers, while organizations can use the same format for their own materials. The project does not require commercial education to be excluded.

AI may reduce the cost of organizing permitted knowledge sources into reusable learning material. Generated content still needs source tracking, cross-checking, and human review. Osmium does not include an AI generation service or automatic rights assessment.

## Schema pressure-test notes

The current Objective has one `concept` field, while an integrated clinical objective may span several concepts. Keep the current relation for this test and record the modeling pressure rather than silently duplicating or broadening it. Audio can currently be discussed in Markdown but there is no typed media asset reference. Tables and equations belong to renderer support rather than new educational entity types. Revisit multi-concept objectives, media references, and assessment response types after inspecting packages from multiple domains.
