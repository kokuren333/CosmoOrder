# Learning Experience / Knowledge Atlas Gap Analysis

監査日: 2026-09-25  
対象: この作業ツリーに存在するOsmium Core / Package / Store / Desktop / CLI / schema / examples

## 1. Executive summary

現状は、**Packageをローカルで検証・配布し、Readerで読み、限定されたAssessmentを採点し、履歴とdigest別の進捗を記録する構造化学習Runtime + Authoring Studio**である。単なるMarkdown viewerやLMSではない。一方、adaptive learning runtime、Knowledge Atlas、横断Personal Atlasはまだ実装されていない。

目指す方向とよく一致する土台はある。Rust CoreがPackage semanticsを所有し、Package層がsource/build/installとtyped authoringを担い、StoreがPackage本文から分離した履歴を記録する。`osmium_core::query`にはページング可能な一覧と上限付き近傍探索が既にあり、Atlas用の最初の読み取り面として再利用できる。

最大の隔たりは「ノードを描けないこと」ではない。今のConcept関係は`requires`のみであり、Atlasが表す知識意味論は狭い。CurriculumはObjectiveを並べるが、Routeのゴール・分岐・remediation・学習状態に応じたsequencingはない。Storeの正答率はdigestとObjectiveごとの観測集計であり、mastery推定ではない。Assessmentはsingle-select / booleanとexact採点に限られ、実行・制作・診断等のActivityモデルもない。

**推奨: 今はschemaを増やさない。** v0.1のConcept + Objective + Curriculum + Resource + AssessmentをPackage-local graphとして扱い、既存Core queryをUIで利用してPackage Atlasの小さなread-only prototypeを作る。Learner UIはLessonを既定とし、Route、Atlasへ必要時にズームアウトする。Personal Atlasは概念対応候補とProvenanceを持つRuntime派生データとして、Package graphと学習履歴を変更せず後段で試す。Embedding誤結合は有害になり得るため候補提示までに留め、same-as確定やmastery移送を自動化しない。

## 2. Current architecture inventory

以下はコードで確認した現状である。監査時点の作業ツリーには多数の未commit変更があるため、これはclean commitの監査ではなく、**現在のworkspace snapshot**の監査である。

| 層 | 実在する実装 | 監査上の意味 |
|---|---|---|
| Rust workspace | Root `Cargo.toml`; `osmium-core`, `osmium-package`, `osmium-store`, `osmium-cli`, `apps/desktop/src-tauri` | semanticsはCore/Package/Storeに分割。Desktopは薄いIPC adapterを目指す。 |
| Schema / package | `spec/v0.1/package.schema.json`; `osmium_core::schema::{DocumentKind,SCHEMA_VERSION}`; `PackageDocuments` / `PackageModel` in `crates/osmium-core/src/validation.rs` | JSON Schema bundleはv0.1。Manifestはpackage ID/version/title/language/entitiesを持つ。現schemaファイルにブランド名付き`$id`はない。 |
| Content IR | `crates/osmium-core/src/content.rs`: `Content`, `Block`, `Span`, `compile_markdown` | renderer-neutral・inertなMarkdown IR。script/HTMLを実行せず、AssessmentやActivityを実行するIRではない。 |
| Concept / Objective | schema defs `concept` / `objective`; Core `validate_package` | Conceptには`requires: [Concept ID]`; ObjectiveにはConcept参照とdescription。`requires` graphは参照・循環検証とtopological orderを行う。 |
| Resource / Assessment | schema defs `resource` / `assessment`; `evaluation::evaluate` | ResourceはMarkdown path/title/`teaches` Objective参照。AssessmentはObjectiveを`measures`し、v0.1 builtin response/evaluatorはsingle-select/booleanとexact。採点は決定的。 |
| Curriculum / relation | schema `curriculum.objectives`; `crates/osmium-core/src/query.rs::build_graph/context` | Curriculumは順序付きObjectiveリスト。Context queryは`requires`, `concept`, `teaches`, `measures`, `orders`をDTOとして返す。任意のsemantic relationはschemaにない。 |
| Authoring | `crates/osmium-package/src/authoring/workspace.rs::{AuthoringWorkspace,WorkspaceEdits,save_workspace}` | typed multi-entity draft。Core validationの前にsourceを変えず、既存JSON objectの未公開fieldを保持する。TauriはこのPackage APIを呼ぶ。 |
| Package IO | `crates/osmium-package/src/{lib.rs,init.rs,distribution.rs,library.rs}` | Source load、scaffold、build、distribution verify、install/uninstallを所有。Distributionは再検証され、payloadはdigest別immutable directoryに格納される。 |
| Store / learning state | `crates/osmium-store/src/{lib.rs,runtime.rs,migrations/001.sql}` | `events`はappend-only、`progress`は`(package_digest, objective_id)`単位のattempt/correct/last score。progressはeventから再構築可能。`accuracy`は記述統計でmastery判定ではない。 |
| Runtime / Reader | `Runtime::{lesson,resource,install,uninstall,read,answer,progress,history}`; `apps/desktop/src/App.tsx`; `components/{Packages,Lesson,Reader,Assessment,ProgressPanel,HistoryPanel}.tsx` | Lessonの目次/本文/Assessment・履歴がある。既定の画面はLibrary/lessonで、Route/Atlas画面は未実装。 |
| Desktop IPC | `apps/desktop/src-tauri/src/commands.rs`, `lib.rs` | commandからCore/Package/Store APIへ委譲。CLI実装の直接呼出しやReact内のschema validationではない。 |
| CLI / lint / validate | `crates/osmium-cli/src/{args,commands}.rs`; Core `validation.rs`, `lint.rs`; Package `distribution.rs` | `validate`, `lint`, `build`, `install`, `inspect`, `query`, `context`等がある。警告はvalidation failureやmasteryと別。 |
| Tests / examples | crateごとのunit/integration tests; `apps/desktop/tests`, `apps/desktop/e2e`; `examples/{arithmetic,arithmetic-expanded,language-pressure-test,mathematics-pressure-test,medicine-pressure-test,programming-pressure-test,...}` | 多Entity / 参照 / Assessment / Distribution / Desktop lifecycleを確認するfixtureがある。domain pressure examplesはArticle + Assessment中心で、interactive task modelの実例ではない。 |

