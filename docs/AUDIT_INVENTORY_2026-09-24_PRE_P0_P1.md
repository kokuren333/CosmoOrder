# Osmium Audit / Inventory — 2026-09-24

この監査は、現作業ツリーのコード、テスト、DesktopのWindows WebView E2Eを対象にした。監査中は機能コードとschemaを編集していない。既存の変更・削除・未追跡ファイルは保持している。

## 1. Executive summary

Osmiumは、ローカルで教材を検証・配布・導入し、学習者がMarkdown教材を読み、`single_select`／`boolean`問題に回答し、履歴から進捗を再構築するところまで実行できる。CLIとDesktopはCore、Package、Storeの同じ主要処理を利用する。Windows Desktop E2Eでは、教材作成から保存・検証・install・学習・回答・再起動後の履歴復元までを実際に確認した。

現状は「単一Concept／Objective／Resourceを対象にしたGUI authoringと、複数Entityを読むlearner runtime」の段階である。SchemaとCoreはより大きいgraphやAssessmentを表現・検証・実行できるが、Desktop AuthoringでEntity群や関係を編集できない。Libraryにはempty/error/loading UXの不足があり、複数versionを選択して開く経路も欠ける。update/uninstall、assessment作成UI、同期、Mastery/SRSは未実装。

## 2. Current architecture

| 領域 | 役割 / 主なAPI | 現在使われる経路 | 未使用・限定事項 |
|---|---|---|---|
| `apps/desktop` | Tauri 2 + React/TypeScript。Library、Lesson、Reader、Assessment、Progress、History、Authoring。`client.ts`がIPC DTOを呼ぶ | React → Tauri commands → Package/Core/Store | UIは複数Entityの編集不可。Native folder chooserは存在するがE2Eでは呼び出しを注入で置換 |
| `crates/osmium-core` | I/Oなしのschema、JSON/YAML文書、意味検証、lint、参照・graph query、Markdown IR、評価、progress projection | `osmium-package`がloader/build前に利用、CLI query/inspect/answer、Store評価 | schema上の拡張値を実行する機構はない。Mastery/SRS engineなし |
| `crates/osmium-package` | `load_source`、`authoring::{open_source,save_source}`、`init::{init_source,init_source_with_generated_id}`、`distribution::{compile_source,build,read_distribution}`、`library::Library`、Reference編集 | CLIとDesktopでsource、build、install、distribution読み込みを共用 | Authoring editorは最初のConcept/Resourceのみ編集。YAMLはread-only |
| `crates/osmium-store` | SQLite event log、回答のtransaction/idempotency、progress/history、backup/export。`runtime::Runtime` | CLI/Desktopの学習・install操作 | package更新やuninstall APIなし。進捗は観測統計で、習得推定ではない |
| `crates/osmium-cli` | clap引数、JSON envelope、Core/Package/Runtimeへのadapter | standalone CLIおよびE2E fixture導入 | 引数・終了コード・出力DTOはCLI固有。ドメイン検証を別実装してはいないが、全操作を包むapplication facadeはまだない |
| `spec/v0.1` | Source/Distributionと各EntityのJSON Schema 2020-12定義 | Core埋込schema、fixtures、文書 | この作業ツリーでは`package.schema.json`自体が既存WIPで変更済み。HEADとの差分を含むため、ここでいう監査対象はリリース済みbaselineではなくworking-tree版 |
| `fixtures/`, `examples/` | valid/invalid fixtures、Arithmetic golden package、medicine/math/language/programming等のpressure-test package | Rust tests、CLI、Desktop E2E | pressure-testの網羅性は製品データや公開packageの保証ではない |
| `apps/desktop/e2e` | Windows実アプリをCDP経由で操作。CLIからfixture packageをbuild/install | 通常buildされたTauri exe + WebViewを使用 | `VITE_OSMIUM_E2E=1`時はfolder pickerをwindow path injectionで代替。OS dialog自体は未検証。Windows以外の実機確認なし |
| `docs/` | Architecture、schema、format、authoring、assessment、provenance、identity等 | 設計・実装の説明 | 一部文書はPhaseの履歴表現を含む。最新の実装状態と一致するか各記述を読む必要がある |

