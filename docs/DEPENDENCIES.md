# 開発依存の選定記録

2026-09-22、Phase 1。全依存解決結果はCargo.lockを正本にする。リポジトリ自身の公開ライセンスとは別の記録。

| 直接依存 | 確認したversion | ライセンス | 用途 |
|---|---|---|---|
| serde | 1.0.229 | MIT OR Apache-2.0 | 構造化した診断の入出力 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | JSONの値とSchemaを扱う |
| jsonschema | 0.56.0 | MIT | JSON Schema 2020-12による検証 |

crates.ioのmetadataと公式API資料を確認して選定。jsonschemaはdefault featuresを無効化し、HTTP/ファイル参照の自動取得を含めない。Packageから任意schemaを受理せず、固定bundle内のfragmentのみ参照する。serde_jsonのunbounded_depthは有効化しない。

依存の利用可能性とnativeビルドの検証は、脆弱性がないことの証明ではない。アーカイブ/YAML/parser導入時には入力制限と悪意あるfixtureも追加する。Phase 1はfilesystem loaderを提供しないので、schemaがvalidという理由で外部教材を実行しない。

参照: [jsonschema公式Rust API](https://docs.rs/jsonschema/0.56.0/jsonschema/)、[serde_json公式Rust API](https://docs.rs/serde_json/1.0.151/serde_json/)。