代表的なコード契約:

- `PackageModel::prerequisite_order()`は「前提を依存Conceptより前に並べる」決定的順序であり、Curriculum推薦やmastery予測ではない（`crates/osmium-core/src/validation.rs`）。
- `query::context()`はdepth/node/byte budget付きBFSで、`MAX_CONTEXT_NODES=512`、`MAX_CONTEXT_DEPTH=8`、`MAX_CONTEXT_BYTES=256 KiB`を持つ（`crates/osmium-core/src/query.rs`）。Atlas全描画用ではなく、bounded local neighborhoodに適する。
- Storeのprogress projectionはPackage digest別。v1とv1.1 digestが変わればprogressも分かれ、自動移行しない。eventにsnapshot・digest・assessment情報が残る。
- uninstallはpackage payloadを除き、Store eventは残す。一方、現在のUIから削除済PackageにひもづくHistoryを十分に再表示できるかは既存POST-P0/P1監査でもgapとして記録されている。Package metadataのsnapshot/UI lookupを改善せずPersonal Atlasへ進むのは順序が逆。

## 3. Existing strengths

1. **正しいsemantics境界**: Rust Coreはframework-independent。Package層がfilesystem、source/build、distribution、libraryを扱い、Storeがlearning eventを扱う。Reactにvalidatorを複製しない設計である。
2. **Package-local authoritative model**: 各Package内IDの参照解決と`requires`循環検査はdeterministic。Packageを独立作者の教育上の解釈とみなす目標に合う。
3. **知識依存と教育順の部分的分離**: `Concept.requires`と`Curriculum.objectives`は別fieldで、validation commentもCurriculum orderとprerequisite orderを区別している。ここを崩さない。
4. **bounded Core query**: `inspect/query/context`はAtlasのread modelに再利用できる。最初のAtlasのためにGraph schemaや巨大graph engineを導入する必要はない。
5. **Source / installed runtime境界**: typed Workspaceで編集したsourceはCore validationを経てbuild/installされ、RuntimeはDistributionから読む。
6. **拡張情報の保存**: Workspace saveは既存JSON objectをmergeし、GUIの範囲外のextension等を保持するテストがある。Packageの機械可読性とGUIの段階的開示に向く。
7. **再現可能なassessmentと履歴**: Core exact evaluator + append-only events + digest-scoped progressは、LLM会話履歴より再現・監査しやすい土台。
8. **local-first/offline**: Package runtimeはネットワークやLLMを必須にせず動作する。これは差別化の実装済み部分。