依存方向は概ね `CLI/Desktop → Core + Package + Store`、Package loader/buildがCoreに検証を依頼、Store RuntimeがLibraryとSQLiteを束ねる形で保たれている。OS/networkをCoreへ持ち込んでいない。

## 3. Working user flows

| Flow | 観測結果 | 分類 |
|---|---|---|
| Library起動・0件 | 空状態と「sourceから教材を追加」が出る。empty表示自体はReact testと隔離homeのGUI操作で確認 | 動作。ただし起動時loadingと真のemptyを区別しない初期瞬間がある |
| Libraryから追加 | CTAはAuthoring画面へ移動。新規作成と既存source選択を次画面で選ぶ | 動作するが1段余分で、「既存教材の追加」への直接導線ではない |
| install済み教材一覧 | Libraryがversionごとのcardを表示し、title、version、Concept/Objective/Resource/Assessment数を出す | 動作 |
| 教材を開く・Concept/Objective | curriculum順で複数Concept/Objectiveを表示。prerequisiteは検証・queryされるが依存graphの視覚化や適応選択はない | 動作、表示とnavigationに限定 |
| Resource / Markdown | CoreでcompileしたIRをDesktopで表示。unsafe HTML/linkやimageは実行せず、安全なlinkとalt text等を表示 | 動作 |
| Assessment / 回答 | `single_select`と`boolean`の選択、採点、feedback、Learning Event記録が動く | 動作。試験監督・不正防止の仕組みではない |
| Progress / History | Objective別attempt/correct/accuracy等と回答eventを表示。再起動後も復元 | 動作。resume地点やMastery判定ではない |
| 新規教材 | name/language/save locationでscaffold作成。自動ID、JSON編集、save、Core validation、build/install、learner previewへ進む | 実GUI E2E成功 |
| 既存source | folder chooser、JSON source editor、診断、save、validate、install previewが動作。valid/broken fixtureを実GUIで確認 | 動作。chooser操作はE2Eでpath injection代替 |
| YAML source | loaderは受理するがGUIはread-onlyと明示。編集・保存不可 | 部分実装 |

画面上のempty/error表示は、初期loadingとの区別が不十分。`Packages.tsx`は`packages.length === 0`で空状態を描き、初期refresh失敗は診断へ送るがLibrary内に再試行可能なerror stateはない。

**複数versionのGUI制約:** list cardはID/version単位だが、`Packages.tsx`の選択callbackはIDだけを渡す。`openPackage`もversionなしで`openLesson(packageId)`を呼ぶ。Storeは複数versionが一致すると`OSM_VERSION_REQUIRED`を返す。version共存自体は動くが、GUIから曖昧さを解消できない。

## 4. Feature capability matrix

`Yes`は同じ層で機能が実装済み、`Partial`は一部の操作・表示だけ、`No`はその層に操作がないことを示す。

