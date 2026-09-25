# Package authoring と lint

Status: current implementation. 歴史的な Phase 記録は `IMPLEMENTATION_PLAN.md` と `PHASE2B_HANDOFF.md` に残す。

`osmium validate <path> --json` はschema、参照、循環、本文と安全なpathを検証する。失敗は `ok: false`、exit 1（非互換は4）。`osmium lint <path> --json` はvalidateを通ったPackageの静的解析であり、warningがあっても `ok: true`、exit 0。stdoutは単一のversion 0.1 JSON envelope、stderrは人間向け診断行。`diagnostics` の既存fieldは維持し、lintには省略可能な `entity_type` と `entity_id` を追加した。診断はfile、entity ID、code、messageで安定ソートする。

## 3つの層を混ぜない

```text
schema validation = deterministic hard rules
lint              = explainable heuristic warnings
review            = human / model qualitative evaluation
```

heuristicはhard errorにしない。逆に、確定的に判定できる規則はheuristicのままにしない。この境界は実装でも守っている。`OSM_LINT_LICENSE_UNKNOWN` や `OSM_LINT_ASSESSMENT_SHALLOW` は「教材が不完全である」という主張ではなく、「著者が一度確認すべき点がある」という説明可能な警告である。

## 硬い検証（`validate`）

| Code | 条件 |
| --- | --- |
| `OSM_SCHEMA` | schema違反。Assessment定義に `score`／`response`／`correct`／`timestamp` などを書くとここで落ちる（definition/result分離の構造的強制） |
| `OSM_REFERENCE` | Objective, Resource, Assessment, Curriculum, Concept参照の不整合 |
| `OSM_SOURCE_REFERENCE` | `evidence_reference_ids` がregistryに存在しない、重複、またはregistryなしでevidenceを宣言している |
| `OSM_DUPLICATE_SOURCE` / `OSM_DUPLICATE_ID` / `OSM_DUPLICATE_OPTION` | ID重複 |
| `OSM_SOURCE_LOCATOR` | 公開Referenceにlocatorがない、`package_asset` のlocatorが安全な相対pathでない |
| `OSM_SOURCE_PRIVACY` | 配布されるlocatorに埋め込みcredentialまたはcredential様query parameterがある |
| `OSM_ANSWER` | `evaluation.answer` が存在しないoptionを指す |
| `OSM_CYCLE` / `OSM_PATH` / `OSM_PATH_COLLISION` / `OSM_LANGUAGE` / `OSM_CAPABILITY` | 前提循環、非可搬path、path衝突、BCP 47不正、未対応capability |

## Lint code

| Code | 条件 | 層 |
| --- | --- | --- |
| `OSM_LINT_OBJECTIVE_NO_RESOURCE` | Objectiveを教えるResourceがない | coverage |
| `OSM_LINT_OBJECTIVE_NO_ASSESSMENT` | Objectiveを測るAssessmentがない | coverage |
| `OSM_LINT_ORPHAN_CONCEPT` | ConceptがObjectiveにも他Conceptのprerequisiteにも参照されない | coverage |
| `OSM_LINT_OBJECTIVE_NOT_IN_CURRICULUM` | Curriculumが存在するがObjectiveをどれも列挙しない。Curriculumなしなら抑制 | coverage |
| `OSM_LINT_PREREQUISITE_ORDER` | 同じCurriculum内で、明示されたConcept prerequisiteの最初のObjectiveが従属Conceptの最初のObjectiveより後にある | coverage |
| `OSM_LINT_METADATA` | Resourceの `creator` または `attribution` が欠ける | metadata |
| `OSM_LINT_RESOURCE_SHORT` | Markdown本文が240文字未満 | metadata |
| `OSM_LINT_LANGUAGE` | effectiveなPackage／Resource languageがない | metadata |
| `OSM_LINT_LICENSE_UNKNOWN` | 再利用条件が「意味のあるknown license」ではない | license |
| `OSM_LINT_REFERENCE_METADATA` | 引用されたReferenceにcitationもpublisher+日付もない | evidence |
| `OSM_LINT_SOURCE_VISIBILITY` | Resourceが引用するevidenceがprivate provenanceだけ | evidence |
| `OSM_LINT_ASSESSMENT_COGNITIVE_LEVEL` | Assessmentが `cognitive_level` を持たない | assessment |
| `OSM_LINT_ASSESSMENT_SHALLOW` | 2択かつstimulusが60文字未満 | assessment |
| `OSM_LINT_REUSED_DISTRACTOR` | 同じoption textが複数のAssessmentで使われている | assessment |
| `OSM_LINT_OPTIONAL` | optional capabilityは保持されるが実装されない | capability |

## Licenseはfield presenceではなくmeaningを見る

