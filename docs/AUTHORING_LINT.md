# Package authoring と lint

Status: current implementation (2026-09-23). 歴史的な Phase 記録は `IMPLEMENTATION_PLAN.md` と `PHASE2B_HANDOFF.md` に残す。

`osmium validate <path> --json` はschema、参照、循環、本文と安全なpathを検証する。失敗は `ok: false`、exit 1（非互換は4）。`osmium lint <path> --json` はvalidateを通ったPackageの静的解析であり、warningがあっても `ok: true`、exit 0。stdoutは単一のversion 0.1 JSON envelope、stderrは人間向け診断行。`diagnostics` の既存fieldは維持し、lintには省略可能な `entity_type` と `entity_id` を追加した。診断はfile、entity ID、code、messageで安定ソートする。

| Code | 条件 |
| --- | --- |
| `OSM_LINT_OBJECTIVE_NO_RESOURCE` | Objectiveを教えるResourceがない |
| `OSM_LINT_OBJECTIVE_NO_ASSESSMENT` | Objectiveを測るAssessmentがない |
| `OSM_LINT_ORPHAN_CONCEPT` | ConceptがObjectiveにも他Conceptのprerequisiteにも参照されない |
| `OSM_LINT_OBJECTIVE_NOT_IN_CURRICULUM` | Curriculumが存在するがObjectiveをどれも列挙しない。Curriculumなしなら抑制 |
| `OSM_LINT_PREREQUISITE_ORDER` | 同じCurriculum内で、明示されたConcept prerequisiteの最初のObjectiveが従属Conceptの最初のObjectiveより後にある |
| `OSM_LINT_METADATA` | Resourceのcreator、license、attributionが欠ける |
| `OSM_LINT_OPTIONAL` | optional capabilityは保持されるが実装されない |

Curriculum間の順序、Objectiveのないprerequisite Concept、複数Objectiveをまたぐ厳密な教授順序は推測しない。lintは教材の正確性、品質、専門家レビューを保証しない。現行manifestにはPackage単位のdescription、creator、license fieldがない。Desktopはtitle、version、language、件数を表示し、Resource単位のcreator/licenseは技術情報に表示する。

## 将来のschema検証課題

Objectiveと複数Conceptの関係、prerequisite relation type、shared assessment stimulus、Resource部分参照、複数Curriculumでの再利用は実教材で検証してからschema改訂を判断する。現行の `Objective.concept` は単数のまま。manifestとentityの名前空間付き `extensions` は保持できるが、provenanceの安定仕様はまだない。将来human authored、AI assisted、human reviewed、source verifiedを区別する可能性がある。現行の`validate`はこれらの状態を証明しない。

## Content language / source findings

`manifest.language` is the default BCP 47 content language; optional `Resource.language` overrides it. `validate` rejects malformed tags and dangling/duplicate `source_ids`. Package source records are optional for compatibility. Public records need a usable locator; non-private distributable locators cannot be OS absolute paths or contain query parameters. Private source records may contain a dummy/local authoring locator because distribution strips the record and Resource reference. Distribution verification rejects private records and attribution-only locators.

`OSM_LINT_RESOURCE_SHORT` is a heuristic warning for Markdown bodies below 240 characters; it is not an error or a quality score. Review content in context: a concise reference card can be intentional, while a core lesson may need explanation, examples and distinctions. `OSM_LINT_LANGUAGE` flags a missing effective package/resource language. Source registry and visibility rules are hard validation/privacy constraints; length and coverage are review heuristics. Lint cannot establish factual accuracy, medical safety, copyright permission, or expert review.

## Current capability after this increment

| Capability | Status | Current boundary |
|---|---|---|
| Package init / metadata, Concept, Curriculum, Resource, Assessment edits | Partially supported | `init` creates a starter; further edits are deterministic JSON/YAML/Markdown file operations, not CRUD CLI verbs. |
| Content language | Supported | Package default plus optional Resource BCP 47 override; Assessment/Concept inherit Package language. |
| Source registration / provenance | Partially supported | Optional typed manifest registry and Resource/Assessment `source_ids`; registration is file-based. |
| Source visibility | Supported | public / attribution_only / private enforcement at build and distribution-read boundaries. |
| Validation / lint / build / inspect | Supported | JSON CLI; semantic source refs, portability/privacy checks, source-aware distribution, body-length heuristic. |
| AI-agent-friendly structured operations | Partially supported | Structured deterministic JSON CLI exists; no source CRUD or full entity editing commands. |
| MCP | Unsupported | Documented adapter boundary only; no server. |
