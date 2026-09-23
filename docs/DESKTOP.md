# Osmium Desktop 操作手順

状態: 最小v1のDesktop Runtime。Tauri 2 + React + TypeScript + Vite。重要ロジックは
Rust Core / `osmium_store::runtime` にあり、UIはそれを呼ぶだけです。採点、検証、
version解決、progress集計をTypeScript側へ複製していません。

本文のMarkdown表示はGFM table/strikethrough、数式（KaTeX）、コード強調表示を含みます。raw HTMLは解釈せず、コードは表示・コピーのみで実行しません。主要なRuntime UI文言は日本語／Englishで切り替わり、Package titleと本文の言語とは独立しています。wide table/equation/codeは横スクロールします。狭い画面と200%文字サイズでの基本responsive CSSはありますが、実機mobile・200% zoomでの視覚E2Eは未実施です。

## 構成

```text
apps/desktop/
  src/                 # React renderer（表示と操作のみ）
    client.ts          # Tauri command への唯一の経路。fetch/絶対path/動的loadなし
    outline.ts         # Package宣言の読み取り専用ビュー（順序や正誤は決めない）
    components/        # AppShell / DeveloperDetails / Library / Lesson / Reader / Assessment / Progress / History
  src-tauri/
    src/commands.rs    # command層。runtime + content IR を呼ぶだけ
    src/lib.rs         # window生成、data root決定、command登録
    tauri.conf.json    # CSP、frontendDist、bundle設定
    capabilities/      # core:default のみ（外部plugin権限なし）
    tests/desktop.rs   # windowなしの受入れ試験
  e2e/desktop-e2e.mjs  # 実アプリをCDPで操作するE2E受入れ
```

## Windows での起動方法

前提: Rust stable（MSVC）、Node.js 20以上、WebView2 Runtime（Windows 11は標準同梱）。
MSVC Build Tools が必要です。

```powershell
# 1. rendererをbuild（dist/ を生成。Rust buildはdistを埋め込む）
cd apps/desktop
npm ci
npm run typecheck
npm run lint
npm test
npm run build

# 2. Desktop実行ファイルをbuild
cd ../..
cargo build --workspace

# 3. 起動（debug）
.\target\debug\osmium-desktop.exe

# 4. 配布用bundle（NSIS installer）を作る場合
cd apps/desktop
npm run tauri build
```

`npm run build` は必ず `cargo build` より先に実行してください。`apps/desktop/dist` の内容が
実行ファイルへ埋め込まれます。`apps/desktop/src-tauri/build.rs` がrenderer sourceを
Cargoの再ビルド対象として登録しているため、rendererを編集した後に `cargo build` を
やり直せば埋め込みも更新されます。

## 保存場所

既定はOS標準のユーザーデータ領域です。

```text
%LOCALAPPDATA%\Osmium\
  library/<digest>/        # 検証済みPackage本文（immutable）
  staging/                 # 導入途中の隔離領域
  state.sqlite             # installed index + append-only learning events + progress
  .library.lock            # プロセス間排他lock
```

`OSMIUM_HOME` を設定すると切り替わります。CLIの `--home` と同じ解決規則です。Desktopは
起動時に `OSMIUM_HOME` を読み、未設定なら上記既定を使います。データrootとDB pathは
画面下部の「実行環境の詳細」を開くと確認できます。

## 教材を開いて問題を解く手順

1. **ライブラリ**: 起動直後の教材カードから対象Packageを選びます。一覧は導入済み
   Packageを再検証してから表示します。0件の場合は先にCLIでinstallしてください。
2. **Curriculum / Concept**: 選択したPackageのCurriculum、Concept、Objectiveが表示され
   ます。テーマ内の学習目標ごとに「読んで理解する」「問題で確かめる」が関連づけられます。
   「最初の教材を読む」は目次の先頭を開きます。推薦・習得判定は行いません。
3. **教材を読む**: 教材名を選ぶと本文を表示します。本文の上下にある「前の教材」「次の教材」で
   パッケージ宣言順に移動できます。Package由来のHTMLは実行されず、literal textとして
   表示されます。Resource末尾の「参考資料」を開くと、Resourceの`source_ids`に結び付いた
   根拠資料を確認できます。publicは題名・書誌情報・locator、attribution_onlyは題名・書誌情報を
   表示し、privateは表示しません。公開URLは現在クリック可能にせず文字列表示します。