| Capability | Schema | Core | CLI | Desktop Authoring | Learner GUI |
|---|---:|---:|---:|---:|---:|
| Package metadata/title | Yes | Yes | Yes (`init`/`inspect`) | Partial (title/langの編集) | Yes (title表示) |
| Package ID | Yes | Yes (validation/lookup) | Yes (init任意指定、install/query) | Yes (API自動生成、Advanced表示/コピー、通常編集なし) | Partial (Developer detailsのみ) |
| Language tag | Yes | Yes (BCP 47構文検証) | Yes (`init --language`) | Partial (新規は10言語select+Other、既存はcode文字列) | Partial (本文言語の設定だけ。自動翻訳なし) |
| Package version | Yes | Yes (format/identity pair) | Yes (install/read指定) | No (scaffold値をGUI編集しない) | Partial (一覧表示はするがversion選択してopen不可) |
| 複数Concept | Yes | Yes (参照・循環・query) | Yes (`query`,`context`,`inspect`) | No (先頭1件のみ) | Yes (curriculum/Conceptを表示) |
| Objective | Yes | Yes (参照/graph/progress) | Yes (query/context/progress) | No | Yes (Concept配下で表示) |
| 複数Resource / relation | Yes | Yes (path/reference検証、query) | Yes (query/context/read) | No (先頭Resourceのtitle/bodyのみ) | Yes (全Resourceを表示・読む) |
| Curriculum / ordering | Yes | Yes (型/参照検証) | Yes (inspect/query/context) | No | Yes (順序・グループ表示) |
| prerequisites | Yes | Yes (参照/循環、順序) | Yes (inspect/context) | No | Partial (前提順で構造表示。graphと適応制御はなし) |
| Assessment: single_select/boolean | Yes | Yes (構造/answer検証、採点) | Yes (`answer`,`read`,`query`) | No (scaffold以外を作成・編集不可) | Yes (両形式を回答) |
| Reference/Evidence | Yes | Yes (aliases、存在、visibility、query policy) | Partial (add/attach/list) | Partial (診断/reviewに表示、編集不可) | Yes (公開・attributionを表示、privateを除外) |
| Provenance | Yes/Workspace convention | Yes (validation/sanitization境界) | Partial (sourceの検証/build) | No (provenance UIなし) | No (authoring provenanceを配布・表示しない) |
| Visibility | Yes | Yes (validation/sanitization) | Partial (Reference互換visibilityを指定) | No | Yes (public/attribution/private規則を反映) |
| Distribution + digest | Yes | Partial (構造schema。file I/O/hashはPackage) | Yes (`build`,`install`,`packages`) | Partial (validate後install preview。artifact出力操作はない) | Partial (検証済みinstalled packageだけ読む) |
| Multiple entity authoring | Yes | Yes | Partial (Reference編集以外はraw source編集) | No | Read-only表示はYes |
| Mastery / SRS / resume | — | No | No | No | No |

## 5. Schema/Core/CLI/Desktop/Learner matrix

| Feature | Schema | Core | CLI | Authoring GUI | Learner GUI |
|---|---|---|---|---|---|
| package metadata | title/id/version/language/extension等 | format・意味を検証 | init/inspect | title/languageのみ | title等を表示 |
| Concept/Objective | 複数配列とtyped relation | relation/存在/重複検証 | query/context/inspect | 既存の先頭Concept titleのみ | 複数を表示 |
| Resource | 複数、Markdown path、Objective relation | pathとrelationを検証しMarkdown IRを生成 | query/read/reference操作 | 先頭Resourceのtitle/bodyのみ | 複数を読める |
| Assessment | single_select/boolean、feedback/revision | 構造・回答の検証とdeterministic evaluation | answerで回答可能 | scaffold以外の編集不可 | 回答・採点結果・feedbackを表示 |
| prerequisite/reference | Concept prerequisite、manifest registry、Evidence link | graph cycle、参照切れ、visibilityを理解 | query/context、Reference add/attach/list | 編集不可 | relationと閲覧可能なEvidenceのみ表示 |
| provenance/visibility | schema項目と`.osmium/` authoring workspace | private情報の検証・distribution sanitization | Reference visibilityを一部設定 | 通常編集UIなし | private情報を出さず、public/attributionを反映 |
| distribution | distribution manifest/files/hash定義 | schema部分。実file検証はPackage | build/install/packages | install経路のみ | installed distributionを読む |
| language | BCP 47意図の文字列 | 構文検証 | initで設定 | 新規language select、既存language raw text | package本文は原語のまま表示 |
| version / ID | 必須。IDは`namespace/name`、versionはsemver subset | ID/versionをruntime keyに使う | install/readで指定可能 | ID自動生成、versionは固定/scaffold値 | versionカード表示、選択openは不足 |

Working treeの`spec/v0.1/package.schema.json`、Core、examplesには現在追加済みのReference、visibility、license status、cognitive level等がある。これは「この作業ツリーで動く契約」の評価であり、clean `HEAD`または外部向けstable schemaが同一だとは断定しない。

## 6. Package lifecycle

