# CosmoOrder

Authoring contracts for content language, References and Evidence, privacy visibility, and the CLI/MCP/Skills boundary are documented in [`docs/AI_AUTHORING.md`](docs/AI_AUTHORING.md), [`docs/SOURCES_AND_PROVENANCE.md`](docs/SOURCES_AND_PROVENANCE.md) and [`docs/AUTHORING_WORKSPACE.md`](docs/AUTHORING_WORKSPACE.md). The package `language` is content metadata and is independent of Desktop UI locale. CLI provides structured JSON init, validate, lint, build, inspect, query, context, and three Reference authoring operations; other entity authoring remains file-based.

CosmoOrderは、教材を、機械が読め、人間が監査でき、AI agentも改善できる再利用可能な学習資産として扱うためのオープン学習基盤です。AIの計算資源を毎回同じ説明に使うだけでなく、一度作った教材を何度でも学べる形で残すことを重視します。**Build once. Learn many times.**

現在はローカルでのPackage作成・静的検証・配布物のbuild/install・学習・イベント履歴保存を行うCLIと、Tauri 2 + ReactのDesktopを実装しています。AI Tutor、AI採点、AIによる自動教材生成、MCP authoring、Hubは未実装です。決定的なCLIと機械可読なPackageを、将来のAI authoring integrationの足場とします。

## 3つの構成要素

| 要素 | 役割 | 現在の状態 |
| --- | --- | --- |
| Learning Package | Concept、Learning Objective、Curriculum、Resource、Reference／Evidence、Assessmentの意味と教材本文を保持 | 開発版schema 0.1、Referenceと配布形式を実装 |
| CosmoOrder Runtime | Packageをvalidate、render、assess、executeし、Learning Eventを記録 | Core、CLI、Desktop、SQLite Storeを実装。progressはイベントから導出 |
| Hub | 将来のRegistry／配布サービス | 未実装 |

Packageに個人の回答履歴は書き込みません。`validate` は構造と参照の整合性を確認し、`lint` は教材の網羅性を警告します。どちらも教育的正確性や専門家の確認を保証しません。

## 何を混ぜないか

```text
Reference            = learner/distributionから到達できる知識資源
Evidence             = Resource/Assessmentの内容を支えるReferenceとの関係
Authoring Provenance = 制作に使った入力と履歴（.osmium/）
```

この3つは別の場所に置きます。医学fixtureには学習者向けのReferenceだけを置き、非公開の制作メモやlocal pathはPackageの外（gitignoreされた`.osmium/`）へ置きます。`private` visibilityは古いPackageを読めるようにするための互換経路であり、制作入力をPackage内に保持する推奨手段ではありません。詳細は[`docs/SOURCES_AND_PROVENANCE.md`](docs/SOURCES_AND_PROVENANCE.md)と[`docs/AUTHORING_WORKSPACE.md`](docs/AUTHORING_WORKSPACE.md)を参照してください。

可視性は `record_visibility`（記録を公開できるか）と `locator_visibility`（locatorを公開できるか）の2軸に分離し、旧 `visibility` enumは互換ビューとして維持します。再利用条件はfield presenceではなく意味で判定し、未設定は `{"license_status":"unknown"}` として明示します。公開URLは通常のquery（`?id=`、`?lang=`）を許可し、credential様queryと埋め込みuserinfoのみ拒否します（完全なsecret検出は保証しません）。

**検証可能性のレベル**は区別して扱います。Resource-level traceability（Resource→Reference）は実装済みです。claim-level verifiability（この文がどのReferenceに基づくか）は未実装であり、実装済みとして扱いません。

## 本文のレンダリング境界

