# research-and-create-resource

## Inputs

- A Package source directory and target Concept or Objective.
- Learner level, requested language (BCP 47), scope, and user-provided references.
- Actually available capabilities: local files, shell, MCP, browser/search. Never assume one exists.

## Goal

Produce an evidence-grounded, readable Markdown Resource, link it to its learning Objective and normalized source records, and leave the Package reviewable.

## Capabilities

Use existing `osmium init`, `inspect`, `query`, `context`, `validate`, `lint`, and `build` JSON operations. Edit ordinary JSON/YAML/Markdown files deterministically. CLI does not fetch remote URLs or copy local files. CLI/MCP are capabilities, this Skill is workflow policy, and the model supplies reasoning.

## Procedure

1. Inspect and validate the Package; read the target Concept/Objective and prerequisite neighborhood.
2. Inventory capabilities. Acquire evidence in this order: user-provided sources, package-contained material, accessible local files, available browser/search. Do not invent evidence for unsupported claims.
3. Normalize source metadata independent of acquisition method. Choose `public`, `attribution_only`, or `private`; never copy an absolute path, credential, private URL, or machine-specific detail into distributable metadata or prose. Build must remove private records.
4. Draft original Markdown in the requested language or effective Package language. Explain the core idea and, where useful, intuition, prerequisites, examples, counterexamples, distinctions, common errors, context, and summary. These are heuristics, not mandatory headings or fixed length.
5. Add resource-specific `source_ids`. Link Assessments only when evidence supports their claims. Separate observation from inference and flag uncertainty.
6. Run `osmium validate --json` and `osmium lint --json`; resolve errors and privacy findings. Review warnings in context. Build to a new path and inspect the distribution.
7. Return changed files, visibility decisions, validation/lint/build results, and missing evidence for human review.

## Hard constraints

- Schema and entity/source references validate.
- Language tags are valid BCP 47.
- Distributable locators are portable and contain no credentials, query tokens, or OS absolute paths.
- Private provenance and author-machine information do not enter a distribution.
- Do not claim expert review, clinical approval, rights clearance, or certainty without evidence.

## Capability-aware degradation

If search is unavailable, use only provided, bundled, or accessible local evidence. Avoid unsupported or time-sensitive assertions; return a structured needs-sources result such as `{"status":"needs_sources","missing":["current clinical guideline for the target population"]}`. Continue only with grounded portions. Research and authoring may be separate agents: pass normalized source records and unresolved evidence needs, not tool histories.

## Quality and failure

A learner should usually understand the target idea from the Resource itself. Use examples and distinctions where they help; avoid padding, unsupported details, redundancy, or fixed word/question counts. If validation fails, report exact diagnostics and do not claim a shareable build. For rights or source uncertainty, request human review instead of guessing. Lint and validation do not establish factual correctness or lawful reuse.
