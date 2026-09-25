# Osmium Architecture

状態: 最小v1の当初設計と現行実装の境界を記録する。Phase表記の一部は履歴であり、各操作の現在状態は本文およびREADMEを参照。公開API保証ではない。

現在の `PackageDocuments` は未検証のJSON文書集合、`PackageModel` は構造・意味検証を通過した読取り専用モデル。元のextensionsも保持する。モデル生成はファイルの存在や実際のsymlink安全性を証明しないため、filesystem adapterの検証完了前にinstall可能と扱わない。`prerequisite_order` は循環検証用の前提順序であり、Curriculumの順序や習熟推定を置換しない。

Phase 2aで `osmium-package::load_source` を追加。Source inventory、portable path/containment/link/size検査、JSON/YAML parse、schema/意味検証、Markdown本文読込みを順に行う。Coreにはfilesystem依存を追加していない。診断文書名を実ファイル名に対応づける。返されるLoadedSourceはbuild用のsnapshotである。installerは後のPhaseで実装済みだが、悪意ある同時書換えへの完全な耐性は保証しない。

## 責務と依存方向

Phase 3aの `osmium-package::distribution` はSource snapshotを正規化し、payloadのSHA-256一覧を持つmanifestを生成する。directoryとZIPの読込みは共通のschema/hash/inventory/意味検証へ収束する。ZIPは上限付きメモリで解析し、archiveが指定したpathへ直接展開しない。buildは保存先と同じ親の一時領域へ出力し、再検証後に公開する。未知の拡張値を実行しない。ファイル同時差替えの完全防御や電源断時のdirectory durabilityは保証しない。

```text
CLI ───────────┐
Desktop IPC ───┼── Application operations ── Core
future MCP ────┘             │                │
                            ├── Package I/O  └── pure domain model
                            └── SQLite Store

future Hub ── Registry HTTP adapter ── Package I/O
```

`osmium-core`は概念モデル、参照検証、graph、assessment、Markdownからcontent IRへの変換を持つ。filesystem、SQLite、Tauri、HTTP、Cloudflareへの依存を持たない。Package I/Oはサイズ制限の下で読み込んだファイルと診断元情報をCoreへ渡す。Storeはeventsを永続化する。CLI/Desktopの採点や進捗を別実装しない。

Application operationsはまずモジュール境界とし、必要性が出るまで汎用plugin基盤や新しいcrateを増やさない。WASM対応は将来の適合試験事項であり、現段階で対応済みとはしない。

Phase 4aの`osmium-core::evaluation::evaluate`は検証済みPackageModel、Assessment ID、JSON回答から純粋なEvaluationを返す。single_select/booleanをexact評価し、候補外・型違いを入力エラーにする。結果には問題snapshot、revision、Objective/Concept IDs、response、score、correct、feedback、評価器`org.osmium.exact.v1` version `1`を含む。時刻・UUID・DB書込みはこの関数へ持ち込まず、次のStore/application層で扱う。

## Entityと参照

Phase 2bでは `osmium-core::query`（inspect/query/context）と `osmium-core::lint` を追加し、`osmium-cli` を引数・出力・exit statusのadapterとして実装。Source作成はPackage層の `init_source` が担い、全pathの事前検査、リンク親拒否、create_newによる上書き防止、生成後validationを行う。未知extensionはquery/contextでuntrusted dataとして扱い、lintは構造充足を教育品質と同一視しない。

- Package: package ID、作品version、schema version、capabilities、内容一覧、Reference registry。
- Concept: 学ぶ対象。安定IDとtitle、requiresを持つ。個人の習熟度を持たない。
- LearningObjective: 何ができるか。Conceptを参照する。
- Curriculum: Objectiveを選択し推奨順序を持つ。Conceptのrequiresを変更しない。
- Reference: learner/distributionが到達できる知識資源。`kind`（解決方法）と`type`（資料種別）を分ける。
- Resource: `teaches`でObjectiveを参照し、`evidence_reference_ids`でEvidenceを示し、本文・素材・権利情報を記述。
- Assessment: `measures`でObjectiveを参照し、Stimulus/Response/Evaluation/Feedbackを分離。`cognitive_level`は任意・非規範。
- LearningEvent: 実際に提示された問題、回答、評価、時刻を記録。definitionへは書き戻さない。

参照はpackage ID + entity kind + entity IDで修飾する。ファイル名変更ではIDを変えない。Packageのversionが変わっても古いeventを更新しない。型の異なるID参照を拒否する。

## Core API境界（意味契約、signatureはPhase 1で確定）

