# Desktop presentation refinement

基準: `75fb0978c6ed367b6b27332222272645d6a2bfa9`。2026-09-23。

- Appは既存のstateとTauri呼出しを保持し、AppShellへbrand・ライブラリ・教材・進捗・履歴のnavigationと文字サイズ設定を分離する。
- Libraryはタイトルとversionを中心としたカード。DTOにないlanguage/進捗を推測しない。内部metadataは閉じたDeveloperDetailsへ置く。
- Lessonは既存outlineに沿ってCurriculum → Concept → Objectiveを入れ子にし、各Objectiveに宣言済みResource/Assessmentを関連づける。先頭教材へのCTAは目次順であることを示し、推薦・習得判定を追加しない。
- Readerは本文幅、余白、前後移動、目次への文脈を整える。Assessmentはクリック可能なradioカードと採点→feedback→次問を中心にする。booleanも選択してから採点する同じ操作に統一し、送信値と評価は変えない。
- Progressは既存Objective観測値をカードで表示する。目標間の合計は延べ件数と明記し、全体の習得率を作らない。Historyは問題文・正誤・時刻を中心に、raw値をdetailsへ移す。
- 色・余白・幅・角丸のtokensを持つローカルCSS。light/dark、100/150/200%、focus、reduced-motionを確認する。
- Core/Package/Store/Runtime/CLI、Tauri commands、DTOとMarkdownの安全なIR rendererは変更しない。外部通信・依存assetsを追加しない。
- 既存frontend/Rust checks、Desktop E2Eを実行。UI disclosure/navigation/options/progressの回帰テストを追加し、実画面も確認する。