4. **問題で確かめる**: 問題文を選ぶと問題を表示します。`single_select` と `boolean` は
   いずれも回答カードを選択し、「採点する」で送信します。採点はRustの評価器
   `org.osmium.exact.v1` v1 が行い、結果とfeedbackが表示されます。採点後に次問へ進めます。
5. **LearningEvent / Progress**: 回答ごとにappend-only eventが保存され、progressが
   更新されます。「進捗」でObjective単位の試行数・正答数・正答率・最終回答時刻、
   「履歴」で問題文・正誤・日時を確認できます。概要の回答数と正答数はObjective別の
   観測値を合算した延べ件数です。内部IDや回答のraw値は詳細、進捗の再構築はメンテナンス内にあります。
6. **再起動**: windowを閉じてプロセスを終了し、再度起動すると installed package、
   history、progress がそのまま復元されます。同じ `request_id` を再利用しない限り、
   再回答は新しいeventとして記録されます。

画面上部の共通ナビゲーションからライブラリ・現在の教材・進捗・履歴へ移動できます。
文字サイズはヘッダーの「文字サイズ」を開き、100% / 150% / 200%に切り替えられます。keyboard操作、
focus表示、`prefers-reduced-motion`、`prefers-color-scheme` に対応しています。

## Golden Package の build と install

```powershell
# 検証とlint（source）
.\target\debug\osmium.exe validate examples\arithmetic --json
.\target\debug\osmium.exe lint examples\arithmetic --json

# 再現可能な配布物を作る（既存出力は上書きしない）
.\target\debug\osmium.exe build examples\arithmetic --output arithmetic.osmium

# 配布物を検証してからlibraryへinstall
.\target\debug\osmium.exe validate arithmetic.osmium --json
.\target\debug\osmium.exe install arithmetic.osmium --json
.\target\debug\osmium.exe packages --json
```

install中はlibrary lockをCLIが保持するため、Desktop起動中はinstallできません。逆も
同じです。Desktopを終了してからCLI操作を行ってください。

## 検証

```powershell
# Rust全体
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace

# renderer
cd apps/desktop
npm run typecheck
npm run lint
npm test
npm run build

# 実アプリのE2E（Windows、WebView2が必要）
$e2eHome = Join-Path $env:TEMP ("osmium-e2e-" + [guid]::NewGuid().ToString())
npm run e2e -- --home $e2eHome --app "..\..\target\debug\osmium-desktop.exe"
```

E2EはGolden Packageの validate/lint/build/install、Desktop起動、installed package表示、
Curriculum/Concept閲覧、Markdown本文表示、回答と採点、feedback、progress、history、
完全終了、再起動後の復元までを実際のwindowで確認します。同時にwebviewが
`tauri.localhost` 以外のnetwork requestを出していないことも検証します。
`--home` は新規または空のディレクトリを指定してください。既存データを含む場合は拒否します。
選択カードとキーボード、真偽式、技術情報のdisclosure、light/darkと100/150/200%の
横幅も検証し、`<home>/screenshots/` に表示確認用PNGを保存します。

## v1での制限

- Desktopは1プロセス1sessionでlibrary lockを保持します。CLIと同時実行できません。
- Package本文の画像・音声・動画は未対応です。v0.1のResourceはMarkdownのみで、
  参照されないファイルはdistributionに含められないため、asset解決経路はありません。
- 数式はTeX sourceを不活性テキストとして表示するだけで、組版しません。
- 表・脚注・定義リスト・取り消し線などの拡張は構造化せず、テキストとして保持します。
- 採点は `single_select` と `boolean` のexact評価のみです。
- progressは観測値であり、習得の保証ではありません。version間で自動統合しません。
- UIの自動テストはDOM/型/logic単位で、スクリーンリーダー実機確認は未実施です。
- installer bundle（NSIS）の実配布試験は未実施です（debug実行とE2Eのみ検証済み）。
