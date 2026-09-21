# Roadmap

最小v1の完了条件は[実装計画](IMPLEMENTATION_PLAN.md)で管理する。この文書の項目は未実装の拡張目標であり、現在の機能一覧ではない。

| 順序 | 拡張 | 最初から保持する境界 |
|---|---|---|
| 最小v1後 | math、画像、音声、動画、外部URL、YouTube | TeX/素材を正本、ResourceProvider、明示network操作、alt/captions/license |
| 同上 | multiple select/numeric/text/fill blank/ordering/matching、共通stimulus | Response/Evaluation/Feedbackの分離、versioned capability、決定的採点の適合試験 |
| 同上 | Theme tokens、scoped CSS、安全なHTML subset | presentation enhancement、安全性・accessibility、無効時の本文fallback |
| 続いて | Cloudflare Hub、publishing、search、download、fork/versioning | provider-independent Registry HTTP、Workers/D1/R2/Queues、immutable release、upload→validate→index→publish |
| 続いて | 最小MCP、agent authoring | Core APIのtyped adapter、read-only既定、context制限、patch検証、権限分離 |
| 需要に応じて | SRS/FSRS、mastery、adaptive/placement/recommendation | append-only events、交換可能なprojectionとCurriculum engine |
| 需要に応じて | local LLM/OpenAI-compatible API、Tutor、AI grading | 外部client/evaluator、評価のmethod/model/versionを保存、Core必須依存にしない |
| 需要に応じて | Claim/Evidence、quality gate、provenance | optional module、構造充足と教育品質を区別 |
| 需要に応じて | sync、annotation、collaboration | event UUID/device ID、mutable objectsとcacheの分離、selector、独立サービス |
| 需要に応じて | code/Python/Jupyter/HTML/JS/WASM、hotspot/interactive/3D | 明示sandbox capabilityとfallback。Coreに任意実行を入れない |
| 需要に応じて | Web/Mobile、QTI/CASE/xAPI/LTI/EPUB/SCORM | renderer/store分離、標準とのadapter、exportによる脱出経路 |
| 配布拡大時 | signing/TUF/Sigstore、self-host/federation、moderation/trust | canonical manifest/hash、Registry protocol、配布許可と教育品質を分ける |

Golden Packageは最小schemaと同時に1つ作り、数学・語学・プログラミングへ広げる。画像付き医学連問はmediaとassessment groupの仕様検証に使う。現行機能で表現できない例を無理に平坦化せず、拡張の必要性を記録する。

HubはRuntimeの前提条件にしない。Hubの停止・provider変更時も手元のdirectory/ZIPとLearning Eventが残ることをすべての拡張の条件とする。