## 4. Gap analysis

| Gap | 現状 | 問題 / 対応判断 |
|---|---|---|
| Knowledge relation semantics | Conceptのedgeは`requires`のみ | `related`, `generalization`, `analogy`などは未表現。`requires`を万能edgeとして流用すると意味を壊す。Package-local Atlas v0は現edgeだけで開始。一般Relation schemaは実ユースケースと学習上の評価が揃ってから。 |
| Knowledge vs pedagogy | Concept.requiresは知識前提候補、CurriculumはObjective列 | 学習者個人の学習前提か、著者推薦の順序か、必要条件かが現場面で曖昧になり得る。v0.1の意味を明記し、Curriculum順序から知識関係を推定しない。 |
| Route / goal | CurriculumのObjective sequenceは存在 | User goal、現在地、来歴、次に進む理由・unlock、複数route、選択的分岐はない。単にCurriculumの現在indexを「adaptive route」と呼んではいけない。 |
| Learning state | attempt count、accuracy、last scoreだけ | 「mastered/weak/unseen」は現在のschema/storeから正当化できない。Assessment機会のないConceptにもscoreを誤投影しない。 |
| Remediation/review | feedbackとResource/Assessmentリンクはある | 誤答から別説明へ誘導、再試行ポリシー、spaced review schedulerはない。まず実教材で一つのremediation loopを設計し、core primitiveよりcurriculum/link表現でできるか検証する。 |
| Activity / modality | Resource(Markdown) + Assessment(single-select/boolean) | read→quiz以外のsolve/debug/simulation/projectは第一級ではない。しかしActivity全種をCore enumにする根拠もない。interactive tasksの要求を集めてから拡張境界を決める。 |
| Evidence | Assessment evaluationがscore/evaluator/snapshotをEventに記録 | 現行の採点結果はあるが、提出物、観察、rubric、口頭回答等の一般Evidenceモデルはない。新たな汎用Evidence graphは急がない。 |
| Atlas UX | bounded context DTOはあるがAtlas UIなし | Atlasが装飾グラフなら学習成果に寄与しない。goal、coverage、prerequisite、why-next、resource accessに接続して初めて価値がある。 |
| Cross-package mapping | Package-local ID/title以外のsemantic descriptorやmapping storeなし | 同名の曖昧さ、翻訳、分野差を確実に解けない。embedding類似度をidentityと見なさない。 |
| History after uninstall | Eventは保持、利用不可Packageの状態もStoreで追跡 | 表示タイトルやPackageへの参照が切れる場合がある。History presentation snapshot/identity resolutionをAtlasより先に埋める価値が高い。 |
| Mobile | Tauri mobile entry annotationはあるが、generated Android/Apple project/configはworkspaceにない | Rust CoreやSQLiteが移植可能でも、dialog/plugin、file access、storage permission、background inferenceは検証されていない。Mobile readyとは言えない。 |
| Authoring disclosure | typed multi-entity workspace/APIとLevel 3 CLIはある | GUIでのLevel 1 (教師の短い作成task) / Level 2 (learning design)のタスク設計と検証が不十分。画面に複数entityを並べられることはprogressive disclosureの証明ではない。 |

### 批判的評価

- **Concept中心化のリスク**: すべての学習成果がConceptへ還元されるとは限らない。技能・手技・作品・判断はObjective/Artifact/Evidenceを中心にした方が自然な場合がある。まず今の「ObjectiveはConceptに属する」制約が実例で十分か試し、万能のontologyだと主張しない。
- **Atlasの成果仮説**: Atlasが成績を改善する証拠は現状ない。routeの理由や穴を見つける補助として検証し、常時表示・全体graphを目標にしない。
- **Embeddingの誤結合**: 多義語、翻訳、異なる粒度を誤ってsame-asにすると前提/進捗の誤表示へ伝播する。近傍は候補であり、relation type付き・確信度付き・Provenance付きにする。必要に応じ人間確定を要する。
- **Graph作者負担**: 著者にConcept IDやedgeを必須入力させれば素材作成率を下げる。Level 1はtitle/resource/簡単なquestion/基本順序だけで完了可能にする。Graphは任意のLevel 2へ置く。
- **Personal Atlasの費用対効果**: ユーザーが複数の学習Packageを同時利用し、重複や転移候補を探す具体的頻度が確かめられるまで、embedding runtimeやHNSWを作るのは過剰。
- **Atlasの失敗モード**: inferred edgeがexplicit author claimに見えたり、mappingがmasteryを移すことを許さない。source Packageの書換え禁止は守る。