`"license": "再利用条件は未設定。"` のようなplaceholderは以前の「fieldが存在するか」だけの規則を通っていた。現在は `license_status: "known"` かつlicense本文が実質的であることを要求する（`未設定`、`未定`、`unknown`、`TBD`、`N/A` は不可）。明示されていない再利用条件は `{"license_status":"unknown"}` と書く。これはwarningであり、未解決の権利問題を抱えたまま公開する判断を妨げない。リポジトリ自身のfixtureはこの状態を明示している。

## URL検証

公開URLにqueryがあるだけで拒否しない。`?id=123`、`?lang=ja`、`?article=foo`、`?q=fluid+balance` は通常の公開queryであり、拒否すると著者が実際に有用なlocatorを隠すほうへ誘導される。

拒否するのは、埋め込みuserinfo（`user:password@host`）と、credential／署名とみなされるquery parameter名である（`token`、`access_token`、`refresh_token`、`id_token`、`api_key`、`apikey`、`key`、`secret`、`client_secret`、`password`、`auth`、`signature`、`sig`、`sas`、`x-amz-*`、および `_token`／`-token`／`_secret`／`_key` で終わるもの。percent-encoded表記も含む）。

これはheuristicであり、完全なsecret検出を保証しない。`?s=...` のようなopaqueなparameterが秘密でも検出できない。documented limitationとして扱い、「credentialを検出できる」と主張しない。検査は配布前のソースmanifestに対して行うため、`locator_visibility: hidden`や`visibility: private`のlocatorにも適用する。非公開URLはmanifestに書かず、`.osmium/`へ置く。

## Content language / evidence findings

`manifest.language` is the default BCP 47 content language; optional `Resource.language` overrides it. `validate` rejects malformed tags and dangling/duplicate `evidence_reference_ids`. Package Reference records are optional for compatibility. Public records need a usable locator; distributable locators cannot be OS absolute paths, and recognizable credential-like URL parameters are rejected at every visibility. Legacy private records are stripped with their Evidence relations during build. Distribution verification rejects private records and hidden-locator records that kept their locator.

`OSM_LINT_RESOURCE_SHORT` is a heuristic warning for Markdown bodies below 240 characters; it is not an error or a quality score. Review content in context: a concise reference card can be intentional, while a core lesson may need explanation, examples and distinctions. Reference registry and visibility rules are hard validation/privacy constraints; length, coverage, license, evidence completeness and assessment depth are review heuristics. Lint cannot establish factual accuracy, medical safety, copyright permission, or expert review.

## Evidenceの2つのレベル

```text
Resource-level traceability  = 実装済み
Claim-level verifiability    = 未実装
```

lintもvalidateも、ある文があるReferenceに支えられていることを確認しない。ResourceとReferenceの対応を確認するだけである。後者が実装済みであるかのように書かないこと。

## Current capability after this increment

| Capability | Status | Current boundary |
|---|---|---|
| Package init / metadata, Concept, Curriculum, Resource, Assessment edits | Partially supported | `init` creates a starter; Reference registration has three CLI verbs; all other edits are deterministic JSON/YAML/Markdown file operations. |
| Content language | Supported | Package default plus optional Resource BCP 47 override; Assessment/Concept inherit Package language. |
| Reference registration / Evidence | Partially supported | `osmium reference add` / `attach` / `list`; canonical `references` + `evidence_reference_ids`, legacy `sources` + `source_ids` still read. |
| Source visibility | Supported | `record_visibility` / `locator_visibility` with the legacy enum as a compatibility view; enforced at build, distribution-read and runtime-DTO boundaries. |
| Authoring workspace | Supported | `.osmium/` is skipped by the loader and gitignored, so authoring input cannot reach a build or a commit. |
| Assessment depth | Partially supported | Objective mapping is required and validated; `cognitive_level` is optional and free-form; definition/result separation is structurally enforced. |
| Validation / lint / build / inspect | Supported | JSON CLI; semantic Reference refs, portability/privacy checks, evidence- and assessment-aware lint. |
| AI-agent-friendly structured operations | Partially supported | Structured deterministic JSON CLI exists; no full entity CRUD. |
| MCP | Unsupported | Documented adapter boundary only; no server. |

## 将来のschema検証課題

Objectiveと複数Conceptの関係、prerequisite relation type、shared assessment stimulus、Resource部分参照、複数Curriculumでの再利用は実教材で検証してからschema改訂を判断する。現行の `Objective.concept` は単数のまま。`locators` のリスト化、claim-level citation（Citation／EvidenceAnchor／ReferenceMark）、Assessment IR、`cognitive_level` のtaxonomy固定はすべて未実装であり、必要になった時点で判断する。manifestとentityの名前空間付き `extensions` は保持できるが、provenanceの安定仕様はまだない。将来human authored、AI assisted、human reviewed、source verifiedを区別する可能性がある。現行の`validate`はこれらの状態を証明しない。
