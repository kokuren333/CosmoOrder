# Osmium Package Format（草案0.1）

状態: 開発版0.1のSchema、SourceのJSON/YAML読込み、意味検証と適合fixtureを実装。[Schema契約](../spec/v0.1/README.md)を参照。下記例は長期互換を保証した公開形式ではない。Distribution、Event永続化は後続Phaseで実装する。

## SourceとDistribution

Sourceは `osmium.yaml` または `osmium.json` をmanifestとし、同時存在はエラー。Entityは明示したJSONファイル、本文はUTF-8 Markdownを基本にする。JSONL/front matterを同時に導入しない。YAMLはJSON互換subsetとし、重複キー、custom tag、無制限aliasを認めない。

```text
source/
  osmium.yaml
  entities/concepts.json
  entities/objectives.json
  entities/curricula.json
  entities/resources.json
  entities/assessments.json
  content/introduction.md
```

manifest形状案:

```yaml
schema_version: "0.1"
package_id: org.example/arithmetic
package_version: "0.1.0"
title: 足し算の基礎
language: ja-JP
capabilities:
  required: []
  optional: []
entities:
  concepts: entities/concepts.json
  objectives: entities/objectives.json
  curricula: entities/curricula.json
  resources: entities/resources.json
  assessments: entities/assessments.json
extensions: {}
```

Coreの対応機能はschema versionが定める。追加のcapabilityは名前空間とversionを含むIDとし、未知requiredなら起動・学習を拒否、optionalならpayloadを保存し、利用可能なfallbackだけを表示する。fallbackも通常の安全性検証対象とする。

Distributionは `manifest.json`、正規化したEntity JSON、Markdown、許可assetsからなるdirectory。`.osmium`はこれをroot直下に置くZIP。元のコメントやauthor notesはSourceに残り、Distributionへの自動同梱はしない。ネットワーク依存の `$ref` を実行時に取得しない。

## Entityの最小形状

| Entity | 必須の中心field |
|---|---|
| Concept | id、title、requires（Concept IDの配列） |
| LearningObjective | id、concept（Concept ID）、description |
| Curriculum | id、title、objectives（順序付きObjective ID配列） |
| Resource | id、type=`markdown`、title、path、teaches（Objective ID配列） |
| Assessment | id、revision、measures、stimulus、response、evaluation、feedback |

EntityのIDはpathと独立、package内の同kindで一意。参照先kindをfieldの契約で決める。v1はpackage内参照だけで、他packageへの暗黙fetchや依存解決はしない。Resourceにcreator/source/license/attribution/provenance、Entityにnamespaced extensionsを保持できるようにする。

Assessment案:

```json
{
  "id": "addition.01",
  "revision": "1",
  "measures": ["addition.basic"],
  "stimulus": {"markdown": "1 + 1 はいくつ？"},
  "response": {
    "type": "single_select",
    "options": [{"id": "a", "text": "1"}, {"id": "b", "text": "2"}]
  },
  "evaluation": {"type": "exact", "answer": "b"},
  "feedback": {"markdown": "1に1を足すと2です。"},
  "extensions": {}
}
```

single_selectの回答はoption ID、booleanの回答はJSON boolean。正解も同じ型。選択肢IDの重複、候補外の正解、空のmeasures、壊れたObjective参照を拒否。採点は1/0、partial credit、randomization、numeric toleranceは後続。

## 安定性・hash

`schema_version`は形式、`package_version`は教材のrelease、`revision`は問題の変更履歴、SHA-256は実際の内容を識別する。それぞれを混同しない。同じpackage ID/versionで異なるdigestのinstallを拒否する。

buildはmanifest以外の全payloadについて相対path/byte size/SHA-256をmanifestに記録し、manifestの正規化bytesからpackage digestを計算する。manifestへ自身のdigestを埋め込まない。ZIP transport digestは別値とし、意味上のpackage digestと区別する。hash一覧にない余分なfileも拒否する。