| 操作 | 状態 | 実態 |
|---|---|---|
| install | Implemented | Distributionを再検証し、stagingからdigest directoryへ公開。Desktop/CLIともRuntime経由 |
| 同じID/version・同じdigest再install | Implemented | 冪等でalready-installedを返す |
| 同じID/version・違うdigest | Implemented (拒否) | `OSM_VERSION_CONFLICT`。同じversionの内容変更を上書きしない |
| 違うversionの共存 | Implemented | digest directoryで共存。実E2E/testで確認 |
| update | Missing | 明示的なupdate/upgrade API、選択、rollbackなし。新versionをinstallする操作まで |
| uninstall | Missing | UI/API/CLIにpackage removalなし |
| source削除 | Partial | installed filesはSourceからコピーされたimmutable distributionなので残る。Sourceへの追跡・relink機能はない |
| source移動 | Partial | install済みcopyには影響なし。編集には新しいpathを選んで開く必要がある |
| 保存場所 | Implemented | `<data-root>/library/<digest>/`, `<data-root>/state.sqlite`, `staging/`。Windows既定は`LOCALAPPDATA/Osmium` |
| digest検証 | Implemented | install/list/read時にfile manifestとdigestを再検証。tamperは報告する |
| corrupt package | Partial | 検証でerror。自動修復/再取得/uninstall UIはない。破損entryがあるとLibrary list取得自体がerrorになりうる |

Package IDはv0.1で必須、`namespace/name`形式。Runtime/StoreはID+versionをinstalled package identityとして使用し、同一pairのdigest衝突を拒否する。distributionはIDを保持する。reverse-domain風syntaxは所有権を証明しない。現状はpackage identityとnamespace/name風文字列を一つに詰めた互換契約であり、publisher identityやslugの保証ではない。詳細は[`package-identity.md`](architecture/package-identity.md)。

## 7. Source format support

| Format | Open | Edit / Save | Validate | Build | Install |
|---|---|---|---|---|---|
| JSON manifest + entity JSON source directory | Yes | Partial: title/language、先頭Concept/Resourceの4項目だけ | Yes (Core経由) | Yes (Package API/CLI) | Yes (Desktop/CLI) |
| YAML manifest source directory | Yes | No: GUI read-only。Package loader/parserは読める | Yes | CLI/APIは可能 | CLIは可能。Desktop editor installはeditable要件で対象外 |
| Markdown resource file | packageからread | Partial: editorが先頭Resource本文を保存 | sourceとして検証 | packageとしてbuild | packageとしてinstall |
| Source directory | Yes | manifest format等に応じて上記制限 | Yes | Yes | Yes |
| Distribution directory | CLI/APIでread | No | Yes | artifactから再buildはCLI/API経路あり | Yes |
| `.osmium` ZIP distribution | CLI/APIでread | No | Yes | No (既存archiveからbuildしない) | Yes |

Desktopの既存source pickerはauthoring Source directoryを対象とし、built artifactをSource editorへ変換するUIではない。Entity文書自体はJSONで、単体JSON文書をGUI raw editorで任意編集する機能もない。

## 8. Test status

2026-09-24に現working treeで実行:

| 検証 | 結果 | 主な保証 / 制約 |
|---|---|---|
| `cargo fmt --all -- --check` | Pass | workspace Rust format |
| `cargo check --workspace` | Pass | workspace compile/typecheck |
| `cargo test --workspace` | Pass: Rust 163件、失敗0 | Core、Package、Store、CLI、Desktop Rustのunit/integration。doc testsは0 |
| `cargo clippy --workspace` | Pass | warning-level lint含むがdeny failureなし |
| Desktop `npm run typecheck` | Pass | TypeScript |
| Desktop `npm run lint` | Pass | oxlint、deny-warnings |
| Desktop `npm test` | Pass: 19件 | IPC/error projection、Markdown、安全な表示、navigation、presentation、Assessment radio interaction等 |
| Desktop normal `npm run build` | Pass | Vite build。JS chunk 736 kBで500 kB warning |
| Desktop E2E | Pass: 233 checks | Windows Tauri exe/WebViewをCDP操作。create/edit/save/validate/install/reader/assessment/progress/history/restart、複数domain package、visibility、安全なMarkdown、表示倍率/dark mode等 |
| `git diff --check` | Pass | 改行変換warningのみ。whitespace errorなし |