## 5. Proposed target model (minimal evolution)

現schemaで表せる部分を維持し、新primitive追加を保留する。

| 責務 | 現状 / 今後の最小解釈 | 分類 |
|---|---|---|
| Knowledge | Package内Conceptと`requires`（有向prerequisite） | 既存primitiveで利用 |
| Learning objective | Conceptに属するObjective | 既存primitiveで利用。実例でConcept必須の限界を検証 |
| Instruction | Objectiveを教えるMarkdown Resource | 既存primitiveで利用 |
| Practice / assessment | Objectiveを測りdeterministicに採点するAssessment | 既存primitiveで利用可能な範囲のみ |
| Curriculum | 教育者のObjective順序 | 既存primitive。Concept graphと同じedge集合に統合しない |
| Evidence | 現Assessment snapshot/response/scoreをEventに保存 | 既存Attempt eventで基本用途。rubric/portfolioの必要時まで汎用schema追加は不要 |
| Activity | `read`, `watch`, `solve`, `debug`等の統一step primitive | **未追加推奨**。ResourceとAssessmentが一つのRouteで組み合わせられない具体例が出たら、Activity instanceが既存entityを参照する最小形で評価。Coreに各domain interactionをhard-codeしない |
| Learning State | digest/Objective別の観測記録 | 現状で使う。masteryやtransferは別のmodel/algorithmとして未決定であるとUIにも明示 |
| Relation | `requires`, `concept`, `teaches`, `measures`, `orders`を区別 | 現行query DTOのtyped relationをPackage-local read modelへ再利用。schema上の新generic edgeは未追加 |

今後追加する可能性のあるsemantic Relationは少数の有向・型付き概念に絞る。候補は`related_to`、`broader_than`、`narrower_than`、`derived_from`等。`requires`はpedagogical prerequisiteとして明示し、relatedと同一視しない。実装時には逆関係・循環許容性・Provenance・versioningと既存`requires`の互換性を定義してからschemaを変える。

## 6. Package-local Atlas

### Data / query

- Package boundaryを基本Atlas境界とし、node identityは`(package_id, package_version/digest, entity_kind, local_id)`。ID単独をPackage横断で解釈しない。
- 現行の`query::inspect/query/context`をまず使う。`context`はRelationとbounded neighborhoodを既に返す。必要な場合も、全体表示向け新schemaではなく、Coreのページング/小さな概念概要を追加する。
- v0 AtlasはConcept + `requires`を明示表示する。Objective/Resource/Assessment/Curriculumはlayer toggle/detailsで見る。Knowledge layerとCurriculum layerを凡例・UIで分ける。
- graphは初期表示の主画面にしない。選択Conceptの近傍、module grouping、検索、semantic zoom、最大表示数、検索結果を中心にした局所表示にする。100k nodeをDOM/Canvasへ投入しない。
- Package source graphと作者の主張がauthoritative。Runtimeは閲覧専用でsource Packageを書き換えない。

### UI / Runtime

Atlas node選択からLesson/Resourceへ移動し、ConceptのObjective、前提、順序、紐づくResource/Assessmentを確認できるread-only panelを作る。新しいGraph APIを作る前に、既存`context`が必要な情報を十分に返すか確認する。

## 7. Personal Runtime Atlas (derived data)

これは将来案であり、今回schema/DBを変更しない。

### Identity / SQLite tables (proposal)

Package内Conceptのkeyは`(package_digest, concept_id)`とする。package ID/versionは表示・来歴に使うが、更新で内容が異なるConceptを混同しない。Source Packageは一切書き換えず、Runtime DBにversion付きprojectionを追加する。

最小候補:

