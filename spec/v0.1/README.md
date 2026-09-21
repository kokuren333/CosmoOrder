# 開発形式0.1の構造契約

`package.schema.json`はJSON Schema 2020-12の自己完結bundleです。rootはmanifest、各 `$defs` にconcept/objective/curriculum/resource/assessment、複数形のEntity配列、learning_eventを定義します。外部 `$ref` はありません。Rustを使わないvalidatorでも、同じbundleのroot `$ref` を対象定義に切り替えて利用できます。

この形式は開発版です。Distributionのintegrity metadataはPhase 3で別schemaへ追加します。現時点のmanifestはSource用であり、ZIPの信頼性や学習Eventの真正性を保証しません。

## 文法と上限

- Entity IDは小文字ASCIIから始まる英数字と `.`, `_`, `-` の区切り（連続区切り不可）、最大128文字。
- package IDは `org.example/name` 形式、最大200文字。package versionは非負整数の `major.minor.patch`（先頭0不可）。prerelease/build suffixは開発0.1では未対応。
- revisionは1以上の十進整数を文字列で表す。最大32文字。
- extension/capability IDは `org.example.feature.v1` のような名前空間付きversion ID、最大200文字。
- extensionsは最大64項目、値は未知のJSON値も保持。core objectの未知fieldは拒否。
- Entity配列は最大10,000件、参照配列は重複不可、選択問題の候補は2〜100個。
- 非空文字列本文は最大16,384文字。Resource本文ファイルのbyte上限は後のloader policyで定める。
- pathは最大240文字、slashで区切る。絶対path、backslash、colon、制御文字は構造で拒否。dot segment、device名、symlink等はloaderの意味検証が必要。
- languageはBCP 47を意図する文字列。現在のSchemaは長さだけを制約する。BCP 47構文はPhase 2で検証する。
- EventのUUID/date-timeはformat validation有効で検証する。サンプルのdigestは形状説明用の値で、実データとの整合はStore実装時の責務。

## 構造検証と意味検証

single_selectの正解は文字列、booleanの正解はboolean。未知response形式は拒否します。候補IDの一意性、正解の候補内存在、Entityのstable ID重複、参照先存在、graphの循環、capability対応はCoreの意味検証で保証します。Event snapshotとの整合はStore導入時に追加します。schema-validだけでインストール可能と判定してはいけません。

`parsing::parse_json` は入力metadataを4 MiBまでに制限し、入れ子やUnicode escapeを使った重複キー、余分なJSON document、不正UTF-8、過剰な深度を拒否します。JSON syntax診断には実際のline/columnを付けます。`validation::validate_package` は構造検証後に意味検証し、未知required capabilityを `OSM_CAPABILITY`、非対応schema版を `OSM_SCHEMA_VERSION`、参照切れを `OSM_REFERENCE`、循環を `OSM_CYCLE` として報告します。意味診断のfileは現時点では文書種別名であり、loaderで実ファイル名へ対応づけます。

pathの字句検証ではdot segment、Windows device名、末尾dot/space等も拒否します。ただし実filesystemのsymlink/reparse pointやUnicode正規化衝突、参照fileの存在はまだ検証していません。追加capabilityの実装はないため、requiredはすべて非対応として明示拒否します。optionalの宣言とpayloadは保持します。

`osmium-core::schema::validate_document`は入力を変更せず診断を返します。schemaは埋め込み済みで、Packageからschemaを渡すAPIはありません。jsonschema依存のHTTP/file解決機能も無効化しています。診断は最大100件、位置はJSON Pointerです。元ファイル位置を取得していないためline/columnはnullです。

## Fixtures

- `examples/arithmetic/`: 自作の日本語教材、全5種類のEntity、単一選択と真偽問題。
- `fixtures/valid/learning-event.json`: Package外に保存する履歴の形状例。
- `fixtures/invalid/`: 回答型不一致、未知core field、必須参照field欠落。
- `crates/osmium-core/tests/schema.rs`: 全定義のmeta-schema適合、offline compilation、良/不正fixture、version拒否、拡張値のroundtripとID制約を検証。
