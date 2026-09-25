# CosmoOrder P0 Atlas-ready Runtime implementation

実装日: 2026-09-25  
対象: v0.1 schemaを維持したPackage-local read model、history/uninstall、CosmoOrder R0表示名。

## 1. 実際に変更したもの

- `crates/osmium-core/src/query.rs`: 既存のbounded `context`のConcept近傍が、Concept / prerequisite / Objective / Resource / Assessment / Curriculum順を一度に返し、Curriculum順を`requires`から分離したままであることをtestで固定した。既存のnode/depth/byte boundsも対象に含める。
- `crates/osmium-store/src/runtime.rs`: versionを選択してCore contextを呼ぶread-only adapterを追加した。
- `apps/desktop/src-tauri/src/commands.rs`, `src/lib.rs`: bounded `package_context` commandを登録。併せてuninstall後の保存済み回答を全Packageから読める`all_history`を登録した。
- `apps/desktop/src/client.ts`, `src/types.ts`: DTOとIPC clientを追加した。
- `crates/osmium-store/src/lib.rs`: newest-firstのbounded全体履歴queryを追加した。DB schema変更なし。
- `apps/desktop/src/App.tsx`, `AppShell.tsx`, `HistoryPanel.tsx`: Packageが未選択でもHistoryを開け、uninstall済みPackageのイベントsnapshotに残る問題文、Package ID/version、Objective IDを表示できるようにした。
- `crates/osmium-store/tests/state.rs`, `apps/desktop/src-tauri/src/commands.rs`, `crates/osmium-core/src/query.rs`, `apps/desktop/tests/presentation.test.ts`, `apps/desktop/e2e/desktop-e2e.mjs`: semantics、bounded API、history retention、orphan history UIを確認するテストを追加した。
- Desktopの画面名、window/document title、Tauri productName、READMEの現在の製品説明をCosmoOrderにした。結晶アイコンはそのまま。

## 2. 変更しなかったもの

schema version、Package format、SQLite migration/table、crate/binary名、CLI invocation、`.osmium`、`osmium.json`、`.osmium/`、schema/state identifiers、`OSM_*`、`OSMIUM_HOME`、Tauri bundle identifier `org.osmium.desktop`、data directory、iconは変更していない。新しいGraph schema、Route推薦、mastery、Activity/Evidence、embedding、Personal Atlasも追加していない。

## 3. Lesson read model

既存の`Runtime::lesson`と`apps/desktop/src/components/Lesson.tsx`がPackage-local Lessonを作る。現状の表示中心はCurriculum → Objective/Concept groupingと、そのObjectiveを`Resource.teaches` / `Assessment.measures`で結びつけた教材と問題である。本文はResource、確定的な回答評価はAssessmentが担う。これはPackage宣言内容の表示で、個人向け推薦ではない。

## 4. Route read model

新しいRoute entityや推薦APIは作っていない。v0で説明可能なRouteは、既存のCurriculumのauthor order、`Concept.requires`の前提参照、Lesson内のcurrent itemから合成できる。Curriculum positionは`orders`関係とCurriculumのordered `objectives`、前提は`requires`関係としてCore DTOから別々に読み取る。`before/current/next`はCurriculumまたは既存reader orderから得る表示状態であり、adaptive recommendationではない。今のDesktopには専用Route画面や「なぜ次か」の説明UIはまだない。

## 5. Atlas read model

`osmium_core::query::context(model, EntityKind::Concept, id, depth, node_limit)`を再利用した。`Runtime::context`は指定したinstalled Package/versionを選択し、Coreへread-only委譲する。Tauriの`package_context`は既定depth 2 / node limit 64で呼び出せる。Core側の絶対上限はdepth 8、512 nodes、256 KiB DTO payloadであり、未知ID、kind、過大なbounds、過大targetはdiagnosticで拒否する。