```text
concept_descriptor(
  package_digest, concept_id, descriptor_hash,
  normalized_title, aliases_json, short_definition,
  module_context, external_ids_json,
  PRIMARY KEY(package_digest, concept_id)
)

semantic_vector(
  profile_id, descriptor_hash, dimensions, vector_blob,
  created_at, PRIMARY KEY(profile_id, descriptor_hash)
)

concept_mapping(
  left_digest, left_concept_id, right_digest, right_concept_id,
  relation, score, method, profile_id, provenance,
  status, evidence_json, created_at,
  PRIMARY KEY(left_digest,left_concept_id,right_digest,right_concept_id,relation,method,profile_id)
)
```

Relation candidates: `same_as`, `overlaps`, `broader_than`, `narrower_than`, `related`. Mapping rows store `method` (`exact_external_id`, `normalized_label`, `embedding`, `graph_rerank`, `human_confirmed`等), score, descriptor/package digests, profile/version, time and `provenance` (`explicit` / `inferred`). Candidates and human-confirmed links remain distinguishable. Human confirmation cannot silently rewrite author Package.

### Candidate generation / embedding

順序はexact external ID → normalized title/alias → optional embedding candidate retrieval → graph-neighborhood consistency reranking。DescriptorはConcept title/aliases/short definition/module context/keywordsに限る。Article body、Resource、Assessment、Question、Historyはembeddingしない。Atlas描画時にモデルをload/実行せず、install/updateで差分計算しSQLiteへcacheする。

Embedding profileはRuntime管理のversioned profileとし、model/revision、tokenizer、入力構成、dimension、normalization、量子化/数値形式を固定する。`osmium-semantic-v1`を一度永続化したらprofile IDはdata contractとなる。CosmoOrder rename後もalias/migrationか新profile versionを明示し、既存vectorを名前だけ変えて同じものと偽らない。Package作者が自分のvector spaceをcanonical dataとして持ち込む設計にはしない。

### Scale / safety

100k Concepts / 数百Packageを初期の性能目標候補とするが、同時に1M edge・巨大ANN基盤を先行実装しない。SQLiteのindexed exact/normalized candidateと小さいbatchで計測し、benchmarkが必要性を示してからincremental indexまたはANN/HNSWを検討する。384-dimensional quantized CPU ONNXはmobileの候補であって決定ではない。モデルサイズ、license、memory、thermal、battery、cold-start、storage cost、multilingual qualityを実機評価してから選ぶ。

Package mappingとmastery transferは別契約。初期実装のtransferは無し。仮に将来行う場合でも、target Packageのdiagnostic evidenceが必要であり、同一concept mappingだけでaccuracyやmasteryをcopyしない。

## 8. Learning UX: Lesson / Route / Atlas

1. **Lesson (default)**: 今のResourceと必要なAssessmentを表示する。学習者はID/edge/schemaを見ない。
2. **Route (goal view)**: goalに向かい「何から来たか / 今 / 次 / なぜ」を見せる。v0では作者のCurriculumと`requires`を根拠として表示し、Runtimeが推定した最適経路と称さない。選択したgoalに対するcoverageやunlockは実際のmodelに対応づける。
3. **Atlas (orientation view)**: user actionでzoom-outし、Package内module/Concept領域を俯瞰。局所近傍をdefaultとし、state overlayは現状のattempt履歴と、authoritative package relationと、inferred cross-package mappingを別レイヤにする。

現状はLessonの目次/本文とAssessment/Progress/History panelまで。RouteとAtlasは目標画面である。現在のaccuracyのみで`mastered`/`weak prerequisite`を塗るのは誤解を生む。現段階では`attempted`, `recently correct/incorrect`, `not yet assessed`等の観測ラベルが妥当。学習状態がないことを`unseen`と、測定未設定を`weak`と取り違えない。

## 9. Authoring UX: Level 1 / 2 / 3

| Level | 表示 | 現状と必要なこと |
|---|---|---|
| 1 一般作者/教師 | title、言語、Resource本文、簡単なAssessment、基本順序 | New sourceとtyped Editorはあるが、教師が最短で一つのlessonを完成するtask設計/使い勝手を確認する必要。Concept/IDを要求せず、Package APIがID生成とCore validationを担当する。 |
| 2 Learning designer | Concept構造、前提、Objective、Curriculum、evidence、remediation、難易度 | typed Draft APIにConcept/Objective/Resource/Curriculum/Assessmentと参照はある。Graph可視化、explicit knowledge vs sequenceのUI、分岐/remediation/evidence policyはない。まず既存APIの見せ方を整える。 |
| 3 Agent/developer | Package JSON/schema、extensions、CLI、validate/lint/build/test/CI | Package v0.1、CLI、Core validator、builderが既に機械向けの明示面。GUIに内部JSONを再実装しない。 |

