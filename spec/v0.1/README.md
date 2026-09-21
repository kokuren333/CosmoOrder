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
- languageはBCP 47を意図する文字列。Schemaは長さを、Coreの意味検証はBCP 47構文を検証する。登録済み言語かどうかのネット照合はしない。
- EventのUUID/date-timeはformat validation有効で検証する。サンプルのdigestは形状説明用の値で、実データとの整合はStore実装時の責務。

## 構造検証と意味検証

single_selectの正解は文字列、booleanの正解はboolean。未知response形式は拒否します。候補IDの一意性、正解の候補内存在、Entityのstable ID重複、参照先存在、graphの循環、capability対応はCoreの意味検証で保証します。Event snapshotとの整合はStore導入時に追加します。schema-validだけでインストール可能と判定してはいけません。

`parsing::parse_json` は入力metadataを4 MiBまでに制限し、入れ子やUnicode escapeを使った重複キー、余分なJSON document、不正UTF-8、過剰な深度を拒否します。JSON syntax診断には実際のline/columnを付けます。`validation::validate_package` は構造検証後に意味検証し、未知required capabilityを `OSM_CAPABILITY`、非対応schema版を `OSM_SCHEMA_VERSION`、参照切れを `OSM_REFERENCE`、循環を `OSM_CYCLE` として報告します。意味診断のfileは現時点では文書種別名であり、loaderで実ファイル名へ対応づけます。

pathの字句検証ではdot segment、Windows device名、末尾dot/space等も拒否します。`osmium-package`は実filesystemのsymlink/reparse point、NFC＋小文字化で同じになるpath、参照fileの欠落・表記不一致も拒否します。追加capabilityの実装はないため、requiredはすべて非対応として明示拒否します。optionalの宣言とpayloadは保持します。

## Sourceの読込み契約

`osmium.json`か`osmium.yaml`のどちらか1つをSource rootへ置く。EntityファイルはJSON、Resource本文はUTF-8の `.md`。ルートの `.git` は走査・同梱せず、他のnotesは安全性・サイズ検査のみ行い、参照されなければLoadedSourceに含めない。

上限はrootの `.git` を除いて4,096 entries（directory含む）、合計64 MiB、深度32、読込み対象の各fileは4 MiB。metadata上のサイズだけでなく実際の読込みbytesにも制限を適用する。元のSourceは変更しない。

YAMLはUTF-8・単一document・深度64以下。anchor、alias、tag（標準tagを含む）、merge key、重複key、非文字列keyを拒否。引用文字列とblock scalarは文字列のまま保持。plain scalarのうちJSON構文のnull/boolean/numberだけをその型に変換し、それ以外は文字列。空scalarはnull。schema_version/revisionのような文字列fieldは引用すること。`yes`や日付を暗黙にbool/dateへ変換しない。独自YAML objectの実行は行わない。

filesystem検査は静的な教材ディレクトリを対象とする。読込み途中のサイズ/path変更は検出するが、別processが親directoryを継続的に差し替える競合の完全防御はまだ保証しない。build/installでは隔離stagingの内容を再検証する必要があり、Phase 3で実装する。Unixでの検証実行は未確認、現時点の実機試験はWindows。

`osmium-core::schema::validate_document`は入力を変更せず診断を返します。schemaは埋め込み済みで、Packageからschemaを渡すAPIはありません。jsonschema依存のHTTP/file解決機能も無効化しています。診断は最大100件、位置はJSON Pointerです。元ファイル位置を取得していないためline/columnはnullです。

## Fixtures

- `examples/arithmetic/`: 自作の日本語教材、全5種類のEntity、単一選択と真偽問題。
- `fixtures/valid/learning-event.json`: Package外に保存する履歴の形状例。
- `fixtures/invalid/`: 回答型不一致、未知core field、必須参照field欠落。
- `crates/osmium-core/tests/schema.rs`: 全定義のmeta-schema適合、offline compilation、良/不正fixture、version拒否、拡張値のroundtripとID制約を検証。