| 操作 | 入出力と責務 |
|---|---|
| parse_source | 制限済みファイル集合 → model + source map + diagnostics |
| validate | model + supported capabilities → schema/refs/graph/assessment診断 |
| inspect/query/context | validated model + typed filter + 出力量制限 → semantic DTO |
| compile_content | Markdown → renderer非依存IR、raw HTMLは不活性なtextとして扱う |
| evaluate | immutable item snapshot + validated response → score/feedback/evaluator version |
| project_progress | events + model version → Objective単位の再構築可能な進捗 |
| build（I/O層） | validated source → normalized files + integrity metadata |
| install（I/O層） | local distribution + trusted library root → immutable install record |
| submit_attempt（application） | item identity + response + request ID → atomic event + projection |
| export_state（store） | event log → versioned JSONL（バックアップ時は整合したsnapshot） |

## Content rendering boundary

```text
Learning Package Markdown
  → osmium-package reads and validates the referenced resource
  → osmium-core::content::compile_markdown
  → renderer-neutral Content { blocks: Vec<Block> }
  → Desktop IPC ContentView / ResourceView DTO
  → React renderer builds nodes from Block / Span
```

`osmium-core::content` is the only Markdown parser and owns the semantic conversion. `Block` models headings, paragraphs, lists and list items, quotes, code, math, tables, inert HTML text and rules; `Span` models text, inline code, emphasis, strong, strikethrough, math and classified links. Images reduce to their alt text because the current package schema has no typed asset reference. The DTO sent to Desktop carries the compiled `Content`, not a second Markdown string. **RuntimeごとにMarkdown parserを持たせない。** Web/mobile or a future preview can consume the same Core IR rather than independently reinterpret package text.

The IR contains no React, DOM, HTML, KaTeX output, syntax-highlighter output or renderer class names. Core classifies link targets: only `http`, `https`, and `mailto` schemes remain absolute links; package-relative links must be plain contained paths; absolute filesystem paths, traversal, schemes such as `javascript:`, `data:`, and `file:`, and malformed destinations lose their link target while retaining readable label text. Raw HTML remains literal inert text. Desktop renders these nodes with React; KaTeX and lowlight decorate only typed TeX/code values, and package code is never executed. KaTeX's generated output is converted into React nodes with bounded rendering options; package-authored HTML is never sent to an HTML parser or sink.

Packageが指定するpathを直接OS APIに渡さない。GUIは受理済みpackage/entity IDを指定し、任意pathをIPC経由で読み出せない。問題のcorrect answerはローカル教材に含むため、試験の不正防止基盤とは位置づけない。

## ローカル保存と回復

OS標準のユーザーデータ領域を既定とし、CLI/テストでは `OSMIUM_HOME` で切替可能にする。リポジトリ配下に利用者の実データを作らない。

```text
<data-root>/
  library/<package-digest>/    # 検証済みのfiles、内容はimmutable
  staging/                    # 隔離された導入途中の内容
  state.sqlite                # learning events + installed index + projections
```

filesystemとDBは一つのtransactionにできない。stagingで検証を完了し、同じfilesystem内でrenameしてからDB登録する。起動時に孤立した導入物を検出・再検証して復旧し、導入失敗で既存教材を消さない。Source編集中の変更を読む危険には、stagingへコピーした内容を再検証・hashすることで対処する。

Phase 3bではPackage層の`library::Library`がfilesのinstall/list/readを実装する。Library handleは`.library.lock`のOS排他lockを保有し、並行操作は待ち続けずI/O診断を返す。アプリは操作ごとにopen/dropする。全fileをstagingへcreate_new・syncし、再検証後にdigest directoryへrenameする。同一ID/versionで異なるdigestは拒否し、同一digestは冪等。listは全導入物を再検証し、DB登録前に中断された完全なdirectoryも検出できる。SQLite metadata登録はPhase 4で追加する。中断で残ったstagingはlibraryに表示せず、自動削除もしない。電源断durabilityと悪意ある別processのfilesystem差替えに対する完全保証はしない。

eventsはUUIDのevent ID、device ID、schema versionを持ち、追記のみ。request IDを使い送信retryを二重学習と数えない。別の意図的な回答には新しいrequest IDを使う。SQLite transactionでeventとprojectionを更新し、失敗時に半端な記録を残さない。

Phase 4bの`osmium-store::runtime::Runtime`をCLI/Desktop共通のapplication境界とする。Library lockを操作session中だけ持ち、Storeが導入metadataを再同期する。installは一度検証したメモリsnapshotを使い、DBに残る同ID/versionの旧digestとも照合する。StoreはSQLiteのapplication_idとuser_versionを確認して、空DBへの001 migrationだけを行う。未知版・別用途・破損DBは置換しない。将来版への自動migrationは未実装。