Agent-friendlyはmachine-readableなPackageとdeterministicなCLI/APIで実現し、Level 1 UIへschemaを転嫁しない。

## 10. Extensibility: builtins vs extensions

現在のbuiltinはMarkdown Resource、single-select/boolean Response、exact Evaluation、limited renderer-neutral Markdown IRで小さく保たれている。これを全ての学習modalitiesに拡張する型unionへ肥大化させない。

次のdomain interactionが現実化した時、まずActivity-like instanceが既存Resource/Assessmentを参照し、Runtime capabilitiesの宣言とsafe fallbackが必要かを検証する。Code editor、GeoGebra、anatomy 3D、simulation等はextension/runtime capability候補で、Coreが実行コードやdomain dataの意味を理解する必要はない。任意plugin system、remote executable、package scriptはセキュリティ/互換性境界が整うまで作らない。

## 11. Mobile constraints

- Core Rust・SQLite・bounded queryという形はAndroid/iOSへ持ち込みやすい。ただし現状workspaceにTauri generated Android/iOS projects、application IDs、signing setupはなく、mobile build実績を確認できない。
- Desktop uses native dialog plugin and desktop E2E/CDP. Mobile needs picker/storage permission lifecycle, app sandbox paths, backup/export, pause/resume, low-memory behavior, keyboard/touch layoutsを別に検証する。
- `OSMIUM_HOME`/`OSMIUM_DEBUG_PORT`、Windows `%LOCALAPPDATA%`、Linux XDG、filesystem lock semantics、SQLite journalはmobileのhome/sandbox運用へそのまま決め打ちしない。Core APIはplatform-free、data rootはplatform adapterから渡す。
- Offline inference modelはUI起動時に必須にしない。必要時load、CPU/quantized、model downloadを強制せず、indexが欠損/古くてもpackage閲覧を続けられるderived cacheとする。
- Model size/license/locale qualityと100k embedding cacheは、mobileのdisk/RAM/batteryに重い。mobile MVPにPersonal Atlas embeddingを含めず、metadata/exact/title mappingから評価する。

## 12. Migration strategy

Package v0.1は現状維持。Atlasは現在のConcept/Objective/Curriculum/Resource/Assessmentから派生する。追加fieldが必要になった場合は、stable v0.1を暗黙に意味拡張せず、schema versioning、旧Package読み込み、unknown extension preservationを先に定める。Runtime派生mapping/vectorはsource Packageと別DB tables・descriptor hash/profile versionで管理し、recomputeできるcacheとする。

Existing progress remains digest-scoped. Concept mapping alone never transfers progress. Existing event snapshot and source graph remain immutable. User-facing title or brand rename stays independent of Package protocol and database evolution.

## 13. Explicit non-goals

- Hub全体のcanonical ontologyや巨大Package graph
- LLMを必須にするRuntime/Atlas
- 全Resource body/Assessment/learner history embedding
- QTI級のuniversal assessment/activity schema
- 必要性が実証される前の任意plugin execution system
- Packageの全nodeを一枚の画面へ描くこと
- embedding similarityをsame-as identityやmastery transferと見なすこと
- CosmoOrder brand renameを理由にstable Package/Store schemaを一括改変すること

## 14. Phased implementation plan

### P0 — semantics / read contract / re-audit (schema変更なし)

1. このgap analysisとrename boundaryを設計基準に置く。
2. v0.1 `requires` = Package-local prerequisite、Curriculum = Objective sequence、progress = digest-scoped observed attempt projection、とUI/helpを含め明記する。masteryと呼ばない。
3. Coreの`query::inspect/query/context`を既存CLI/Desktopから再利用し、Package-local Atlasのview contractを確定する。新schema field・Store tableは作らない。
4. uninstall後historyの表示可否を実装コードとE2Eで追い、表示情報が不足するならP1の先頭に入れる。
5. `context`のboundedness、unknown field preservation、digest-separated progressの既存testsを回帰条件として固定する。