開発profile `osmium-json-0.1` はUTF-8（BOMなし）、再帰的にkeyをUnicode scalar順へ整列、配列順を維持、余分な空白なし、末尾LFを1つ付ける。文字列のquote/backslashと制御文字をJSON escapeし、非ASCIIとslashはそのまま保持する。Unicodeの文字列正規化はしない。整数は符号付き64-bitまたは符号なし64-bit、その他のJSON numberは有限binary64として扱い、serde_json 1.0.151のcompact出力を開発版の参照実装とする。整数と浮動小数点の`1`/`1.0`、`0`/`-0.0`は区別する。高精度十進値は文字列で持つ。これはRFC 8785準拠や言語横断の数値canonicalization保証ではない。依存更新時も固定vectorを維持し、byte規則変更時はprofile versionを上げる。独立仕様としての安定化は1.0前の課題とする（DD-007）。

MarkdownのCRLFとCRはLFへ変換し、それ以外の空白や末尾改行は維持する。Sourceは変更しない。manifestは`distribution_version: "0.1"`、`canonicalization: "osmium-json-0.1"`、Source manifestを入れた`package`、pathをkeyとする`files`を持つ。各file recordは`size`と64桁小文字hexの`sha256`のみ。自己参照する`manifest.json`はfilesから除外し、hash未記載・未参照・欠落のfileを拒否する。

buildのZIPはpath順、Stored（無圧縮）、固定DOS時刻1980-01-01 00:00:00、Unix permission 0644、directory entryなしで生成する。同じ参照実装ではarchive bytesも再現する。readerはStored/Deflateの通常の単一volume ZIPを受理し、ZIP64・暗号化・symlink・特殊file・重複pathを拒否する。ZIPは68 MiB以下、展開後64 MiB以下、各file4 MiB以下、4,096 files以下。archiveはメモリ内で検証し、任意pathへ展開しない。directory版にはSourceと同じ深度・entry数制限も適用する。異なるOSでの再現試験は未実施。

buildは存在しない出力だけを許可し、親directoryを既存のものに限定する。一時出力の再検証後にrenameする。悪意ある別processによる親directory差替えやPOSIXでの出力先作成競合の完全防御は保証しない。hashは整合性チェックであり、発行者の認証ではない。

## 互換性とmigration

local libraryは`<home>/library/<package-digest>/`のimmutable directoryで管理する。installはdistributionのみを受理し、Sourceは先にbuildする。同一ID/version/digestは再利用、同一ID/versionの異なるdigestは拒否する。複数versionは共存でき、Runtimeの読込み時に複数候補があれば明示version指定を要求する。library上限は1,024 Package versions。`.library.lock`でOsmiumプロセスの操作を直列化し、lock競合はexit 3のI/O診断として再試行可能にする。stagingへ全fileを書いて再検証した後、libraryへ同一filesystemのrenameで公開する。教材本文はDBへ格納しない。

開発版0.1は対応する完全なschema識別子のみ受理し、未知versionは明示拒否する。stable 1.0はconformanceを満たした時点でfreezeし、旧schema/fixturesを残す。breaking changeは新version＋変換器＋旧資料を残す方針とする。未実装のmigration commandを存在するように案内しない。

未知core fieldは誤記防止のため拒否し、optionalな追加情報は `extensions` 内へ置く。unknown payloadは読み書き/buildで損失なく保存する（JSON値としての同等性。コメント・空白の同一性は保証しない）。未知required capabilityをoptionalへ勝手に落とさない。

forkは新package ID、lineageで元ID/version/digestを記録する将来仕様とする。自動的に履歴をコピーしない。Sourceのlockfileは将来のbuild再現性の道具であり、Packageの恒久的意味論とは分ける。

## Learning Event（Package外）

event_schema_version、event_id、device_id、request_id、package ID/version/digest、assessment ID/revision/hash、Objective IDs、問題snapshot、response、score、evaluator ID/version、UTC timestamp、duration_ms、hints_usedを保存する。未計測値はnull。問題のsnapshotには採点に必要な宣言を含め、後の教材変更に影響されず解釈できるようにする。

Learning Eventは利用者のデータであり、配布Packageに含めない。進捗はeventsから再生成し、教材の閲覧によって履歴を外部送信しない。