既存Context DTOはtarget summary、bounded nodes、typed relation endpoints、relation name、`incoming`方向を含む。Concept targetから必要な深さを取れば、前提Conceptとdependent Concept、属するObjective、teaches Resource、measures Assessment、関連Curriculumとその順序を取得可能である。Core testはarithmetic fixtureの1つのConceptについて全種類のnode/relationを検査する。source Packageは不変で、Desktopにrelation解決ロジックは複製していない。本格的なAtlas UIはP1。

## 6. Semantic invariants

- `Concept.requires`: Package-localな有向prerequisite relation。一般的なrelated edgeでも、mastery relationでもない。Core validationは参照を解決し、cycleを拒否し、決定的なtopological orderではprerequisiteをdependentより前に置く。
- `Curriculum.objectives`: 作者定義の順序付きpedagogical sequence。Knowledge graphやadaptive routeではなく、`requires`から生成しない。Core contextの`orders` relationはこの別semanticを示す。
- Store progress: `package_digest + objective_id` scoped observed projection (attempt count, correct count, latest score/time)。mastery estimateではない。違うdigestには自動transferせず、同じdistributionの再導入では既存projectionが再利用される。
- Learning events are append-only snapshots. Package uninstallation removes payload but does not delete events.

## 7. History / uninstall findings

確認したgapは、Storeにeventが保持されても、Packageがない時のHistory導線がなく、Package依存の問題文lookupができないことだった。全体history queryと既存eventの`assessment_snapshot.stimulus.markdown`を使って表示を保つ変更で解決した。DB migrationは不要。削除済みPackageのタイトル・Objective descriptionのsnapshotは現行eventにないため、タイトルはPackage ID/version、ObjectiveはIDで表示する。この情報の豊富化が必要なら将来のevent snapshot contractとして提案し、今回は既存eventを書き換えない。

Store integration testsはuninstall後もpackage-scoped及びglobal historyにイベントとsnapshotが残ることを確認する。E2EではPackageをuninstallしたfresh sessionから履歴導線を開く。

## 8. CosmoOrder R0 rename結果

表示名の範囲でDesktop header / accessibility label / title / Tauri `productName` / README current product heading・description / import-export dialog labelsをCosmoOrderへ変更した。既存iconは保持した。`.osmium`や`osmium.json`を含むmachine compatibility identifiersは変更していない。R0はbrand display onlyであり、crate / CLI / app-data / protocol migrationを意味しない。

## 9. Test results

- `cargo test --workspace`: pass。追加を含むCore query / Package authoring / Store state / Tauri adapter tests、全workspace testsとdoc-testsがgreen。Storeにはsame-digest uninstall/reinstall restorationを追加した。
- Desktop `npm test`: 23/23 pass。削除済みPackageのsnapshot問題文、package/version、Objective IDをHistoryPanelが描画するpresentation testを含む。
- `npm run typecheck`: pass。
- `npm run lint`: pass。
- `npm run build`: pass。minified JS bundleが500 kBを超えるという既存のVite advisory warningはあるがbuild failureではない。
- Desktop E2E: pass、264 checks。E2E専用frontend buildと更新したTauri executableを使用し、最後に通常buildへ戻してTauri executableも再buildした。全Packageをuninstallした新規セッションからHistoryを開き、event snapshotを確認した。

## 10. P1で初めて実装すべきもの

1. `package_context`を使うread-only Package Atlas: local neighborhood / search / layer-detail UI、Core-owned context DTOだけを使う。
2. Curriculum orderと`requires`を区別したRoute view、現在位置と明示された関係の説明。新しいadaptive recommendationではない。
3. Knowledge state overlayはobserved attempts / recently correct or incorrect等に限定し、Package canonical graphとRuntime-inferred stateのprovenanceを区別する。
4. Orphan HistoryにPackage title / Objective descriptionを出す価値が高い場合、append-only compatibilityを保つevent snapshot拡張案を別途設計する。

Personal Atlas、embedding、mastery transfer、new Activity/Evidence schema、全graph描画、Hub/global ontologyはこのP0にもP1 Atlas prototypeにも含めない。
