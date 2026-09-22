# Osmium 実装計画

更新日: 2026-09-22。状態: Phase 2bのCLI（`validate`/`lint`/`inspect`/`query`/`context`/`init`）を実装。Phase 3以降が次の作業。[Phase 2b引き継ぎメモ](PHASE2B_HANDOFF.md)は再開前の履歴資料。

## 1. 調査と要件の優先順位

開始時のリポジトリには `osmium_session_codex_context.md` のみがあり、全6,866行を読了した。コード、テスト、ビルド設定、Git履歴、リポジトリ内AGENTS.mdは存在しなかった。今回のgoal-objective.mdを現行の作業範囲とし、設計ログをコンセプトと長期的な境界の一次資料として扱う。元の設計ログは改変しない。

設計ログ中の初期案より、後半の敵対的レビューと最終モデルを優先する。今回の指示が求める最小のローカル縦切りを「最小v1」と呼ぶ。ログに含まれるHub、MCP、多数の問題形式、メディア、CSS等を含む広いv1案は削除せず、後続の拡張目標として区別する。

原則:

1. Packageはplain filesであり、UIやクラウドやLLMから独立する。
2. Concept / LearningObjective / Curriculum / Resource / Assessmentを別Entityにする。
3. 生のLearning Eventを残し、Objective単位の進捗を再計算可能にする。
4. SourceとDistributionを分け、安定IDとrevision/hashを分ける。
5. Packageは信頼しない。任意コード、暗黙のネットワークアクセスを許可しない。
6. 不明なoptional extensionは保存し、不明なrequired capabilityは理由付きで拒否する。
7. GUI、CLI、将来のMCPは同じCoreの意味論を使う。

## 2. 完成させる最小v1

ローカルのSourceを作成 → validate → build → install → Curriculum／Concept／Objective／Resource／Assessmentを表示 → 回答を採点 → Learning Event保存 → アプリを完全終了 → 再起動して履歴と進捗を復元、をネット接続なしで実行する。

含むものは、明示的schema version、JSON Schema、Markdown、single_select／booleanの決定的評価、directoryとZIP配布、SQLite、CLIと最小Desktop UI、互換性・安全性テスト。詳細は[SPEC_V1](SPEC_V1.md)。

最小v1に含めないものは、Hub、MCP実装、同期、AI、SRS、複雑な問題型、音声・動画再生、YouTube embed、Package独自CSS、任意HTML、コード実行、Package間依存解決。[ROADMAP](ROADMAP.md)に追加場所を残す。これらを未対応のまま対応済みとして受理・採点してはいけない。

## 3. 不明確な点・矛盾・リスク

| 論点 | 方針・解決期限 |
|---|---|
| v1範囲が広い | 今回の最小縦切りを優先、広い案は後続目標。DD-001 |
| Concept直結の旧Assessment例 | `measures`はObjectiveへ、古い例は規範にしない。DD-002 |
| prerequisiteと順序の混同 | ConceptのrequiresのみDAG検証、Curriculumは別のObjective配列。Phase 1 |
| YAML、JSONL、front matterの複数案 | 初期はmanifest YAML/JSON＋明示リストのJSON entities＋Markdown。Phase 1でschemaと例を固定 |
| schema_version 0.1/1.x案が混在 | 開発草案0.1、安定1.0は適合試験後。互換を推測しない。DD-003 |
| optional未知値の保存と厳格schema | 名前空間付きextensionsへ保存。未知coreキーは誤記として拒否。DD-004 |
| hash対象・自己参照・ZIP再現性 | manifestを除くpayloadのhash一覧、正規manifestのdigest、ZIPのdigestを区別。Phase 3前にバイト規則を確定 |
| 更新で古い回答の意味が変わる | versionとitem revision/hash、評価器version、問題snapshotを保存。Phase 4 |
| 表示HTML/ASTの永続化 | Markdownを残しASTは再生成、独自ASTを唯一の資産にしない。DD-005 |
| Windows固有path攻撃 | drive/UNC/ADS/device名、case衝突、reparse pointも拒否。Phase 2/3 |
| SQLiteとfilesystemの非原子的更新 | staging→検証→rename、DB登録、起動時reconcile。Phase 3/4 |
| fork後の同一性、履歴移行 | package IDを変えlineageを保存、進捗の自動統合はしない。後続仕様 |
| ライセンス未指定 | 配布前に利用者が選定。試験教材は自作し外部素材を無断同梱しない |
| Desktop prerequisites | MSVC SDK、WebView2の実利用はPhase 1/5で検証。未確認を成功と記載しない |

