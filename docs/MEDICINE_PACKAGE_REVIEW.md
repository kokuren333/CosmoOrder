# Medicine Package authoring review

Date: 2026-09-23, revised after the Reference/Evidence separation. Package: `examples/medicine-pressure-test`.

## Why the old fixture was replaced

The old fixture mixed English Concept/Resource titles with Japanese prose, used two generic case questions to measure several Objectives at once, and linked broad sources to Resources without making those links visible to learners. Its CC0 label also did not establish reuse rights for the newly authored material. The current Package starts from four learning Objectives for one adult-fluid-assessment sequence; it is neither a translation nor a line-by-line expansion of the former fixture.

## Why the privacy source was removed

The previous revision registered `private-author-note` — this review document — as a `private` source linked from a Resource, purely so that privacy filtering had something to filter. That made the medical fixture a privacy test rig. It also put an author-machine path inside a teaching package's manifest, which is the pattern the Authoring Workspace exists to prevent.

The record is gone. What replaced it:

- The **medicine fixture** now carries learner-facing References only. Every record is `record_visibility: public`, `locator_visibility: public`, has a locator and a citation, and is evidence for a medical claim.
- Privacy and visibility pressure moved to **`examples/provenance-visibility-pressure-test`**, which contains a public reference with a safe publication query, a hidden-locator reference that keeps its citation, and a package asset with a `content_hash`, alongside resources with `license_status: known` and `license_status: unknown`.
- **Authoring provenance** — local paths, private URLs, author-only notes — is now tested where it actually lives: `crates/osmium-package/tests/distribution.rs` writes an `.osmium/` directory into a temporary copy and asserts that no byte of it reaches the build. See [`AUTHORING_WORKSPACE.md`](AUTHORING_WORKSPACE.md).

## Scope and learning sequence

This Package is a new Japanese-language introduction for medical students and early postgraduate trainees. It teaches observation and clinical reasoning, not patient-specific treatment. The sequence starts with body-fluid compartments, moves to the patterns of water and electrolyte loss, then interprets findings over time, and ends by integrating a short fictional case.

| Concept | Objective | Resource | Assessment | Level |
| --- | --- | --- | --- | --- |
| 体液区分と水の移動 | 細胞内液・間質液・血漿の位置関係を説明し、水と溶質の移動を区別できる | 体液はどこに分布するか | 区画と細胞膜の関係を理解する選択問題 | `discrimination` |
| 水分・電解質の喪失と体液量の変化 | 水分喪失とナトリウムを含む細胞外液喪失を区別し、脱水と循環血液量低下を同義に扱わない | 喪失の種類から体液変化を考える | 下痢・摂取低下と正常ナトリウム濃度を解釈する問題 | `application` |
| 身体所見と経時変化の読み取り | 病歴、身体所見、バイタルサイン、測定値の推移を組み合わせ、所見の限界も説明できる | 所見を一つずつ、推移とともに読む | 起立時所見を病歴・他所見と合わせる問題 | `application` |
| 複数の情報から体液状態を考える | 仮想症例の摂取・喪失歴と経時的所見から、体液状態について妥当な推論と未確定事項を分けて述べられる | 症例の情報を統合する | 摂取、喪失、尿量、起立時所見を統合する問題 | `integration` |

Concept prerequisites form the same order; the Curriculum lists the four Objectives in that order. Each Objective has one Resource and one Assessment. All four question prompts, choices, and feedback were newly written for this teaching sequence. They use the supported exact single-select format and ask learners to distinguish, apply, or integrate ideas rather than match isolated wording.

## Assessment quality

The items are the part of this fixture that changed most in the last revision, and the change was about what a wrong answer tells you, not about difficulty.

| Item | What it measures | Why the distractors are informative |
| --- | --- | --- |
| `compartments.check` | Distinguishing compartment membership and membrane separation | The wrong options are the two common inversions — treating plasma as intracellular, and treating water as immobile across a concentration gradient. A learner who holds either misconception is identifiable from the answer. |
| `losses.check` | Applying concentration-versus-content to a case with normal serum sodium | The tempting wrong answer is the intuitive one: "sodium is normal, so volume is normal." The item exists to separate that inference from the correct reading. |
| `findings.check` | Interpreting two findings without over-claiming | The wrong options are the two failure modes the lesson names: certainty from two signs, and dismissing volume change because one blood pressure was normal. |
| `integration.check` | Integrating several findings and stating what remains unknown | The wrong options are "one preserved value rules it out" and "one sign determines the diagnosis and treatment." Both are the errors the whole sequence is built to prevent. |

Every item declares `measures` (its Objective) and `cognitive_level`, and every item carries `evidence_reference_ids`. No item is a restatement of a lesson sentence with one absurd option, which is the pattern `OSM_LINT_ASSESSMENT_SHALLOW` and `OSM_LINT_REUSED_DISTRACTOR` were added to surface; the expanded arithmetic fixture still shows it.

## Reference selection and Resource-specific Evidence

