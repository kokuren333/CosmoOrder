# Osmium

Osmiumは、教材・概念・学習目標・問題・学習履歴をportableな形式で扱う、local-firstの学習Runtimeです。AIやHub、アカウントがなくても学習できることを基本にします。

現在はPhase 2のCore読込み・意味検証を実装しています。実行可能なアプリやCLIはまだありません。

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

Rust stable（開発確認環境1.98.1）とWindowsではMSVC build toolsが必要です。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

初回はCargo依存を取得します。取得後は `--offline --locked` をCargoのclippy/test/buildへ指定できます。教材Schema検証そのものは外部ネットワークを使いません。

[Schema](spec/v0.1/package.schema.json)と[適合性の説明](spec/v0.1/README.md)、[最小教材](examples/arithmetic/osmium.json)を用意しています。構造検証に加え、JSON重複キー・参照・循環・問題と正解の整合・非対応capabilityの検証を実装済みです。安全なfilesystem読込みとYAML入力は引き続きPhase 2で追加します。

## 開発の進め方

まずPackage仕様と検証可能なfixturesを作り、Core、CLI、安全なbuild/install、学習履歴、Desktopの順に実装します。各Phaseの完了条件と結果は実装計画へ記録します。意味のある単位で検証してコミットし、明示的な指示なしにpushしません。

リポジトリのライセンスは未選定です。オープンな設計方針だけから配布許諾を推定しないでください。公開前に選定し、Golden Packageと外部素材の権利情報は個別に管理します。