採用判断の理由と影響は[DESIGN_DECISIONS](DESIGN_DECISIONS.md)を参照。原則を変える必要が生じたら、実装前に判断を記録する。

## 4. 技術スタックと構成

| 層 | 採用方針 | 理由 |
|---|---|---|
| Spec | Markdown + JSON Schema 2020-12 | 言語・UIと独立した検証 |
| Core / CLI | Rust、Cargo workspace | 型と意味論を共有、native優先、後にWASM化可能 |
| Source | YAML/JSON + Markdown + assets | 通常のエディタとGitで編集 |
| Distribution | 正規化JSON + files、ZIP `.osmium` | directoryと同じ意味、特殊バイナリ不要 |
| State | SQLite（Rust adapter） | transaction、migration、ローカル履歴 |
| Desktop | Tauri 2 + React + TypeScript + Vite | CoreからUIを交換可能に分離 |
| Test | Rust unit/integration、UI test、Golden fixtures | 重要ロジックをGUIなしで確認 |
| 将来Hub | Workers / D1 / R2 / Queues | 既存のCloudflare方針を維持、protocolから隔離 |

ライブラリ候補はserde/serde_json、clap、JSON Schema validator、ZIP、SHA-256、SQLite adapter、CommonMark parser。Phase 1で保守状況・安全性・ライセンス・Windowsビルドを確認し、実際に導入するものだけlockfileへ固定する。YAML実装は重複キー、alias展開、深度制限が必須。未確認の最新版番号はここで決めない。

確認済み環境: Rust 1.98.1 / Cargo 1.98.1 / Node 24.15.0 / npm 11.12.1、Gitあり、Visual Studio Installerの検出ツールあり。MSVCツールセット・SDK・WebView2の使用可能性は未検証。

```text
README.md
docs/                       # 設計・仕様・判断・操作手順
spec/v0.1/                  # schemaとconformance契約、後にspec/v1/
crates/
  osmium-core/              # model、parse、validation、graph、assessment、content IR
  osmium-package/           # 安全なfilesystem/ZIP、build、install
  osmium-store/             # SQLite、migration、event/export、projection
  osmium-cli/               # コマンドとstructured output
apps/desktop/
  src/                     # React、交換可能renderer
  src-tauri/               # typed commands、Core呼び出し
fixtures/                   # valid / invalid / security / compatibility
examples/                   # 自作Golden Packageのsource
scripts/                    # 検証の入口（必要に応じて）
```

このツリーは推奨構成であり、空の将来サービスやplugin frameworkを一括生成しない。Hub/MCPは導入時に独立adapterとして追加する。

## 5. 小さな実装フェーズ

各Phaseは前の完了条件を満たしてから進む。大きいPhaseは表の成果物単位で分割コミットする。

| Phase | 成果物・主な変更先 | 完了条件と検証 | コミット例 |
|---|---|---|---|
| 0: 読解・計画 | `.gitignore`, README, docs, 原文 | 全文読了、計画整合、リンク/差分確認、初期Git commit | `docs: add initial implementation plan` |
| 1: Specと基盤 | Cargo workspace、spec/v0.1、fixtures、最小例 | 全Entity schema、response/evaluation制約、良/不正fixture、offline schema解決、fmt/clippy/test/build成功 | `chore: initialize project structure`, `feat: add package manifest schema` |
| 2a: loaderとvalidation | core model/parser/paths/graph | malformed、duplicate IDs/keys、missing ref、型不一致、cycle、path逃避を拒否。optional未知値のroundtrip、未知required/schema拒否 | `feat: implement package loader` |
| 2b: CLI | init/validate/lint/inspect/query/context | stdout JSONのみ、stderr/exit code契約、read-only検証、context件数/bytes上限、空・不正入力のCLI試験 | `feat: add package authoring cli` |
| 3a: build | package builder、canonical/hash契約 | 同じsourceから同じdigest、directory/ZIP意味一致、source不変、build再検証、hash tamper検出 | `feat: build portable packages` |
| 3b: install | library、staging、CLI install | directory/ZIP両方、サイズ/個数上限、zip bomb、symlink/reparse、collision、atomic install、同version差替え拒否、失敗時既存package保全 | `feat: install local packages safely` |
| 4a: assessment | core evaluation、CLI answer | single_select/boolean、無効回答拒否、feedback、再送のidempotency、問題snapshotと評価器version | `feat: add deterministic assessments` |
| 4b: Learning State | store/migrations、CLI history/progress/export-state | append-only event transaction、別process再起動後一致、projection再構築一致、失敗rollback、migration/backup | `feat: add local learning state storage` |
| 5a: reader | desktop scaffold、content IR、安全なMarkdown renderer | install/open、Concept/Objective/Curriculum/resource表示、HTML/危険URL非実行、package本文からIPC不能、offline smoke | `feat: add markdown resource renderer` |
| 5b: 学習UI | 回答・feedback・履歴・進捗 | 全縦切りをDesktopで実施、完全終了後復元、keyboard/focus/labels/contrast/200%文字拡大確認、UI build/typecheck/test | `feat: complete offline learning workflow` |
| 6: 適合性・運用文書 | Golden Packages、security/compatibility tests、README/docs | 自作数学・語学・プログラミング教材で再現。実データ破壊なしのend-to-end、旧fixture維持、全checks成功、制約明記 | `test: add package compatibility coverage`, `docs: document package format` |

