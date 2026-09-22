# Osmium

Osmiumは、教材・概念・学習目標・問題・学習履歴をportableな形式で扱う、local-firstの学習Runtimeです。AIやHub、アカウントがなくても学習できることを基本にします。

現在はPhase 2bの教材作成・検証CLIまで実装済みです。学習アプリ、配布Packageのbuild/install、履歴保存は後続Phaseです。

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
cargo run -p osmium-cli -- init my-course --package-id org.example/my-course --language ja-JP
```

通常の結果は単一JSONをstdoutへ返します。`lint`の警告は成功扱いです。`--json`と`--output json`を受理し、help/versionはstderrへ表示します。未対応capabilityはexit 4、対象不正は1、引数不正は2、I/O失敗は3です。`init`は既存の教材ファイルを上書きしません。

Rust stable（開発確認環境1.98.1）とWindowsではMSVC build toolsが必要です。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

初回はCargo依存を取得します。取得後は `--offline --locked` をCargoのclippy/test/buildへ指定できます。教材Schema検証そのものは外部ネットワークを使いません。

[Schema](spec/v0.1/package.schema.json)と[適合性の説明](spec/v0.1/README.md)、[最小教材](examples/arithmetic/osmium.json)を用意しています。JSON/YAML manifestの読込み、重複キー・参照・循環・問題と正解の整合・非対応capabilityの検証を実装済みです。`osmium_package::load_source` はファイルのサイズ・個数・path・symlink/reparse pointを検証し、参照された本文とmetadataを読み込みます。ZIP/build/installは後続Phaseです。

## 開発の進め方

まずPackage仕様と検証可能なfixturesを作り、Core、CLI、安全なbuild/install、学習履歴、Desktopの順に実装します。各Phaseの完了条件と結果は実装計画へ記録します。意味のある単位で検証してコミットし、明示的な指示なしにpushしません。

リポジトリのライセンスは未選定です。オープンな設計方針だけから配布許諾を推定しないでください。公開前に選定し、Golden Packageと外部素材の権利情報は個別に管理します。