Package本文はPackage層が安全に読み込み、Coreの`compile_markdown`が一度だけ型付きContent IRへ変換します。Desktopへ渡す本文DTOはこのIRを含み、DesktopはMarkdown sourceを再解析しません。Runtimeは共通IRを描画し、数学やコードの装飾だけを表示層で行います。raw HTMLは不活性な文字列として扱い、Package内コードは実行しません。RuntimeごとにMarkdown parserを持たせないことを設計原則とします。詳しくは[アーキテクチャ](docs/ARCHITECTURE.md#content-rendering-boundary)を参照してください。

Assessmentも同じ責務分離を文書化しています。Assessment Item、Interaction、Response Model、Scoring、Feedback、Result、Presentationを分け、definitionとlearner resultを構造的に混在させません（[`docs/ASSESSMENT_MODEL.md`](docs/ASSESSMENT_MODEL.md)）。QTIは思想のみ参考にし、native schemaとして取り込みません。対応表とlossyな箇所は[`docs/INTEROPERABILITY.md`](docs/INTEROPERABILITY.md)に記録しています。

## 設計文書

- [実装計画・フェーズと検証条件](docs/IMPLEMENTATION_PLAN.md)
- [アーキテクチャ・Core API境界](docs/ARCHITECTURE.md)
- [最小v1の範囲](docs/SPEC_V1.md)
- [Package形式の草案](docs/PACKAGE_FORMAT.md)
- [Desktopの起動・操作手順と制限](docs/DESKTOP.md)
- [設計判断・未確定事項](docs/DESIGN_DECISIONS.md)
- [AI支援の教材authoringと監査](docs/AI_AUTHORING.md)
- [Reference・Evidence・Authoring Provenance](docs/SOURCES_AND_PROVENANCE.md)
- [Authoring Workspace（.osmium/）](docs/AUTHORING_WORKSPACE.md)
- [Assessment modelと責務境界](docs/ASSESSMENT_MODEL.md)
- [QTI mappingとinteroperability](docs/INTEROPERABILITY.md)
- [authoring/lintの診断契約](docs/AUTHORING_LINT.md)
- [Skillsとauthoring workflow](docs/SKILLS.md)
- [医学Packageのauthoring review](docs/MEDICINE_PACKAGE_REVIEW.md)
- [将来の拡張](docs/ROADMAP.md)
- [一次資料：設計セッション全文](osmium_session_codex_context.md)

教材はMarkdown／YAML／JSON／assetsのファイルとして保存し、個人のLearning StateはSQLiteへ保存します。ConceptとLearning Objective、概念の前提関係とCurriculumの順序、回答の事実と観測されたperformance／progressをそれぞれ分けます。Mastery estimationは将来の課題です。

構成はRust Core／CLI、Tauri 2、React／TypeScript／Viteです。Rust依存はCargo.lockへ固定しています。Hubの実装・運用先は未決定です。

## 現在の開発用検証

現時点で動くCLI（リポジトリrootから）:

```sh
cargo run -p osmium-cli -- validate examples/arithmetic --json
cargo run -p osmium-cli -- lint examples/arithmetic --json
cargo run -p osmium-cli -- inspect examples/arithmetic
cargo run -p osmium-cli -- query examples/arithmetic objectives
cargo run -p osmium-cli -- context examples/arithmetic addition.basic
cargo run -p osmium-cli -- reference list examples/medicine-pressure-test --json
cargo run -p osmium-cli -- reference add examples/arithmetic --id example-ref --kind url \
  --title "Example" --locator "https://example.org/doc?id=1" --citation "Example. Doc." --json
cargo run -p osmium-cli -- reference attach examples/arithmetic example-ref --resource addition.lesson --json
cargo run -p osmium-cli -- build examples/arithmetic --output arithmetic.osmium
cargo run -p osmium-cli -- validate arithmetic.osmium --json
cargo run -p osmium-cli -- install arithmetic.osmium --json
cargo run -p osmium-cli -- packages --json
cargo run -p osmium-cli -- learn org.example/arithmetic
cargo run -p osmium-cli -- answer org.example/arithmetic addition.01 --response '"b"'
cargo run -p osmium-cli -- progress org.example/arithmetic
cargo run -p osmium-cli -- history org.example/arithmetic
cargo run -p osmium-cli -- rebuild-progress
cargo run -p osmium-cli -- export-state --output history.jsonl
cargo run -p osmium-cli -- backup-state --output state-backup.sqlite
cargo run -p osmium-cli -- init my-course --package-id org.example/my-course --language ja-JP
```

`reference add`／`attach`／`list` はSourceへ書き込む唯一のCLI操作です。`add`は既存IDに対して冪等で、不正なIDやcredential様locatorは書き込む前に拒否します。それ以外の編集は通常のファイル編集です。

通常の結果は単一JSONをstdoutへ返します。`lint`の警告は成功扱いです。`diagnostics`には安定した`code`、`severity`、`message`、対象が分かる場合の`entity_type`／`entity_id`を含み、順序は決定的です。編集→`validate --json`→`lint --json`→診断を読んで修正、というagent loopに利用できます。`--json`を受理し、help/versionはstderrへ表示します。buildの`--output`は保存先です（拡張子`.osmium`ならZIP、その他はdirectory）。他コマンドでは従来の`--output json`も受理します。未対応capabilityはexit 4、対象不正は1、引数不正は2、I/O失敗は3です。`init`と`build`は既存の出力を上書きしません。build出力先の親directoryは事前に作成してください。

Rust stable（開発確認環境1.98.1）とWindowsではMSVC build toolsが必要です。

Windowsの既定保存先は`%LOCALAPPDATA%\Osmium`です。`--home <dir>`または`OSMIUM_HOME`で変更できます。教材は`library/<digest>/`へ保存し、同一ID/version/digestの再導入は成功扱いです。同一ID/versionで内容だけが異なるPackageは拒否します。既存教材を上書きするオプションはありません。保存済みファイルの破損も検出し、無断で修復・置換しません。

履歴・導入metadata・進捗は同じ保存先の`state.sqlite`に保存します。回答の再送には同じ`--request-id <UUID>`を使うと二重記録を防げます。新しい回答では省略して新IDを生成できます。`--package-version`は複数versionを導入した場合に指定します。`learn`でResource IDを確認し、`read <package-id> <resource-id>`で本文を取得できます。進捗のaccuracyは観測された正答率で、理解度の判定ではありません。SQLiteバックアップには教材本文を含めないため、`library`も別途保持してください。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace

cd apps/desktop
npm run typecheck && npm run lint && npm test && npm run build
```

初回はCargo依存を取得します。取得後は `--offline --locked` をCargoのclippy/test/buildへ指定できます。教材Schema検証そのものは外部ネットワークを使いません。

[Schema](spec/v0.1/package.schema.json)と[適合性の説明](spec/v0.1/README.md)、[最小教材](examples/arithmetic/osmium.json)を用意しています。JSON/YAML manifestの読込み、重複キー・参照・循環・問題と正解の整合・非対応capabilityの検証を実装済みです。`osmium_package::load_source` はファイルのサイズ・個数・path・symlink/reparse pointを検証し、`.osmium/` をauthoring workspaceとして除外したうえで、参照された本文とmetadataを読み込みます。validate/inspect/query/context/lintは配布directoryとZIPも読み、hashとinventoryを照合します。

[authoring/lintの診断契約](docs/AUTHORING_LINT.md)と、構造・install・learning・progressの確認に使う[拡張教材](examples/arithmetic-expanded/osmium.json)も用意しています。

Schema 0.1を医学・数学・語学・プログラミングの[pressure-test fixtures](examples/)へ適用しています。これはschemaの安定化宣言ではなく、表現上の不足を見つける作業です。`examples/provenance-visibility-pressure-test` はReference可視性・locator可視性・package asset・再利用条件のpressure test専用であり、医学fixtureの都合で汚染しないよう分離しています。

## 開発の進め方

まずPackage仕様と検証可能なfixturesを作り、Core、CLI、安全なbuild/install、学習履歴、Desktopの順に実装します。各Phaseの完了条件と結果は実装計画へ記録します。意味のある単位で検証してコミットし、明示的な指示なしにpushしません。

リポジトリのライセンスは未選定です。オープンな設計方針だけから配布許諾を推定しないでください。公開前に選定し、Golden Packageと外部素材の権利情報は個別に管理します。