Rust内訳: Core 72 (unit 11 + integration 61)、Package 43 (unit 12 + integration 31)、Store 8、CLI 27 (unit 3 + integration 24)、Desktop Rust 13 (unit 9 + integration 4)。例・fixture依存のテストが多く、source/arithmetic、domain pressure-test、valid/broken authoring fixtures、tmpdirを使う。

E2Eは専用`VITE_OSMIUM_E2E=1` buildで保存pathをtempへ注入した。初回、専用buildなしでE2Eを走らせた時はAdvancedの保存path assertionで失敗し、Create前に停止した。E2E専用buildを作り直した後は233 checks pass。初回のdefault save-directory問い合わせで作成された`Documents/Osmium/Courses/gui`は空で、教材は作られていない。OSのnative folder chooser自体は手動操作していない。通常Vite buildは最後に再生成し、E2E注入なしの状態に戻した。

Coverage gaps: native chooser、空Libraryを含む起動直後からの実機再試験、複数versionをGUIから選択するflow、update/uninstall、Unix/macOS実機、networkやpublisher運用はE2E未対象。複数version共存/collision、Storeの再起動・履歴、Archive tamperingなどはRust testsでカバー。

## 9. UX findings

- LibraryとAuthoringは画面上区別されるが、`source`、`package`、`Core`、`Install`、`Concept`等の開発語が通常文言に残る。
- 「sourceから教材を追加」はAuthoringへ遷移するだけで、新規作成／既存sourceを選ぶ段階が後にある。
- 新規作成フォームは教材名、language、保存先が主面にあり、10言語selectと「その他」、既定Documents配下、`変更`ボタン、Advanced内のID/raw code/absolute pathを持つ。通常フォームは内部語を露出しないことをE2Eで確認。
- `Package ID`はPackage APIがslug+UUIDを生成し、Advancedでread-only表示・copy。タイトル変更でIDを再生成しない。
- dirty state、保存前validate/installのdisable、保存後に再検証が必要な表示はある。YAML read-onlyも画面に説明される。
- Library initial stateはpackages `[]`で始まるため、取得完了前の一瞬にemptyが出うる。refresh error用のLibrary内retry表示も弱い。
- version別cardの選択時にversionを渡さず、複数versionがある教材はGUIで開けない。
- DeveloperDetailsはcollapsed disclosure。empty LibraryやAuthoring Reviewには一部technical wordingが見える。

Pressure test: 一般ユーザーは新規scaffoldを作り、タイトルを編集し、本文を読み、学習問題へ回答するところまでUIで可能。ただし複数Concept/Resource/Assessmentを関係づける実制作はGUIだけで完結しない。CLI/schema知識を不要にする目標は「単一Entityのstarter package」には近づいているが、通常の教材制作全体には未達。

## 10. Known limitations

### Fully working

- Source directoryの安全な読込み・schema/意味検証・lint・配布build・digest照合・install。
- 複数versionの保存とbackend上の共存、同version衝突拒否。
- Markdownからrenderer-neutral IRを生成し、安全に表示。
- LearnerのConcept/Objective/Resource/Curriculum表示、single_select/boolean評価。
- Learning Event追記、request idempotency、Progress projection再構築、History/backup/export。
- Desktop単一Entity scaffold/edit/save/validate/install/preview、CLIのbuild/install/inspect/query等。

### Working but limited

- Desktop Authoringはpackage metadata + 先頭Concept + 先頭Resource本文だけ。
- YAMLはread-only、Assessment/Objective/Curriculum/Reference関係のGUI編集なし。
- 複数Entity learner表示は動くが作成/graph編集はGUIでできない。
- installed versionは共存してもLibraryからversion選択不能。
- Reference visibilityとprovenance分離はCore/Package/Runtimeで実装済みだがGUI編集は狭い。

### Partially implemented

- Library loading/error/empty stateの区別、source追加導線。
- Package lifecycleのreinstall/installは堅牢、update/uninstall/repairは未接続。
- Distribution artifactのbuild/read/installはあるがDesktopから`.osmium`のimport/export操作はない。

### Skeleton / placeholder

