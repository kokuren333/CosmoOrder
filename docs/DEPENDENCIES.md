# 開発依存の選定記録

2026-09-22、Phase 1。全依存解決結果はCargo.lockを正本にする。リポジトリ自身の公開ライセンスとは別の記録。

| 直接依存 | 確認したversion | ライセンス | 用途 |
|---|---|---|---|
| serde | 1.0.229 | MIT OR Apache-2.0 | 構造化した診断の入出力 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | JSONの値とSchemaを扱う |
| jsonschema | 0.56.0 | MIT | JSON Schema 2020-12による検証 |
| yaml-rust2 | 0.13.0 | MIT OR Apache-2.0 | 制限付きevent parserでJSON値に変換、default features無効 |
| language-tags | 0.3.2 | MIT OR Apache-2.0 | BCP 47構文、ネット照合なし |
| unicode-normalization | 0.1.25 | MIT OR Apache-2.0 | source pathのNFC衝突検出 |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | filesystem試験とbuildの隔離staging |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | payload・manifest・archiveのSHA-256 |
| zip | 6.0.0 | MIT | 配布archive、Stored/Deflateのみ有効 |
| fs2 | 0.4.3 | MIT OR Apache-2.0 | Windows/Unixのプロセス間advisory file lock |
| rusqlite | 0.40.2 | MIT | SQLiteのbundled buildとbackup API、default features無効 |
| uuid | 1.26.1 | MIT OR Apache-2.0 | Event/device/request ID、v4生成 |
| time | 0.3.45 | MIT OR Apache-2.0 | UTC時刻のRFC 3339表現 |

crates.ioのmetadataと公式API資料を確認して選定。jsonschemaはdefault featuresを無効化し、HTTP/ファイル参照の自動取得を含めない。Packageから任意schemaを受理せず、固定bundle内のfragmentのみ参照する。serde_jsonのunbounded_depthは有効化しない。

依存の利用可能性とnativeビルドの検証は、脆弱性がないことの証明ではない。アーカイブ/YAML/parser導入時には入力制限と悪意あるfixtureも追加する。Phase 1はfilesystem loaderを提供しないので、schemaがvalidという理由で外部教材を実行しない。

参照: [jsonschema公式Rust API](https://docs.rs/jsonschema/0.56.0/jsonschema/)、[serde_json公式Rust API](https://docs.rs/serde_json/1.0.151/serde_json/)。

Phase 2aの依存はcrates.io metadataおよび取得した公式crate sourceで確認した。YAMLは高水準loaderによるalias展開を使わず、event段階で拒否する。yaml-rust2のMarker列は実際には0始まりであり、Osmiumの1始まりへ補正する位置テストを追加した。

Phase 3aではsha2/zipのcrates.io metadataと取得したcrate sourceを確認。ZIPのdefault featuresを無効化し、暗号化/特殊file/ZIP64等を入力境界で拒否する。serde_jsonはcanonical profileの参照版として1.0.151に固定し、binary64値の読込み・再出力で値が変わらないよう`float_roundtrip`を有効化する。