`events`が正本でUPDATE/DELETE拒否triggerを持つ。`assessment_attempts`は同じeventsを読むview、`progress`はdigest/Objective単位のcache。request ID再送はpackage/item hash・response・duration・hintsを照合して元eventを返し、異なる要求への使い回しを拒否する。イベントとprojectionの書込みを1 transactionへまとめる。`rebuild-progress`はsequence順に全eventsを再生し、再生失敗時は元cacheへrollbackする。観測値はattempts/correct/accuracy/last score/time。version間で自動統合しない。

`export-state`は1つの読込みtransactionからJSONLを書き、`backup-state`はSQLite backup APIで整合したDBを作る。両者とも一時fileから非上書きで公開する。historyは最大64件・8 MiBのページ。新規DB作成には移行前データがないためbackupは不要とし、将来の非空schema移行はbackup契約を実装するまで受理しない。個人データへの暗黙のネットワーク通信はない。

projectionは試行数、正答数、最終回答時刻等から開始する。客観的な「習得保証」と表示しない。schema migrationは番号付きとし、migration前backup、rollback時の挙動、将来のschemaを開いた際の書込み拒否を試験する。DB破損時に空DBへ黙って置換しない。

## Renderingと権限

CommonMark本文とmetadataを保持し、Coreのcontent IRをrenderer-neutralな表示入力とする。現在のIRにはheading/list/quote/table/math/codeなどの構造、inline math、classified link destinationが含まれる。DesktopはこのIRをReact node treeへ変換し、raw HTMLを解釈せず、ReactのHTML sinkを使わない。KaTeXとlowlightは型付きIRのTeX/code値から装飾nodeを生成し、その生成結果だけをReact nodeへ変換する。Package codeは表示・copyのみで実行しない。外部URLは表示だけを基本とし、自動fetchしない。

標準themeに加えてKaTeXとhighlight.js系の表示themeを使う。semantic headings、フォームlabels、keyboard操作、focus、文字拡大、contrast、reduced motionをRuntimeの責任とする。Math sourceはTeXを正本とし、mediaは現行Packageにtyped asset referenceがなく、将来ResourceProviderとの設計が必要。scoped CSSは独立した検証を通すpresentation enhancementにする。

## 将来の接続点

Evaluator、ResourceProvider、Renderer、将来のMasteryEngine、CurriculumEngine、Registry、Importer/Exporterは責務として分ける。保存形式に必要なextensions/capabilitiesだけ最初から作り、動的コードloadは実装しない。AIやMCPは将来の外部クライアント、同期はevent identityを利用する別サービスの候補とする。Hubは未実装で運用先も未決定。Hubへ学習履歴を送る経路を最小v1に作らない。

## Authoring and agent operations

The stable direction is a shared capability surface: CLI and a future MCP adapter call the same Core and Package operations. Core validates package meaning and source references without I/O; Package handles bounded source loading, build sanitization, distribution verification and the three Reference authoring edits. CLI remains an argument/JSON envelope adapter. MCP should expose equivalent structured inputs/outputs and must not introduce alternate validation semantics. There is not yet a full application-operation facade or MCP server; extract one when an MCP adapter is implemented rather than prebuilding a generic plugin layer.

Skills live above this capability surface and specify workflow/policy, not new operations. Search capability, shell availability, or MCP availability are agent runtime capabilities, not package semantics. Source acquisition method is not required in the portable Reference record. Detailed Reference fields, Evidence and visibility behavior are in [`SOURCES_AND_PROVENANCE.md`](SOURCES_AND_PROVENANCE.md).

## Reference, Evidence and authoring boundaries

Three kinds of information are kept apart on purpose, and the module layout enforces it:

```text
manifest.references[]            Reference   — distributable knowledge resources
evidence_reference_ids[]         Evidence    — Resource/Assessment → Reference links
<package>/.osmium/               Authoring Provenance — inputs and history, never distributed
```

`osmium-core::reference` owns the vocabulary — registry field names, the Evidence field, the two visibility axes, the reuse-policy predicate and the locator query heuristic — so validation, lint, distribution and the Runtime projection cannot drift apart. `crates/osmium-core/tests/reference.rs` pins the compatibility mapping in one place.

The boundary is applied at three independent points, because a single point is a single bug away from a leak:

| Point | Enforces |
| --- | --- |
| `osmium-package::load_source` | `.osmium/` is skipped during inventory, so authoring input never becomes package payload |
| `osmium-package::distribution` (`compile_source`, `verify_files`) | private records, hidden locators and non-public asset bytes are removed, then a built archive that still carries one is rejected |
| `osmium-store::runtime::Runtime::resource` | only record-public, resource-linked, locator-public fields reach the learner DTO |

Assessment responsibility boundaries — item, interaction, response model, scoring, feedback, result and presentation — are documented in [`ASSESSMENT_MODEL.md`](ASSESSMENT_MODEL.md), and the QTI mapping plus its lossy points in [`INTEROPERABILITY.md`](INTEROPERABILITY.md).