- Adaptive next concept/mastery/SRS/resume UIはなし。Prerequisite順序はvalidation/query/display上の概念。
- future Hub/MCP/publisher identityはarchitecture上の候補のみ。

### Not implemented

- 複数Entityと関係のAuthoring GUI、Assessment作成UI。
- Package update/uninstall UI/API、Hub、sync、publisher verification。
- Mastery保証、spaced repetition、adaptive next concept選択。

### Current longest practical flow

New package (scaffold) → title/languageと先頭Concept/Resourceを編集 → save → Core validate → Package Distribution build → Runtime install → Library/learner preview → Markdown read → single_select/boolean answer → Learning Event → Progress/History → app restart後に復元。

### Technical debt

- `namespace/name`を持つ必須package IDをruntime identityにも使う。CLI/GUIは異なるadapterだが共通application-operation facadeはない。
- App stateのversion識別が画面選択まで一貫しない。
- Schema/Core/CLI/examplesのworking-tree変更が多く、clean released baselineとworking contractが見分けにくい。
- DistributionとDB登録は別resource transactionであり、孤立staging/reconciliationを含めたrecovery policyに未実装範囲が残る。
- asset/imageはtyped package resourceでなくalt textへ縮退。

### UX debt

- technical vocabulary、empty CTAからsource選択までの余分な画面、loading/error状態。
- 複数version chooser、複数Entity editor、Assessment authoring、update/uninstall。
- Raw language codeは既存source編集時にそのまま表示される。

### Schema capability not yet used end-to-end

Assessment定義のGUI作成、複数Concept/Objective/Curriculum/Resourceとrelation編集、prerequisite graph編集、Reference/Evidence authoring GUI、Distribution export/import UI、publisher/slug分離、typed media asset。

## 11. P0 / P1 / P2 gaps (implementationはこの監査では行わない)

| Priority | Gap | 根拠 |
|---|---|---|
| P0 | Libraryのloading/loaded-empty/errorを分離し、失敗時に再試行できるようにする | 起動時false-emptyと回復導線はinstall済み教材への信頼を損なう |
| P0 | Libraryからversionを明示して教材を開く | Backendが複数versionを正式に許すのにGUIでは曖昧化されている |
| P0 | Library「教材を追加」から新規作成／既存sourceの選択を直接案内 | learner/authoringの境界と追加flowが初見で不明瞭 |
| P0 | 複数Concept/Objective/ResourceおよびrelationをGUIで編集する基礎primitive | Osmiumのpackage graphを実際の教材制作に使う前提 |
| P1 | single_select/boolean Assessment authoringとGUI上のobjective/resource relation | 作った教材を回答・履歴までend-to-endで作者が作れるようにする |
| P1 | `.osmium` import/exportとupdate/uninstall/version lifecycleを整える | artifactとinstalled packageの運用に必要。ただし内容のmigration semanticsを先に固定しない |
| P1 | YAML edit/save方針（read-only表示のままか対応するか）を決める | 対応形式とUI期待の不一致を解消する |
| P2 | prerequisite/curriculum graphの視覚化、adaptive selection、Mastery/SRS | 基礎のauthoringとlifecycleが整ってから検討すべき |
| P2 | Hub publisher identity、slug、immutable identityのschema設計 | publish ownership/migrationの要件が揃った段階でADR化する |

## 12. 「今のOsmiumを一文で言うと何か」

**Osmiumは、検証可能なpackageをローカルで配布・installし、構造化された教材を読んで回答履歴と進捗を記録できる縦断基盤であり、GUI Authoringはまだ単一Concept/Resourceのstarter editorに限られる。**

## 監査コマンドと作業範囲

- Pass: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace`
- Pass: Desktop `npm run typecheck`, `npm run lint`, `npm test`, normal `npm run build`, Desktop E2E (233 checks)
- Pass: `git diff --check`
- 実施しなかったもの: native OS directory chooserをマウス操作する手動試験、Windows以外の実機試験、Hub/ネットワーク/publisher運用試験。
- この監査で追加したのは本ドキュメントのみ。Source、schema、tests、既存WIPは変更・削除していない。
