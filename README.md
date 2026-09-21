# Osmium

Osmiumは、教材・概念・学習目標・問題・学習履歴をportableな形式で扱う、local-firstの学習Runtimeです。AIやHub、アカウントがなくても学習できることを基本にします。

現在は設計・実装計画の段階です。実行可能なアプリやCLIはまだありません。

## 設計文書

- [実装計画・フェーズと検証条件](docs/IMPLEMENTATION_PLAN.md)
- [アーキテクチャ・Core API境界](docs/ARCHITECTURE.md)
- [最小v1の範囲](docs/SPEC_V1.md)
- [Package形式の草案](docs/PACKAGE_FORMAT.md)
- [設計判断・未確定事項](docs/DESIGN_DECISIONS.md)
- [将来の拡張](docs/ROADMAP.md)
- [一次資料：設計セッション全文](osmium_session_codex_context.md)

教材はMarkdown／YAML／JSON／assetsのファイルとして保存し、個人のLearning StateはSQLiteへ保存します。ConceptとLearningObjective、概念の前提関係とCurriculumの順序、回答の事実と推定習熟度をそれぞれ分けます。

推奨構成はRust Core／CLI、Tauri 2、React／TypeScript／Viteです。バージョン固定と実行手順はPhase 1以降に追加します。Hubは将来Cloudflareで構築する独立した配布サービスです。

## 開発の進め方

まずPackage仕様と検証可能なfixturesを作り、Core、CLI、安全なbuild/install、学習履歴、Desktopの順に実装します。各Phaseの完了条件と結果は実装計画へ記録します。意味のある単位で検証してコミットし、明示的な指示なしにpushしません。

リポジトリのライセンスは未選定です。オープンな設計方針だけから配布許諾を推定しないでください。公開前に選定し、Golden Packageと外部素材の権利情報は個別に管理します。