The References were selected before drafting Resource prose. Searches prioritized an open physiology textbook, a public-health source for diarrhoeal losses, a professional clinical reference for volume depletion, and an adult clinical guideline for assessment. The References do not all support every Resource.

| Reference | Material | Material type | Linked Resources | Linked Assessments |
| --- | --- | --- | --- | --- |
| `openstax-fluid-compartments` | OpenStax, *Anatomy and Physiology 2e*, §26.1, body-fluid compartments | `webpage` | 体液はどこに分布するか | `compartments.check` |
| `who-diarrhoeal-disease` | WHO, diarrhoeal disease fact sheet; water and electrolyte losses | `webpage` | 喪失の種類から体液変化を考える | `losses.check` |
| `merck-volume-depletion` | Merck Manual Professional Edition, volume depletion; serum sodium versus volume, signs, limitations, and interpretation | `webpage`, `updated_at: 2026-06` | 体液はどこに分布するか／喪失の種類から体液変化を考える／所見を一つずつ、推移とともに読む／症例の情報を統合する | all four |
| `nice-cg174` | NICE CG174, recommendations for adult fluid and electrolyte assessment | `guideline`, `version: CG174` | 所見を一つずつ、推移とともに読む／症例の情報を統合する | `findings.check`, `integration.check` |

The physiology explanation uses OpenStax's compartment model without copying its text or quantitative proportions. Merck also supports the first Resource's distinction between serum sodium concentration and body-fluid volume. WHO is cited only for losses accompanying diarrhoeal disease. Merck supports distinctions and cautions about physical signs. NICE supports the multi-source assessment frame and is a public guideline, so its locator is published rather than hidden.

What these References establish is **resource-level traceability**: a learner can see which guidelines and references support a lesson. They do **not** establish claim-level verifiability. No sentence in the Markdown is individually pinned to a Reference, and a reviewer cannot yet ask "what supports this paragraph?" — see [`SOURCES_AND_PROVENANCE.md`](SOURCES_AND_PROVENANCE.md#resource-level-traceability-and-claim-level-verifiability).

## How the Resources were written

The previous pressure-test prose and assessment bank were not translated or paraphrased. They were removed. The four Markdown Resources start from the linked material and organize the concepts as follows:

1. **体液区分** defines intracellular fluid, interstitial fluid, and plasma; a comparison table helps distinguish locations; a short reading procedure separates concentration from amount.
2. **喪失** explains mixed water/electrolyte losses and separates total-water deficit from extracellular-volume reduction; a table connects common scenarios to what they do and do not establish.
3. **所見** places adult assessment findings in context, includes the limits of oral dryness and orthostatic changes, and treats discordant values as questions to investigate rather than noise.
4. **統合** walks through a fictional adult case, distinguishing observations, plausible interpretation, and unanswered questions in a three-column table.

The table and ordered-list markup exercise the existing renderer as part of clinically natural content. No citation list is embedded in Markdown: each displayed reference is derived from the Resource's `evidence_reference_ids` and the Package Reference registry.

## Reuse policy

`license_status: "unknown"` on all four Resources, with no `license` string. The authored prose has no declared reuse terms, and asserting CC0 for material whose rights were never established is exactly the field-presence-without-meaning problem `OSM_LINT_LICENSE_UNKNOWN` now reports. `unknown` is the honest status, and lint says so on every build. The referenced third-party material keeps its own terms, which are described by the Reference records and not by the Resource license field.

## Authoring workflow observations

- **Skill:** `skills/research-and-create-resource/SKILL.md` provided the sequence for package inspection, capability-aware acquisition, the Reference-versus-input decision, Resource-specific linking, quality checks, and reporting.
- **CLI:** `validate`, `lint`, `build`, and validation of the resulting archive were used. `inspect`, `query`, and `context` were used to examine the Package graph and teaching scope. `reference add`/`attach`/`list` now exist and were used for later edits; this fixture predates them.
- **Direct JSON edits:** concepts, objectives, curriculum ordering, Resource metadata/`evidence_reference_ids`, and assessment objects are ordinary JSON. With `reference add` and `reference attach` available, the Reference registry and the Evidence relation no longer need two hand-edited documents that nothing checks against each other.
- **Direct Markdown edits:** all four Resource bodies were written directly under `content/`. This remains the appropriate authoring surface for full explanations, tables, and lists.
- **Remaining friction:** there is no command to create a Resource or an Assessment, so a new item still means hand-editing `entities/assessments.json` and re-running `validate`. That is tolerable; `reference add`/`attach` were the operations that actually caused mistakes, and broad CRUD is not justified by this Package alone. No MCP endpoint was needed.

## Validation boundary

`validate`, `lint`, and a successful build establish structural consistency, Reference resolution and privacy sanitization, not clinical peer review, guideline endorsement, or licensing advice. Resource reuse terms explicitly remain unknown. The text avoids treatment instructions and threshold-based decisions; subject-matter review is still needed before use as a clinical teaching reference.
