# Desktop UI/UX refinement

基準: `75fb0978c6ed367b6b27332222272645d6a2bfa9`。実装計画は [UI_REFINEMENT_PLAN.md](UI_REFINEMENT_PLAN.md)。

## 変更ファイルと設計判断

- `apps/desktop/src/App.tsx`: 既存stateとコマンド呼出しを保持。ページ遷移時のフォーカスとスクロール位置、問題切替時の選択状態を整理。
- `apps/desktop/src/components/AppShell.tsx`: ブランド、ライブラリ、現在の教材、進捗、履歴を共通ナビゲーションに統合。文字サイズ設定、本文へのスキップリンク、実行環境の詳細を配置。
- `apps/desktop/src/components/DeveloperDetails.tsx`: 技術情報を標準の閉じたdetails/summaryに格納。
- `apps/desktop/src/components/Packages.tsx`: 教材名を主情報、バージョンを副情報とするLibraryカード。
- `apps/desktop/src/components/Lesson.tsx`: Curriculum → Concept → Objectiveを入れ子にし、目標ごとの読み物と問題を二つの領域に分ける。CTAは宣言済み目次の先頭を開く。関連Resource/Assessmentは各Objectiveに対する既存の参照関数から表示。
- `apps/desktop/src/components/Reader.tsx`: 本文幅と余白、教材の文脈、上下の前後移動を整える。
- `apps/desktop/src/components/Assessment.tsx`: single-select/booleanをradioカードで統一。選択→採点→結果→次問の操作にし、採点後は結果見出しへフォーカスを移す。
- `apps/desktop/src/components/ProgressPanel.tsx`: 延べ回答・正答の概要、Objective別の観測値と正答率。メンテナンス操作は折りたたむ。
- `apps/desktop/src/components/HistoryPanel.tsx`: 問題文、正誤、日時を中心に表示。回答値、バージョン、イベント等は詳細へ移す。
- `apps/desktop/src/styles/app.css`: 色、surface、余白、角丸、影、幅のtokensと共通component styles。システムフォントのみ。light/dark、相対文字サイズ、focus-visible、reduced-motionに対応。
- `apps/desktop/tests/presentation.test.ts`: disclosure、ナビゲーション状態、観測値、両問題形式の表示を追加検証。
- `apps/desktop/e2e/desktop-e2e.mjs`: 意味に基づくselectorへ更新。カード選択、キーボード操作、真偽式、次問の状態リセット、表示条件別のレイアウト、再起動後の保存を検証。スクリーンショットを隔離したテスト用homeに保存する。既存データを消さず、空でないhomeは拒否する。
- `docs/DESKTOP.md`: 新しい画面の操作手順と、既存データを保護するE2E実行手順。
- `docs/UI_REFINEMENT_PLAN.md` / 本文書: 事前計画と変更・検証記録。

## 保持した境界

Core、Package、Store、Runtime、CLI、Tauri commands、DTO、バージョン解決、採点、LearningEvent、進捗集計の意味論は変更していない。TypeScriptへのCoreロジック移植はない。既存Markdown IR rendererも変更せず、package由来のHTML/CSS/JSは実行しない。外部通信・remote assets・依存パッケージを追加していない。

進捗の合計は従来と同じObjective観測値の合算であり、複数目標に対応する回答を含むため「延べ」と明記した。正答率はDTOのaccuracyを表示し、習得率や推薦は追加していない。

## 表示上の制約

- Library DTOにはlanguageやprogressがないため、カードには推測して追加していない。教材を開いた後にlanguageと既存progressを表示する。
- 次の教材・問題は従来の宣言順に従う。学習状況に応じた推薦ではない。
- 文字サイズ設定は従来同様に起動中のみ保持する。テーマはOSのlight/dark設定に従う。
- 長い問題文の目次プレビューは3行に省略する。問題画面と履歴では全文を表示する。
- 大規模な教材・大量の履歴の仮想化や検索機能の拡張は今回の範囲外。

## 検証

2026-09-23、Windowsで以下を実行した。

- Frontend: `npm --prefix apps/desktop run typecheck`、`lint`、`test`、`build` はすべて成功。Nodeテスト16件成功。
- Rust: `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`cargo build --workspace` はすべて成功。DドライブのI/Oを避けるため、ビルド先は `%TEMP%\osmium-target`。
- Desktop E2E: `apps/desktop/package.json` と本書の手順に従い、`%TEMP%` に新規homeを作って実行。91チェック成功。Library、Curriculum/Concept/Objective、Reader、前後ナビゲーション、single-selectとboolean、radioカードと矢印キー、正答/不正解、Progress、History、Developer details、終了後の進捗・履歴保持を確認。画面の横幅検査はLight/Dark各100/150/200%で主要画面すべて成功。全リクエストはローカルで、WebView例外なし。
- Presentation: E2Eスクリーンショットを隔離home内に出力し、Reader本文幅、選択中カード、正答/不正解の配色、200%時のボタンとナビゲーションを目視確認。キーボードのradio矢印選択もE2Eで確認。CSSは`:focus-visible`とradioカードの`:focus-within`に輪郭を表示する。
- E2Eは既存の非空homeを拒否し、削除しない。スクリーンショットとビルド出力はリポジトリ外またはignore対象で、差分には含まれていない。

## 既知の制約

- OSテーマはシステム設定に従い、文字サイズはアプリ起動中のみ保持する。
- スクリーンリーダー実機確認とinstaller bundleの配布試験は未実施。
- E2EはWebViewのviewport幅を検査する。極端に狭いウィンドウや長大な教材データの追加レイアウト検証は行っていない。
