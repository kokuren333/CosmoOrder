# Osmium

Osmiumは、教材・概念・学習目標・問題・学習履歴をportableな形式で扱う、local-firstの学習Runtimeです。AIやHub、アカウントがなくても学習できることを基本にします。

現在は教材作成・配布・install・学習・履歴保存のCLIを実装済みです。Desktop UIと最終受入れ検証を作業中です。

## 設計文書

- [実装計画・フェーズと検証条件](docs/IMPLEMENTATION_PLAN.md)
- [アーキテクチャ・Core API境界](docs/ARCHITECTURE.md)
- [最小v1の範囲](docs/SPEC_V1.md)
- [Package形式の草案](docs/PACKAGE_FORMAT.md)
- [設計判断・未確定事項](docs/DESIGN_DECISIONS.md)
- [将来の拡張](docs/ROADMAP.md)
- [一次資料：設計セッション全文](osmium_session_codex_context.md)

教材はMarkdown／YAML／JSON／assetsのファイルとして保存し、個人のLearning StateはSQLiteへ保存します。ConceptとLearningObjective、概念の前提関係とCurriculumの順序、回答の事実と推定習熟度をそれぞれ分けます。

推奨構成はRust Core／CLI、Tauri 2、React／TypeScript／Viteです。Rust依存はCargo.lockへ固定しています。Hubは将来Cloudflareで構築する独立した配布サービスです。

## 現在の開発用検証

現時点で動くCLI（リポジトリrootから）:

```sh
cargo run -p osmium-cli -- validate examples/arithmetic --json
cargo run -p osmium-cli -- lint examples/arithmetic --json
cargo run -p osmium-cli -- inspect examples/arithmetic
cargo run -p osmium-cli -- query examples/arithmetic objectives
cargo run -p osmium-cli -- context examples/arithmetic addition.basic
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

通常の結果は単一JSONをstdoutへ返します。`lint`の警告は成功扱いです。`--json`を受理し、help/versionはstderrへ表示します。buildの`--output`は保存先です（拡張子`.osmium`ならZIP、その他はdirectory）。他コマンドでは従来の`--output json`も受理します。未対応capabilityはexit 4、対象不正は1、引数不正は2、I/O失敗は3です。`init`と`build`は既存の出力を上書きしません。build出力先の親directoryは事前に作成してください。

Rust stable（開発確認環境1.98.1）とWindowsではMSVC build toolsが必要です。

Windowsの既定保存先は`%LOCALAPPDATA%\Osmium`です。`--home <dir>`または`OSMIUM_HOME`で変更できます。教材は`library/<digest>/`へ保存し、同一ID/version/digestの再導入は成功扱いです。同一ID/versionで内容だけが異なるPackageは拒否します。既存教材を上書きするオプションはありません。保存済みファイルの破損も検出し、無断で修復・置換しません。

履歴・導入metadata・進捗は同じ保存先の`state.sqlite`に保存します。回答の再送には同じ`--request-id <UUID>`を使うと二重記録を防げます。新しい回答では省略して新IDを生成できます。`--package-version`は複数versionを導入した場合に指定します。`learn`でResource IDを確認し、`read <package-id> <resource-id>`で本文を取得できます。進捗のaccuracyは観測された正答率であり、習得の保証ではありません。SQLiteバックアップには教材本文を含めないため、`library`も別途保持してください。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

初回はCargo依存を取得します。取得後は `--offline --locked` をCargoのclippy/test/buildへ指定できます。教材Schema検証そのものは外部ネットワークを使いません。

[Schema](spec/v0.1/package.schema.json)と[適合性の説明](spec/v0.1/README.md)、[最小教材](examples/arithmetic/osmium.json)を用意しています。JSON/YAML manifestの読込み、重複キー・参照・循環・問題と正解の整合・非対応capabilityの検証を実装済みです。`osmium_package::load_source` はファイルのサイズ・個数・path・symlink/reparse pointを検証し、参照された本文とmetadataを読み込みます。validate/inspect/query/context/lintは配布directoryとZIPも読み、hashとinventoryを照合します。

## 開発の進め方

まずPackage仕様と検証可能なfixturesを作り、Core、CLI、安全なbuild/install、学習履歴、Desktopの順に実装します。各Phaseの完了条件と結果は実装計画へ記録します。意味のある単位で検証してコミットし、明示的な指示なしにpushしません。

リポジトリのライセンスは未選定です。オープンな設計方針だけから配布許諾を推定しないでください。公開前に選定し、Golden Packageと外部素材の権利情報は個別に管理します。