Phase 1でschema草案を実ファイルにし、初回のGolden Packageを同時に作る。Phase 6まで実例を後回しにしない。医学の画像付き連問は後続のmedia/assessment拡張を検証する代表例として残す。

## 6. テストと受入れシナリオ

Schema検証だけで意味的正しさを保証しない。参照解決、前提DAG、回答候補と正解の整合、capability対応はCoreで検証する。lintの充足率を教材品質の保証にしない。

最終受入れは、一時的な空のOSMIUM_HOMEにfixtureからbuild/installし、元sourceを参照しなくても読むこと、正答・誤答で別eventsが残ること、アプリとCLIを終了して新processから復元すること、projectionを消して再生成して同じ結果になること。ネットワークを使わず、2つ目のpackage/versionの導入で旧履歴を変えないことも検証する。

Rust変更では `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace`。UI導入後はnpm scriptsのlint/typecheck/test/buildと実機Desktop smokeを追加する。失敗は理由・影響・再現手順を記録し、根拠なく成功としてコミットしない。

Git管理から依存物、生成物、ログ、cache、秘密、ローカルDBを除外し、lockfiles、schemas、source fixturesは追跡する。コミット前にstatus/diff、秘密情報の混入を確認する。pushと履歴書換えは行わない。

## 7. 進行記録

