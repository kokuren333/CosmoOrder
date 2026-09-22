# Design Decisions

2026-09-22。以下は今回の最小実装に向けた判断。根本原則の変更は行わない。公開形式の確定とは区別する。

## DD-001: 最小v1と広いv1案

- 問題: ログはHub/MCP/media/CSS/8種問題もv1と呼ぶが、今回の指示は最小縦切りを優先する。
- 選択肢: 全機能同時実装／ローカルの最小v1を先に完成させる。
- 採用案: 最小v1を独立した完成条件とし、広い案を後続拡張へ移す。
- 理由: 現行のユーザー指示とCore優先の思想に一致する。
- 将来への影響: HubやMCPを撤回しない。未対応のcapabilityは明示拒否し、後続機能を対応済みに見せない。

## DD-002: ObjectiveとEventを中心にする

- 問題: 初期の例はConceptへ問題・masteryを直接結びつけている。
- 選択肢: 旧例を継承／最終レビューの別Entityモデルを採用。
- 採用案: Resource teaches Objective、Assessment measures Objective。ConceptとCurriculumも分離。
- 理由: 理解対象と達成目標、論理的前提と教材順序は異なる。
- 将来への影響: adaptive curriculumや評価器交換が既存観測を破壊しない。masteryだけを正本として保存しない。

## DD-003: 草案versionと互換性保証

- 問題: schema 0.1/1.xの例と10年以上の保持目標があり、実装前に1.0を保証できない。
- 選択肢: 即1.0固定／開発0.1から適合試験を経てfreeze。
- 採用案: 初期は0.1の明示版、未知schemaは拒否。stable化はPhase 6で判断。
- 理由: 固まっていない例を長期互換の契約にしない。
- 将来への影響: 旧fixtures/schemaを残し、破壊的変更にはmigration pathを用意する。現時点では10年保証を達成済みと表現しない。

## DD-004: 未知値と安全性

- 問題: unknown fieldの保存要求とschemaによる誤記検出が衝突しうる。
- 選択肢: 全field自由／全未知値拒否／厳格core＋namespaced extensions。
- 採用案: coreは厳格、optional extensionのJSON payloadを保存。unknown required capabilityは拒否。
- 理由: typoと将来拡張を区別できる。
- 将来への影響: 新しい意味論はcapability/versionで宣言する。optional fallbackの保存は実行許可ではない。

## DD-005: 最初の表示・問題型

- 問題: media、数式、scoped CSS、HTML、8種responseを一度に作ると検証範囲が広がる。
- 選択肢: 全範囲を最初から実装／Markdownと2種responseで縦切りを検証。
- 採用案: Markdown（HTML不活性）、single_select/boolean、標準themeから開始。Markdownは保持しIRは再生成する。
- 理由: 最小の学習体験と安全なCore境界を先に検証できる。
- 将来への影響: TeX/media/CSS等はResource/Renderer/capabilityへ追加。ReactやIR実装の都合をPackage正本に埋め込まない。

## DD-006: 実装言語と保存境界

- 問題: 新規repositoryに技術設定がない。TypeScript単独も可能。
- 選択肢: Rust Core＋Tauri／TypeScript単独／Python等。
- 採用案: ログのRust Core/CLI、Tauri 2＋React、SQLiteを継承。
- 理由: 既存の設計方針を変える必然性がなく、GUIと意味論を切り離せる。環境にはRust/Nodeがある。
- 将来への影響: Coreの純粋部分をI/Oから分離してWASMの余地を残す。native prerequisite失敗を理由に黙って別stackへ変更しない。

## DD-007: 開発用canonical profile

- 問題: 内容digestを安定させる必要がある一方、未知extensionsに任意のJSON数値が入り、汎用JSON serializerをRFC 8785準拠とは呼べない。
- 選択肢: 外部JCS実装を追加／全小数を拒否／名前とversionのある開発profileを固定する。
- 採用案: `osmium-json-0.1`を定義し、key順・LF・UTF-8と参照serializer、数値の範囲を明記する。Package digestとZIP digestを分離する。
- 理由: 現在の入力モデルを維持し、byte変更を互換性境界として検出できる。未知extensionのJSON値は保持し、Sourceの記法保持とは区別する。
- 将来への影響: stable 1.0前に別言語実装と数値適合vectorを拡充する。異なるcanonical profileを無断で同じdigest体系として扱わない。

## 未確定の判断

Phase 4でDB migrationとprojection規則、Phase 5でIPC/CSPと実機accessibilityを確定する。canonical profileの独立実装適合、公開ライセンス、署名・信頼、fork履歴移行、同期の競合規則は別途決める。将来機能のためだけに汎用plugin loaderを作らない。
