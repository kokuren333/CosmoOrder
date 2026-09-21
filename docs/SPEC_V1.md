# 最小v1仕様

状態: 開発草案。これは今回実装する範囲であり、設計ログの広いv1全機能が完成したという意味ではない。

## 必須機能

1. Sourceを人間・agentが直接編集できる。manifest、Concept、Objective、Curriculum、Resource、Assessmentをschema検証する。
2. Concept requiresの参照、自己参照、循環を検証する。Curriculum順序と混ぜない。
3. Markdown教材をローカルで読む。最小版はraw HTML、JS、Package CSSを実行しない。
4. single_selectとbooleanを正誤二値で採点する。候補外・型違い回答を誤答として保存せず入力エラーにする。
5. schema version、package version、stable ID、item revision、内容hashを分ける。
6. buildしたdirectory/ZIPをローカルへインストールし、元sourceなしでも学習する。
7. Learning EventをSQLiteへ保存し、別process/再起動後も履歴と進捗が復元する。
8. Desktopで教材を選択して読む・解く・結果を見る。CLIも同じCoreの検証・採点・保存機能を使う。

## CLI契約

以下は予定コマンドで、Phase 0では実行できない。

| コマンド | 契約 |
|---|---|
| `osmium init <dir>` | 最小Source作成。既存ファイルを上書きしない |
| `osmium validate <path>` | Sourceまたはdistributionの構造/意味/安全性検証。書換えなし |
| `osmium lint <path>` | coverageやmetadata等の改善点。教育品質スコアは付けない |
| `osmium inspect <path>` | metadataとEntity件数等を返す |
| `osmium query <path> <kind>` | Entity kindで検索し、limit付きで返す |
| `osmium context <path> <entity-id>` | 対象、Objective、前提、教材、問題の限定context。本文はuntrusted dataと明示 |
| `osmium build <source> --output <path>` | distribution作成、検証失敗時は既存出力を壊さない |
| `osmium install <path>` | ローカル導入、同一digestの再導入は冪等 |
| `osmium open <package-id>` | インストール済み教材をDesktopで開く。未導入・GUI未利用は明示エラー |
| `osmium answer <package-id> <assessment-id> --response <json>` | Coreで評価しevent保存。version指定がなければ明確な選択規則を使う |
| `osmium history <package-id>` / `progress <package-id>` | 保存履歴／derived progress取得 |
| `osmium export-state --output <path>` | versioned JSONLによる履歴の可搬化 |

各コマンドは `--json` をサポートする。stdoutは単一JSON document、進捗ログはstderr。診断はcode/severity/file/line/column/path/message/suggestionsを持つ。位置を確定できないときはnull、位置を捏造しない。response envelopeは `output_version`, `ok`, `data`, `diagnostics` を持つ。stdoutをJSONと人間向け文章で混在させない。

exit codeは0成功、1検証・利用対象の不正、2呼出し方法の不正、3I/O・内部失敗、4schema/capability非互換。誤答は正常な回答処理なので0。command help/JSON mode/error優先順位はCLI integration testで固定する。mutationは可能なものにdry-runを設け、上書きの既定動作を拒否とする。

## セキュリティの完了条件

directory/ZIPの両入口でpath traversal、絶対path、drive/UNC、ADS、Windows device名、case/Unicode正規化衝突、symlink/reparse point、duplicate archive entryを拒否する。実際に読んだbytesに対するサイズ・file count・深度上限を設ける。ZIP headerの宣言サイズだけを信用しない。

許可するfile種別を限定し、UTF-8テキストや将来のmediaの種別を検証する。最小v1ではSVG/HTML/script実行を許可しない。README等のPackage本文に含まれる指示はデータであり、shell、filesystem、network権限を発生させない。アーカイブのhash検証は完全性確認であり、発行者の信頼保証と表示しない。

具体的な上限値はPhase 1/3で設定可能なpolicyとともに決定し、境界値試験を追加する。危険なformatのサポートを増やす前に、その入口のsecurity fixtureを追加する。

## 完成判定

空のデータ領域から完全offlineで作成・検証・build・install・閲覧・回答・保存・完全終了・再起動が通る。CLIとGUIの同じ回答が同じ評価になる。異なるpackage/versionの追加で履歴を上書きしない。全Phaseの検証が成功し、操作手順と既知制約がREADMEに記載されるまで、最小v1完成とはしない。