- Phase 0: 全Markdown読了、Git初期化、技術環境確認、文書作成。ローカル文書リンク11件、新規文書のwhitespace、依存物・secret・DB除外と設定例/lockfileの非除外を検証し成功。コード未実装のためlint/typecheck/test/buildは未適用。元の設計ログは原文維持のためwhitespace修正対象外。所有者情報を記録しないfilesystemに対するGitの警告には、当該repositoryだけをコマンド単位のsafe.directoryで指定し、global設定は変更していない。初期commitは `docs: add initial implementation plan`（hashはコミット後の報告とGit履歴を参照）。
- Phase 1: Cargo workspaceと `osmium-core::schema`、自己完結したJSON Schema 2020-12 bundle、Concept/Objective/Curriculum/Resource/Assessment/LearningEvent、最小日本語教材、良/不正fixtures、構造診断を実装。変更先はCargo.toml/Cargo.lock、crates/osmium-core、spec/v0.1、examples/arithmetic、fixtures、READMEと関連設計文書。fmt、clippy（warnings拒否）、offline/locked test（6件）、offline/locked build成功。Rust nativeのMSVCビルドは利用可能。Tauri/WebView2は未検証。HTTP/file schema解決featureが無効であることをcargo treeで確認。コミット名は `feat: add package schema and offline validation foundation`（hashはGit履歴および完了報告を参照）。
- Phase 1の制約: 構造validationのみ。YAML/JSON loaderの重複key拒否、参照・循環・path安全性、BCP 47構文、capability対応、実際のEvent整合、Distribution/GUIは後続。digest形状fixtureは真正なbuild結果ではない。Schemaを通るだけで外部教材をインストール可能とは判定しない。
- Phase 2a（途中）: Coreの読取り専用PackageModel、重複ID、型付き参照、Concept requires循環、選択肢と正解、未知required capability、相対path字句規則を実装。JSONの4 MiB制限、重複キー拒否、実際のsyntax位置の診断も追加。変更先はcrates/osmium-coreのparsing/validationとintegration tests、関連文書。Phase全体は未完了で、YAML、filesystem containment、symlink/reparse、Unicode path衝突、BCP 47、ファイル存在検証が残る。19テスト（既存6＋新規13）を追加・実行し、2,048 Conceptの前提chainも確認。Rust incremental cacheのhardlink警告はfilesystem由来でcopy fallbackにより処理継続。コミット名 `feat: validate package semantics and reject ambiguous json`、hashはGit履歴と報告参照。
- Phase 2a完了: `osmium-package::load_source`、制限付きYAML parser、BCP 47構文検証を追加。静的Sourceのcontainment、symlink/reparse（Windows junctionを実際に作成して試験）、NFC/case path衝突、file存在・拡張子・UTF-8、4 MiB/file・64 MiB/tree・4,096 entries・深度32を検証。YAMLの重複key/alias/anchor/tag/merge/multiple documents/深度超過も拒否。JSON/YAMLから同じモデルを構成し、optional extensionを保持。追加先はcrates/osmium-package、crates/osmium-core/src/yaml.rsとtests、Cargo依存、仕様文書。テスト合計33件、fmt/clippy/buildを検証。コミット名 `feat: load bounded local packages with json and yaml manifests`（hashは履歴・完了報告参照）。残制約: Unix実機未検証、悪意ある別processがSource親directoryを継続的に差替える競合の完全防御は未保証。Phase 3の隔離staging・再検証で配布物の安全性を確定する。CLI/ZIP/install/UIはまだ未実装。
- Phase 2b（未コミット）: CLIの薄いadapter `crates/osmium-cli`（`osmium`バイナリ）を追加し、`validate`/`inspect`/`query`/`context`/`init` を実装。Coreへ純粋な意味ビュー `osmium-core::query`（inspect/query/context、上限 `MAX_QUERY_LIMIT`=256、`MAX_CONTEXT_NODES`=512、`MAX_CONTEXT_BYTES`=256KiB、`MAX_CONTEXT_DEPTH`=8）を、Packageへ非破壊scaffold `osmium-package::init` を追加。CLIは検証ロジックを一切持たず、引数解析・envelope整形・exit code決定のみを行う。stdoutは単一JSON envelope、stderrは診断1行、exit codeは0成功/1対象不正/2呼出し不正/3 I/O内部/4 schema・capability非互換。テスト合計70件、fmt/clippy/build成功。`osmium lint` とPhase 3以降は未実装。作業再開用の詳細は [Phase 2b引き継ぎメモ](PHASE2B_HANDOFF.md) を参照。残課題: `lint`未実装、distribution入力（ZIP/directory）のvalidateはPhase 3、Desktop未着手。
- Phase 2b完了（Codex引継ぎ後）: DeepSeekの未コミット実装を読解し再検証。lintをCoreへ追加し、CLIはwarning付きの成功envelopeを返す。`--json`追加、inspect上限拒否、context対象およびDTO全体のbytes制限、initのID文法/既存YAML/リンク親拒否、create_newと生成後validation失敗時のfile rollback、I/O exit codeを修正。変更はcrates/osmium-core/query・lint、crates/osmium-package/init、crates/osmium-cliと文書。75件のworkspace testsと追加context境界テスト1件、clippy/fmt/buildを検証。実バイナリのstdoutを検証するprocess試験を含む。残課題はdistribution build/install、評価/永続化、Desktop、最終縦切り試験。悪意ある同時filesystem書換えの完全防御は依然保証しない。コミットhashはGit履歴と完了報告を参照。
- Phase 3以降: 各完了時に、実装内容・変更ファイル・検証結果・残課題・commit hashをここへ追記し、利用者にも簡潔に報告する。利用者の最新指示に従い、動作するv1を最終検証した時点で開発を区切り、起動方法を提示する。

## 8. 技術判断の参照

既存設計案の実装可能性を確認するため、2026-09-22に一次資料を参照した。

- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/): WindowsのC++ Build Tools / WebView2要件。
- [JSON Schema 2020-12](https://json-schema.org/draft/2020-12): schemaのdialect。
- [SQLite application file format](https://www.sqlite.org/appfileformat.html): ローカル保存の選択を検討。教材正本は既存方針どおりfilesにする。
