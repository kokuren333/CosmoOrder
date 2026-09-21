# Osmium Architecture

状態: 最小v1の実装前設計。公開API保証ではない。

## 責務と依存方向

```text
CLI ───────────┐
Desktop IPC ───┼── Application operations ── Core
future MCP ────┘             │                │
                            ├── Package I/O  └── pure domain model
                            └── SQLite Store

future Hub ── Registry HTTP adapter ── Package I/O
```

`osmium-core`は概念モデル、参照検証、graph、assessment、Markdownからcontent IRへの変換を持つ。filesystem、SQLite、Tauri、HTTP、Cloudflareへの依存を持たない。Package I/Oはサイズ制限の下で読み込んだファイルと診断元情報をCoreへ渡す。Storeはeventsを永続化する。CLI/Desktopの採点や進捗を別実装しない。

Application operationsはまずモジュール境界とし、必要性が出るまで汎用plugin基盤や新しいcrateを増やさない。WASM対応は将来の適合試験事項であり、現段階で対応済みとはしない。

## Entityと参照

- Package: package ID、作品version、schema version、capabilities、内容一覧。
- Concept: 学ぶ対象。安定IDとtitle、requiresを持つ。個人の習熟度を持たない。
- LearningObjective: 何ができるか。Conceptを参照する。
- Curriculum: Objectiveを選択し推奨順序を持つ。Conceptのrequiresを変更しない。
- Resource: `teaches`でObjectiveを参照し、本文・素材・権利情報を記述。
- Assessment: `measures`でObjectiveを参照し、Stimulus/Response/Evaluation/Feedbackを分離。
- LearningEvent: 実際に提示された問題、回答、評価、時刻を記録。

参照はpackage ID + entity kind + entity IDで修飾する。ファイル名変更ではIDを変えない。Packageのversionが変わっても古いeventを更新しない。型の異なるID参照を拒否する。

## Core API境界（意味契約、signatureはPhase 1で確定）

| 操作 | 入出力と責務 |
|---|---|
| parse_source | 制限済みファイル集合 → model + source map + diagnostics |
| validate | model + supported capabilities → schema/refs/graph/assessment診断 |
| inspect/query/context | validated model + typed filter + 出力量制限 → semantic DTO |
| compile_content | Markdown → renderer非依存IR、raw HTMLは不活性なtextとして扱う |
| evaluate | immutable item snapshot + validated response → score/feedback/evaluator version |
| project_progress | events + model version → Objective単位の再構築可能な進捗 |
| build（I/O層） | validated source → normalized files + integrity metadata |
| install（I/O層） | local distribution + trusted library root → immutable install record |
| submit_attempt（application） | item identity + response + request ID → atomic event + projection |
| export_state（store） | event log → versioned JSONL（バックアップ時は整合したsnapshot） |

Packageが指定するpathを直接OS APIに渡さない。GUIは受理済みpackage/entity IDを指定し、任意pathをIPC経由で読み出せない。問題のcorrect answerはローカル教材に含むため、試験の不正防止基盤とは位置づけない。

## ローカル保存と回復

OS標準のユーザーデータ領域を既定とし、CLI/テストでは `OSMIUM_HOME` で切替可能にする。リポジトリ配下に利用者の実データを作らない。

```text
<data-root>/
  library/<package-digest>/    # 検証済みのfiles、内容はimmutable
  staging/                    # 隔離された導入途中の内容
  state.sqlite                # learning events + installed index + projections
```

filesystemとDBは一つのtransactionにできない。stagingで検証を完了し、同じfilesystem内でrenameしてからDB登録する。起動時に孤立した導入物を検出・再検証して復旧し、導入失敗で既存教材を消さない。Source編集中の変更を読む危険には、stagingへコピーした内容を再検証・hashすることで対処する。

eventsはUUIDのevent ID、device ID、schema versionを持ち、追記のみ。request IDを使い送信retryを二重学習と数えない。別の意図的な回答には新しいrequest IDを使う。SQLite transactionでeventとprojectionを更新し、失敗時に半端な記録を残さない。

projectionは試行数、正答数、最終回答時刻等から開始する。客観的な「習得保証」と表示しない。schema migrationは番号付きとし、migration前backup、rollback時の挙動、将来のschemaを開いた際の書込み拒否を試験する。DB破損時に空DBへ黙って置換しない。

## Renderingと権限

CommonMark本文とmetadataを保持し、content IRは再生成できる中間表現にする。React component、MDX、生成HTMLはPackage semanticsにしない。Desktopの固定されたUIだけがIPCを使用でき、教材文字列はHTML/JSとして実行しない。外部URLは表示だけを基本とし、自動fetchしない。

最小v1は標準themeのみ。semantic headings、フォームlabels、keyboard操作、focus、文字拡大、contrast、reduced motionをRuntimeの責任とする。将来の数式はTeXを正本とし、mediaはResourceProvider、scoped CSSは独立した検証を通すpresentation enhancementにする。

## 将来の接続点

Evaluator、ResourceProvider、Renderer、MasteryEngine、CurriculumEngine、Registry、Importer/Exporterは責務として分ける。保存形式に必要なextensions/capabilitiesだけ最初から作り、動的コードloadは実装しない。AIやMCPは外部クライアント、同期はevent identityを利用する別サービス、公式HubはCloudflare実装である。Hubへ学習履歴を送る経路を最小v1に作らない。