**次セッションが実装開始できる対象ファイル** (P0 scope):

- `crates/osmium-core/src/query.rs`: 既存query DTO/limit/contextを調査し、必要ならpackage-local Atlas用に不足するread-only summaryだけを追加。schema/Relation追加はしない。
- `crates/osmium-core/src/validation.rs` と tests: `requires`/cycle/topological semanticsを不変条件としてtest/doc化。Curriculum順序とは独立であることを確認。
- `crates/osmium-cli/src/commands.rs` / CLI tests: query/contextがP0 Atlas read contractを表現することを確認する既存CLI経路。
- `apps/desktop/src-tauri/src/commands.rs`: query/contextのthin command adapterが必要なときだけ追加。
- `apps/desktop/src/App.tsx`, `components/Lesson.tsx`, `components/Reader.tsx`, `ProgressPanel.tsx`, `HistoryPanel.tsx`: current state/Routeへ使える情報、deferred assumptionsを調査する。P0でUI再実装しない。
- `crates/osmium-store/src/lib.rs`, `runtime.rs`, tests: progress key/digest and retained history semanticsを確認する。P0は既存DB schemaを変えない。
- `docs/LEARNING_EXPERIENCE_GAP_ANALYSIS.md`, `docs/COSMOORDER_RENAME_AUDIT.md`: decision recordとなるこの2文書。

**P0 completion / test conditions**:

- `cargo test --workspace`、desktop `npm test`、`npm run typecheck`、`npm run lint`、`npm run build`、desktop E2Eがgreenであること（実行時は結果/環境を記録）。
- Core testsでconcept prerequisite cycle/dangling reference、context depth/node/byte bounds、Curriculumとprerequisiteのdistinctnessを確認。
- Store testsでsame digest reinstall後のprogress restoration、new digest isolation、uninstall後event retentionを確認。
- GUI/CLIから参照するDTOはCore/Packageが所有し、frontendにrelation resolution/validationを複製しない。
- Package authoring round-trip時にGUI未公開のextension/metadataが保持され、dangling参照を正常保存しない。
- P0終了でも新しいschema field、SQLite table/migration、embedding/model、Atlas canvasは導入しない。

### P1 — Package-local orientation

Package内のread-only Atlas prototype。LessonからRouteの「現在/次/なぜ」を出し、Atlasは局所表示。package graph/curriculum/stateの凡例、resource navigation、uninstall後historyの表示identityを整える。Progressは観測データとして表す。実使用評価でLesson理解・学習選択が改善しないならAtlasを広げない。

### P2 — Personal Atlas candidate cache

Descriptor契約、mapping provenance、SQLite derived cache、profile versioning、incremental install/update invalidationをdesign/benchmarkしてから、title/external-ID候補を実装。Embeddingはaccuracy/false-match reviewとmobile benchmarkを満たした場合のみopt-inで実行。ANNは計測後。

### P3 — richer practice / authoring disclosure

人間作者Level 1/2 UX、concrete remediation route、non-quiz practice use casesを評価。必要な場合にのみ最小Activity reference primitiveとevidence extensionを新schema proposalへ。実装前にold package compatibility/test fixture planを承認する。

### Future

Hub/publishing, community alignment, sync, shared publisher identities, advanced extensions, mastery model/transfer。現状成果物の検証と学習効果なしに着手しない。

## 15. Product principles to sharpen CosmoOrder

1. **道筋には理由を添える**: 次のlessonだけを薦めず、そのPackageのobjective/prerequisiteと現在の目的のつながりを説明する。
2. **教材と履歴をユーザーの手元に残す**: offline・reproducible assessment・exportable historyをUI機能と同じくらい大事にする。
3. **出典と確実性を分けて見せる**: 作者が書いたrelation、Runtimeの観測state、Runtimeの推定mappingを混同しない。
4. **拡大は選択可能にする**: Lessonを静かな既定値にし、Route/Atlasで必要時にだけ全体との位置関係へ広げる。
5. **構造は学びを助ける時だけ増やす**: GraphやActivityの豊かさを、それ自体の成果と取り違えず、作者負担と学習者の意思決定への効果で採否を決める。
