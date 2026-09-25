# research-and-create-resource

## Inputs

- A Package source directory and target Concept or Objective.
- Learner level, requested language (BCP 47), scope, and user-provided references.
- Actually available capabilities: local files, shell, MCP, browser/search. Never assume one exists.

## Goal

Produce an evidence-grounded, readable Markdown Resource, link it to its learning Objective and to the References that support it, and leave the Package reviewable. Authoring input and Evidence are different things; keep them apart.

## Capabilities

Use existing `osmium init`, `inspect`, `query`, `context`, `validate`, `lint`, `build`, and the three `osmium reference` operations (`add`, `attach`, `list`). Edit ordinary JSON/YAML/Markdown files deterministically for everything else. CLI does not fetch remote URLs or copy local files. CLI/MCP are capabilities, this Skill is workflow policy, and the model supplies reasoning.

## The separation this workflow must not blur

```text
Reference              = a knowledge resource a learner or distribution can reach
Evidence               = the relation from a Resource/Assessment to its References
Authoring Provenance   = the inputs and history used to produce the lesson
```

Reading a document to write a lesson is **not** the same act as publishing that document as the lesson's evidence. Both can be true of the same document. They are still different records, and they belong in different places.

| What it is | Where it goes | How it is written |
| --- | --- | --- |
| A guideline, paper, textbook or official document the lesson rests on | the Package, as a Reference | `osmium reference add` then `osmium reference attach` |
| A local PDF, private URL, draft, search result, or note about the process | the Authoring Workspace, `.osmium/` | ordinary files; never in the manifest |
| A file that must travel with the Package | a Package-relative asset | `--kind package_asset` with a relative `--locator` |

`.osmium/` is skipped by the package loader and gitignored, so authoring input cannot reach a build or a commit by accident. Do not set `visibility: private` as a substitute for using it: that field exists so an old package stays loadable, and a private record still had to be authored inside the Package directory.

## Procedure

1. Inspect and validate the Package; read the target Concept/Objective and prerequisite neighborhood.
2. Inventory capabilities. Acquire material in this order: user-provided sources, package-contained material, accessible local files, available browser/search. Do not invent evidence for unsupported claims.
3. Record everything you read as authoring input in `.osmium/`, including its origin. This is a work log, not a deliverable, and it keeps the research step honest without publishing it.
4. Decide, per document, whether it is *Evidence* for the content or only *input*. A document becomes a Reference only when the lesson actually rests on it and a learner may see it. Register it with `osmium reference add`, choosing:
   - `--kind` for how a reader resolves it (`url`, `doi`, `isbn`, `citation`, `package_asset`, `manual`);
   - `--type` for what it is (`webpage`, `article`, `book`, `guideline`, `dataset`, `document`, `other`);
   - `--publisher`, `--author`, `--published-at`, `--updated-at`, `--accessed-at`, `--edition`, `--version` when they are known. Omitting an unknown date is correct; inventing one is not;
   - `--visibility public` when the URL may be shown, `attribution_only` when the record may be credited but the locator must not be published. Never put a signed or credential-bearing URL in a public locator, and do not treat the credential check as a guarantee — read the URL before registering it.
5. Draft original Markdown in the requested language or the effective Package language. Explain the core idea and, where useful, intuition, prerequisites, examples, counterexamples, distinctions, common errors, context, and summary. These are heuristics, not mandatory headings or fixed length.
6. Attach Evidence with `osmium reference attach <ref-id> --resource <resource-id>`; do the same with `--assessment <id>` when an item's claims depend on a specific Reference. Do not attach a Reference the lesson does not actually use.
7. State the reuse policy honestly: `license_status: known` with a real license, or `license_status: unknown`. Never write placeholder prose such as `再利用条件は未設定。` to fill the field — lint now reads meaning, not presence, and will report it.
8. Run `osmium validate --json` and `osmium lint --json`; resolve errors and privacy findings. Review warnings in context. Build to a new path and inspect the distribution.
9. Return changed files, the Reference/Evidence decisions, what stayed in `.osmium/`, and remaining evidence gaps for human review.

## Hard constraints

- Schema and entity/Reference references validate; every `evidence_reference_ids` entry resolves.
- Language tags are valid BCP 47.
- Distributable locators are portable, contain no embedded credentials, and carry no credential-like query parameter. Ordinary publication queries (`?id=`, `?lang=`, `?article=`) are allowed and should not be hidden.
- Authoring input — absolute paths, private URLs, drafts, notes, agent workflow metadata — never enters the manifest, a distribution, or learner-facing prose.
- Private provenance and author-machine information do not enter a distribution.
- Do not claim expert review, clinical approval, rights clearance, or certainty without evidence.

## What NOT to claim

Osmium provides **resource-level traceability**: a learner can see which References a Resource or Assessment is associated with. It does **not** provide **claim-level verifiability**: no mechanism ties a specific sentence to a specific Reference. Do not describe a package, a Resource or this workflow's output as "claim-verified", "sourced sentence by sentence", or "fully referenced". An uncited paragraph can sit between two well-chosen References.

## Capability-aware degradation

If search is unavailable, use only provided, bundled, or accessible local evidence. Avoid unsupported or time-sensitive assertions; return a structured needs-sources result such as `{"status":"needs_sources","missing":["current clinical guideline for the target population"]}`. Continue only with grounded portions. Research and authoring may be separate agents: pass Reference records, `.osmium/` input summaries and unresolved evidence needs, not tool histories.

## Assessment quality

When this workflow also creates or revises items:

- every item states the Objective it measures (`measures`), and `cognitive_level` (`recall`, `discrimination`, `application`, `integration`, or another framework's label) so a reviewer can see what the item asks;
- a package that only asks a learner to restate the lesson's own sentences measures recognition of phrasing, not understanding. Include items that require distinguishing similar concepts, applying a rule to a small case, and integrating several findings;
- distractors must be plausible enough to be tempting. An obviously absurd option does not measure anything, and lint reports reused distractor text across items;
- one question per concept is usually not enough, and four variants of the same sentence is not coverage.

## Quality and failure

A learner should usually understand the target idea from the Resource itself. Use examples and distinctions where they help; avoid padding, unsupported details, redundancy, or fixed word/question counts. If validation fails, report exact diagnostics and do not claim a shareable build. For rights or source uncertainty, request human review instead of guessing. Lint and validation do not establish factual correctness or lawful reuse.
