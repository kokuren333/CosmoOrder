# Medicine Package authoring review

Date: 2026-09-23. Package: `examples/medicine-pressure-test`, version `0.2.0`.

## Why the old fixture was replaced

The old fixture mixed English Concept/Resource titles with Japanese prose, used two generic case questions to measure several Objectives at once, and linked broad sources to Resources without making those links visible to learners. Its CC0 label also did not establish reuse rights for the newly authored material. The new Package starts from four learning Objectives for one adult-fluid-assessment sequence; it is neither a translation nor a line-by-line expansion of the former fixture.

## Scope and learning sequence

This Package was authored as a new Japanese-language introduction for medical students and early postgraduate trainees. It teaches observation and clinical reasoning, not patient-specific treatment. The sequence starts with body-fluid compartments, moves to the patterns of water and electrolyte loss, then interprets findings over time, and ends by integrating a short fictional case.

| Concept | Objective | Resource | Assessment |
| --- | --- | --- | --- |
| 体液区分と水の移動 | 細胞内液・間質液・血漿の位置関係を説明し、水と溶質の移動を区別できる | 体液はどこに分布するか | 区画と細胞膜の関係を理解する選択問題 |
| 水分・電解質の喪失と体液量の変化 | 水分喪失とナトリウムを含む細胞外液喪失を区別し、脱水と循環血液量低下を同義に扱わない | 喪失の種類から体液変化を考える | 下痢・摂取低下と正常ナトリウム濃度を解釈する問題 |
| 身体所見と経時変化の読み取り | 病歴、身体所見、バイタルサイン、測定値の推移を組み合わせ、所見の限界も説明できる | 所見を一つずつ、推移とともに読む | 起立時所見を病歴・他所見と合わせる問題 |
| 複数の情報から体液状態を考える | 仮想症例の摂取・喪失歴と経時的所見から、体液状態について妥当な推論と未確定事項を分けて述べられる | 症例の情報を統合する | 摂取、喪失、尿量、起立時所見を統合する問題 |

Concept prerequisites form the same order; the Curriculum lists the four Objectives in that order. Each Objective has one Resource and one Assessment. All four question prompts, choices, and feedback were newly written for this teaching sequence. They use the supported exact single-select format and ask learners to distinguish, apply, or integrate ideas rather than match isolated wording.

## Source selection and Resource-specific evidence

The sources were selected before drafting Resource prose on 2026-09-23. Searches prioritized an open physiology textbook, a public-health source for diarrhoeal losses, a professional clinical reference for volume depletion, and an adult clinical guideline for assessment. The sources do not all support every Resource.

| Source record | Material consulted | Visibility | Linked Resources |
| --- | --- | --- | --- |
| `openstax-fluid-compartments` | OpenStax, *Anatomy and Physiology 2e*, §26.1, body-fluid compartments | public | 体液はどこに分布するか |
| `who-diarrhoeal-disease` | WHO, diarrhoeal disease fact sheet; water and electrolyte losses | public | 喪失の種類から体液変化を考える |
| `merck-volume-depletion` | Merck Manual Professional Edition, volume depletion; serum sodium versus volume, signs, limitations, and interpretation | public | 体液はどこに分布するか／喪失の種類から体液変化を考える／所見を一つずつ、推移とともに読む／症例の情報を統合する |
| `nice-cg174` | NICE CG174, recommendations for adult fluid and electrolyte assessment | attribution_only | 所見を一つずつ、推移とともに読む／症例の情報を統合する |
| `private-author-note` | This review document as authoring provenance; used to exercise privacy filtering | private | 症例の情報を統合する |

The physiology explanation uses OpenStax's compartment model without copying its text or quantitative proportions. Merck also supports the first Resource's distinction between serum sodium concentration and body-fluid volume. WHO is cited only for losses accompanying diarrhoeal disease. Merck supports distinctions and cautions about physical signs. NICE supports the multi-source assessment frame; its attribution is retained while its locator is not shown to learners. The private authoring review is not evidence for a medical claim. Its local locator, title, and citation must never appear in the learner interface or distribution.

## How the Resources were written

The previous pressure-test prose and assessment bank were not translated or paraphrased. They were removed. The four new Markdown Resources start from the linked source material and organize the concepts as follows:

1. **体液区分** defines intracellular fluid, interstitial fluid, and plasma; a comparison table helps distinguish locations; a short reading procedure separates concentration from amount.
2. **喪失** explains mixed water/electrolyte losses and separates total-water deficit from extracellular-volume reduction; a table connects common scenarios to what they do and do not establish.
3. **所見** places adult assessment findings in context, includes the limits of oral dryness and orthostatic changes, and treats discordant values as questions to investigate rather than noise.
4. **統合** walks through a fictional adult case, distinguishing observations, plausible interpretation, and unanswered questions in a three-column table.

The table and ordered-list markup exercise the existing renderer as part of clinically natural content. No citation list is embedded in Markdown: each displayed reference is derived from the Resource's `source_ids` and the Package source registry.

## Authoring workflow observations

- **Skill:** `skills/research-and-create-resource/SKILL.md` provided the sequence for package inspection, capability-aware source acquisition, normalization, Resource-specific linking, quality checks, and reporting.
- **CLI:** `validate`, `lint`, `build`, and validation of the resulting archive were used. `inspect`, `query`, and `context` were used to examine the Package graph and teaching scope. CLI build/install remain the supported way to place the Package in the Desktop library.
- **Direct JSON edits:** the source registry is edited in `osmium.json`; concepts, objectives, curriculum ordering, Resource metadata/`source_ids`, and assessment schema objects are ordinary JSON. The current CLI has no source registration/list operation, so source normalization and linking required direct edits in `osmium.json` and `entities/resources.json`/`entities/assessments.json`.
- **Direct Markdown edits:** all four Resource bodies were written directly under `content/`. This remains the appropriate authoring surface for full explanations, tables, and lists.
- **Friction:** there is no command to add a Source while simultaneously linking it to a Resource and previewing its sanitized learner-visible fields. Adding one focused `source add`/`source list` workflow could reduce registry/reference mistakes; broad CRUD is not justified by this Package alone. No MCP endpoint was needed.

## Validation boundary

`validate`, `lint`, and a successful build establish structural consistency and privacy sanitization, not clinical peer review, guideline endorsement, or licensing advice. Resource `license` metadata explicitly remains unset. The text avoids treatment instructions and threshold-based decisions; subject-matter review is still needed before use as a clinical teaching reference.
