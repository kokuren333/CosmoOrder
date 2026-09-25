# Osmium → CosmoOrder Rename Audit

監査日: 2026-09-25  
対象: 現在のworkspace source/config/docs/examples/tests。作業ツリーには既存の未commit変更があるため、clean commitとの差分ではなく現snapshotを観察した。

## 1. Summary and inventory method

現状のブランド名と機械識別子は一体ではない。OsmiumはUI/READMEの表示名だけでなく、Rust crateとimport、CLI executable、Tauri bundle identifier、data directory/env var、Package manifest filename・distribution extension・canonicalization profile、SQLite application_id、state export discriminator、Package ID namespace、localStorage key、diagnostic codes、test fixturesに現れる。

したがって名前の一括置換は不可。CosmoOrderを人間向けproduct brandとして先に採用し、source renameとpersistent/package protocolの変更を分離する。

Inventoryはcase-insensitiveな`Osmium|osmium|OSMIUM`を、`.git`, `target`, `node_modules`, generated build artifactsを除いてsource/config/docs/examples/fixtures/testsに対して検索し、さらにTauri config、Cargo manifests/lock、schema/SQLite/distribution identifiers、mobile/release directoriesを個別確認した。各同一用途の繰り返しは表のlocation groupsにまとめた。binary/build cache内の出現はrename対象としない。

確認結果:

- `spec/v0.1/package.schema.json`にブランド名付き`$id`/schema URIはない。schema `schema_version`は`"0.1"`。
- SQLite/JSONLでの`osmium`付きrecord discriminatorはStoreの`record_type: "osmium-state"`。`fixtures/valid/learning-event.json`自体は`event_schema_version`と`event_type: "assessment_attempt"`を使い、brand discriminatorは持たない。custom media type、deep-link scheme、Package archive magicも定義されていない（distributionはZIP）。
- `osmium-semantic-v1`の実装/永続化は現repoにない。これは将来profile名候補としての調査対象である。
- Tauri `gen/android`, `gen/apple`、`.github`、release workflow/configは存在しない。Android package/iOS bundle/signing IDやcustom deep link/protocol schemeはまだ登録されていない。
- Workspaceに`Osmium-full-review.zip`と`osmium_session_codex_context.md`がある。前者は生成/review artifact、後者は開発コンテキスト文書であり、配布/永続契約ではない。既存成果物をrename/deleteしない。必要なら今後のrepo hygiene作業として別に扱う。
- `icons/{32x32.png,128x128.png,icon.ico,icon.png}`は現行の結晶markを参照し、CosmoOrder名でも継続利用できる。brand textとの技術的衝突は確認されず、icon redrawは不要。

## 2. Current naming inventory

### A. Rust / Cargo

| Location | 現在の値 / symbol | Tier | 互換性・影響 |
|---|---|---:|---|
| root `Cargo.toml` workspace members | `crates/osmium-core`, `osmium-package`, `osmium-store`, `osmium-cli`, `apps/desktop/src-tauri` | 2 | member/package/import graph全体のrename。build/lock/CI/scriptsに波及。 |
| `crates/*/Cargo.toml` | `osmium-core`, `osmium-package`, `osmium-store`, `osmium-cli` package names; `osmium_cli` library / `osmium` binary | 2 | unpublished workspaceでも、Rust `use osmium_*`, `cargo run -p`, `--package`, binary test selectors, local scriptsは壊れる。 |
| `apps/desktop/src-tauri/Cargo.toml` / `src/main.rs` | Desktop crate, `osmium_desktop_lib::run()` | 2 | Tauri entry/lib namingとgenerated artifacts/build profile。 |
| Rust source imports | `osmium_core`, `osmium_package`, `osmium_store`, `osmium_cli`; `crates/osmium-core/src/*` module paths | 2 | source compatibility。module identifiersの大多数自体は`Osmium`を含まず、crate namespace prefixが主依存。 |
| Frontend public IPC error type | `OsmiumError`, `toOsmiumError` in `apps/desktop/src/client.ts` | 2 | UI/tests/imports向けsymbol rename。必要なら移行期間にdeprecated aliasをexport。 |
| validation diagnostic codes | `OSM_*` (e.g. `OSM_IO`, `OSM_REFERENCE`, `OSM_SCHEMA_VERSION`) across Core/Package/Store/CLI and JSON UI/API | 3 | scripts・integrations・testsがcode文字列を機械判定し得る。brandingだけで変更しない。将来変更ならdual-code/aliasまたはoutput schema versioningが必要。 |
| Core query relation values | `requires`, `concept`, `teaches`, `measures`, `orders` | — | ブランド名ではなくdomain semantics。維持する。 |

### B. JavaScript / TypeScript / frontend

| Location | 現在の値 | Tier | Risk |
|---|---|---:|---|
| `apps/desktop/package.json`, `package-lock.json` | npm root package `osmium-desktop` (`private: true`) | 2 | lock root key/build workspace scripts。privateでもtooling/importではrename必要。 |
| `apps/desktop/src/client.ts`, consumers/tests | `osmium` IPC object、`OsmiumError`, `toOsmiumError` | 2 | source-level imports。IPC command strings自体は機械protocolとして別契約。 |
| `apps/desktop/src/App.tsx` | localStorage key `osmium.ui-language`; E2E build flag `VITE_OSMIUM_E2E` | 3 (key), 2 (test flag) | key変更だけで保存済み言語設定が見えなくなる。dual-read/copy before switch. Test env name can be aliased. |
| `apps/desktop/src/e2e.d.ts`, `App.tsx`, `components/Authoring.tsx`, `e2e/desktop-e2e.mjs` | `window.osmiumE2e*`, `__osmiumCopied`, `--osmium`, `osmium.exe`, temp `osmium-gui-*` | 2 | test harness/glueだけ。app protocolではないが同時更新必須。 |
| React components / i18n / HTML | Header/aria label/title/help/import strings `Osmium`, `Osmium教材ファイル`; `index.html <title>Osmium</title>` | 1 | display-only; UI branding migration is safe, screenshots/selectors/title E2E assertions need update. |
| UI code | `OSM_*` errors display; CLI examples `osmium install` | 3 / 2 | Machine diagnostic string and CLI invocation contract; do not treat as cosmetic. |

`apps/desktop/src`にIndexedDB database/store名やCosmoOrder/Osmium由来のroute identifierは見つからない。UI stateはReact内、言語設定は上記localStorage keyにある。

### C. Tauri / Desktop / Mobile

| Location | 現在の値 | Tier | Risk |
|---|---|---:|---|
| `apps/desktop/src-tauri/tauri.conf.json` | `productName: "Osmium"`, `identifier: "org.osmium.desktop"`, `nsis`, product descriptions | productName 1; identifier 3 | ProductName is display brand. Bundle identifier can change Windows package identity, future signing/updater/install identity, and Tauri target bundle conventions. |
| `apps/desktop/src-tauri/src/lib.rs` | `.title("Osmium")`, logs `osmium:`, `OSMIUM_HOME`, `OSMIUM_DEBUG_PORT` | title/log 1; env 3 or test-only 2 | Rename displayed title freely; env override is automation/user config. Continue accepting old env. Debug port variable is developer-facing but test scripts rely on it. |
| `apps/desktop/src-tauri/src/commands.rs` | default authoring source path `.join("Osmium")`; temp `osmium-{uuid}.zip` | default path 3; temp name 2 | Existing user-authored source folder could be left behind if future default changes. Temp name has no persistent contract. |
| Tauri data path vs Tauri `identifier` | Desktop calls package `default_home()` rather than Tauri `app_data_dir()` | 3 but **not coupled as implemented** | Windows default is `%LOCALAPPDATA%\\Osmium`; XDG is `$XDG_DATA_HOME/osmium` or `~/.local/share/osmium`. Changing identifier alone does not currently relocate this Store/library. Changing `default_home()` brand folder would. `OSMIUM_HOME` or `--home` may override. |
| `apps/desktop/src-tauri/capabilities/default.json` | capability identifier `default`, window `main`, dialog permission | — | No Osmium brand in capability identifier. Keep it. |
| Platform IDs / deep link | no generated Android/Apple dirs, package ID, iOS bundle ID, signing config or custom scheme found | future Tier 3 | Select a stable CosmoOrder reverse-domain ID before first mobile release; do not infer app store IDs from current crate name. |
| `apps/desktop/src-tauri/icons/*` / `BrandGem.tsx` | current crystal icon/mark | 1 | Keep unchanged as first option; no brand incompatibility found. |

### D. CLI

| Location | Current machine surface | Tier | Compatibility |
|---|---|---:|---|
| `crates/osmium-cli/src/args.rs`, `main.rs`, `Cargo.toml` | executable and clap name `osmium`; commands include `validate`, `lint`, `build`, `install`, `init`, `learn`, `read`, `answer`, `progress`, `history`, `query`, `context` | 2 | Scripts/docs/package automation invoke executable; JSON envelope and command behavior should not change with brand. |
| CLI library | `osmium_cli::run` | 2 | Rust caller import rename. |
| CLI path/config | `--home`; `OSMIUM_HOME`; OS default | 3 for env/default data path | Old env/config must continue to work. `--home` has no brand and should remain. |
| CLI test/e2e/docs | `osmium install` etc, binary selector `osmium.exe`, outputs/examples | 2 | Update examples and test invocations together. |
| shell completions | no checked-in generated completions/scripts found | — | Regenerate if added later; current checked-in inventory has none. |

Recommended CLI transition: ship `cosmoorder` as new command with unchanged subcommands and JSON contract. Keep `osmium` as a wrapper/second binary depending on the same command implementation for at least two release cycles and document deprecation only after real consumer migration. If publishing Cargo crates, retain old `osmium-cli` shim package or installation instructions; package is presently `publish = false`, so no crates.io compatibility state currently exists.

### E. Package format

| Location | Current identifier/value | Tier | Recommendation |
|---|---|---:|---|
| Source manifest file | `osmium.json` / `osmium.yaml` in Package loader, init, references, authoring, fixtures/examples | 4 | Keep for schema 0.1. Renaming filename breaks every current source loader and fixture. A future format can support both manifests during versioned transition. |
| Local authoring metadata | `.osmium/` hidden source workspace; loader skips it; `.gitignore` | 4 | Keep as v0.1 authoring convention. It is not distributed Package identity. |
| Distribution extension | `.osmium` in CLI build, Tauri dialog filter, E2E, tests, docs | 4 | Keep as v0.1 transport. Extension change is a package-format decision independent of product brand. Importing a new extension must not drop old import. |
| Distribution canonicalization | `osmium-json-0.1` in `distribution.rs`, schema const, `manifest.json`, digest fixtures/docs | 4 | It names byte-level canonicalization. Rename would invalidate reproducibility/digests. Keep immutable; new byte rules need a new profile identifier/version and test vectors. |
| Package schema version / schema URI | `schema_version: "0.1"`; no package `$id` / `osmium` schema URI | 4 | Keep version as-is. Do not invent a brand URI change where none exists. If a published schema URI is added, version it and preserve old URI resolution. |
| Package IDs | schema syntax `reverse-domain-like namespace / slug`; generated `org.osmium.generated/<slug>-<32hex>`; fixtures `org.osmium.pressure/*` | 3 | Package ID is currently identity key used by library uniqueness, CLI selection, events, history. Never mass rewrite existing IDs. Generated namespace can remain legacy or change only for new IDs with no reason to mutate old identity. Existing example IDs remain valid. |
| Distribution internals | `manifest.json`, `distribution_version: 0.1`, package payload fields, per-file digest, package digest | 3/4 | Format/API contract, not product label. Preserve for .osmium v0.1. |
| Other machine types | `org.osmium.exact.v1` extension/evaluator fixture; examples `creator: Osmium編集プロジェクト` | extension Tier 3; creator Tier 1/data content | Namespaced evaluator/capability is a machine contract. Creator string is author-provided example metadata, not runtime brand; decide whether to update examples only after authorship attribution review. |

Package ID semantics confirmed in code: syntax is namespace/slug; generated IDs use `org.osmium.generated` + title-derived safe slug + random UUID v4 hex. Library `packages` uniqueness is `(package_id, package_version)`, digest is immutable distribution payload identity, same ID/version with different digest conflicts. Event/history includes package ID/version/digest. This is a machine identity/reference, **not proof of publisher ownership**. Keep legacy IDs byte-for-byte and do not use brand rename to introduce a publisher namespace migration. A future schema may separate opaque immutable Package identity, editable human slug, and verified publisher identity, but only alongside a real publishing/Hub ownership requirement and migration plan.

### F. Schema / serialization

| Identifier | Current use | Tier | Rename risk |
|---|---|---:|---|
| `$schema` | JSON Schema meta-schema points to JSON Schema draft 2020-12 | — | External standard URI; not Osmium brand. Keep. |
| `schema_version`, `event_schema_version`, `distribution_version`, Store `user_version` | version fields `0.1` / DB 1 | 4 | Actual format versions; never change for brand. |
| `osmium-json-0.1` | canonical serialization selector included in distribution manifest and package digest process | 4 | Changing changes package bytes/digests and install identity. |
| `osmium-state` | JSONL export header `record_type` in `crates/osmium-store/src/lib.rs`; event fixture instead uses `event_type: "assessment_attempt"` | 4 | External state export/import contract, even though importer is not implemented. Keep old readers/writers. |
| `OSM_*` diagnostic codes | structured validation/store/CLI errors | 3 | Machine-facing API used by clients/tests. Retain or version/alias. |
| `org.osmium.generated`, `org.osmium.pressure`, `org.osmium.exact.v1` | generated/example Package IDs and extension/evaluator ID | 3/4 | ID is identity and extension namespace. No blind prefix rewrite. |
| `osmium.ui-language` | persistent browser localStorage preference | 3 | Read old key and copy/dual-write on UI rename to preserve preference. |
| `OSMIUM_HOME` | user/CLI/desktop location override | 3 | Keep as a supported alias; optional new `COSMOORDER_HOME` can be checked first with documented precedence, never silently point at a new empty home. |
| SQLite `application_id=1330859337` (`0x4F534D49`, ASCII `OSMI`) and `user_version=1` | Store database validation (`lib.rs`, migration `001.sql`) | 4 | SQLite open rejects foreign application id. Rewriting it on name change can make an existing DB appear foreign. Keep accepted value. |

No serialized enum/type tag beginning with `osmium` was found for Core entity polymorphism. Distribution package JSON is a fixed v0.1 profile without a brand-specific schema URI. Rust JSON DTO naming/serde field casing is a separate API surface; do not rename DTO keys simply for brand alignment.

### G. Database / persistence

| Location | Current persisted data | Tier | Data-loss risk |
|---|---|---:|---|
| `Library::default_home()` | Windows `%LOCALAPPDATA%\\Osmium`; XDG `<XDG_DATA_HOME>/osmium`; fallback `~/.local/share/osmium`; override `OSMIUM_HOME` | 3 | Changing default directory can make installed Packages and history appear missing. Current Desktop delegates here, not Tauri `app_data_dir()`. |
| Package folders | `<home>/library/<digest>/`, `<home>/staging`, `.library.lock` | 3 | Keep layout/read old directory. |
| SQLite | `<home>/state.sqlite`; tables `settings`, `packages`, `events`, `progress`; migration `001.sql`, app id `OSMI`, DB version 1 | 3/4 | Preserve DB filename, migration history/application_id. Brand does not justify a DB migration. |
| Events / derived progress | package ID/version/digest, assessment snapshot/answer/score; progress keyed by digest+objective | 3 | IDs/digest and append-only history must remain intact. Migration must be copy/verify/rollback-capable if home path changes. |
| State export | `record_type: "osmium-state"` JSONL | 4 | Keep old export discriminator for v0.1. Any renamed export should read both. |
| Frontend preference | localStorage `osmium.ui-language` | 3 | Dual-read/copy. |

Safe future home-directory migration: first determine whether user supplied `--home`/`OSMIUM_HOME`/new override; explicit override always wins. For defaults, inspect legacy and destination; if only legacy exists, copy with SQLite backup/closed connection and immutable Package verification, fsync, compare hashes, mark migration completion atomically, and retain the old directory until the new app has opened/verified it. If both exist, do not merge/overwrite automatically; report both paths and require an explicit recovery choice. Idempotent retry and interruption tests are required. Better near-term option: continue reading/writing the legacy default indefinitely.

### H. Documentation / examples

- Root `README.md`, `docs/{ARCHITECTURE,PACKAGE_FORMAT,DESKTOP,IMPLEMENTATION_PLAN,ROADMAP,AUTHORING_*,ASSESSMENT_MODEL,INTEROPERABILITY,SOURCES_AND_PROVENANCE,AI_AUTHORING,AGENT_INTEGRATION,DEPENDENCIES,SKILLS,DESIGN_DECISIONS,...}.md`, `docs/architecture/*`, `spec/v0.1/README.md` contain product names, crate names, commands, extension/manifest/profile references, and screenshots/paths.
- Earlier audit inventory docs (PRE/POST P0/P1) and handoff docs contain historical name/version/commands. Treat as historical records; annotate current brand rather than rewriting historical facts.
- `examples/*/osmium.json` and Resource creator fields include old names/IDs. Manifest filename, package_id and attributed creator are not all branding: preserve v0.1 paths/IDs; update only project-authored UI copy after authorship review.
- `Osmium-full-review.zip` and `osmium_session_codex_context.md` are workspace artifacts. Do not rename or delete during product migration.

### I. Tests / fixtures / samples

- Rust integration suites: `crates/osmium-core/tests/*`, `crates/osmium-package/tests/{source,library,distribution}.rs`, `crates/osmium-store/tests/state.rs`, `crates/osmium-cli/tests/cli.rs`; desktop tests `apps/desktop/tests/*`, `apps/desktop/src-tauri/tests/desktop.rs`; E2E `apps/desktop/e2e/desktop-e2e.mjs`.
- Test contracts include `osmium.exe`, `osmium install`, `.osmium`, `osmium.json`, `.osmium/`, generated `org.osmium.generated/...`, pressure IDs, `OSMIUM_HOME`, `OSMIUM_DEBUG_PORT`, `VITE_OSMIUM_E2E`, `window.osmiumE2e*`, `Osmium` header/title, and `org.osmium.exact.v1`.
- Fixtures/examples include `fixtures/valid/learning-event.json`, `examples/*/osmium.json`, Tauri valid/broken authoring fixtures. Package paths and exact expected bytes are compatibility tests; update only those whose contract actually changes.
- Existing GUI custom crystal assertion in E2E is independent of text branding; keep its asset/assertion unless later product design finds a real semantic mismatch.

### J. CI / release / repository infrastructure

- No `.github` workflows, release scripts, installer manifests, shell completion files, Dockerfiles, or mobile build configurations are currently present in this workspace snapshot.
- Tauri bundle target is Windows NSIS only (`apps/desktop/src-tauri/tauri.conf.json`); output name/product metadata and app `identifier` affect installer artifact identity.
- Cargo `Cargo.lock`, npm `package-lock.json`, `target/` outputs, E2E executable path and screenshots are generated or tooling-facing. Regenerate lockfiles only as part of source package rename; never manually search/replace locks alone.
- Git remote, release/tag/branch namespace were not changed or treated as application brand. Repository URL/name and existing tags are external governance decisions; no evidence in workspace requires changing them now.

### K. User-visible branding

Safe display targets include Tauri `productName`, Tauri window title, HTML title, app header/aria label, Library import label, user-facing help/description, README current product heading, and future About/help strings. Update E2E text/title expectations at the same time. Keep current crystal icon.

## 3. Rename tiers

### Tier 1 — Safe cosmetic rename

Change to **CosmoOrder** now/when requested: README product heading and current explanatory prose, Tauri `productName`, window/HTML title, app header, user-facing strings, Tauri short/long product descriptions, current help labels and icon alt/accessibility text. Keep the icon asset. Do not rewrite dated audit/handoff facts or author-provided citation/creator metadata as if they were UI labels.

### Tier 2 — Source-level rename

After cosmetic transition is stable, rename Cargo package/crate/module/import prefixes, CLI package/library names, private npm app package, Tauri Rust library symbol, frontend error class/helper, developer test variable names and temp output labels. Regenerate `Cargo.lock` and `package-lock.json`, update docs/scripts/E2E and every crate reference together. Rust downstream imports and direct CLI scripts break unless aliases/shims are provided.

### Tier 3 — Persistent / API identifiers

Do not change as routine branding: `org.osmium.desktop` application identifier, `OSMIUM_HOME`, default data directory `Osmium/osmium`, localStorage key, `OSM_*` diagnostic codes, generated/example Package IDs, extension IDs, SQLite `application_id`, schema/record types, file paths/formats, application-support directories, shell command contract. If changed for a deliberate migration, require compatibility reader/alias, an explicit migration, and migration tests.

### Tier 4 — Keep as Osmium legacy identifier

Keep for v0.1 compatibility: `osmium.json`/`osmium.yaml`, `.osmium/`, `.osmium` distribution extension, `osmium-json-0.1`, `osmium-state`, existing `org.osmium.*` Package/extension IDs, `OSM_*` diagnostic family, SQLite OSMI application_id and DB migration history. These are not evidence that the user-facing product must remain named Osmium.

## 4. Proposed target naming

| Surface | Recommended name | Transition notes |
|---|---|---|
| Product/project | **CosmoOrder** | Human-facing name, title case. |
| Machine prefix | `cosmoorder` | Lowercase, no punctuation except existing Rust/CLI separators. |
| Rust crates | `cosmoorder-core`, `cosmoorder-package`, `cosmoorder-store`, `cosmoorder-cli`, `cosmoorder-desktop` | Keep module semantics unchanged; legacy import facades possible during source transition. |
| CLI | `cosmoorder` | New invocation; `osmium` compatibility wrapper for multiple releases. |
| npm private app | `cosmoorder-desktop` | Source/tooling only, lock regenerate. |
| Tauri display | `CosmoOrder` | UI metadata only. |
| Tauri app/bundle ID | Candidate `org.cosmoorder.desktop` | Do not switch until install/update implications and legacy user home handling are explicitly tested; choose mobile IDs before mobile launch. |
| New generated Package ID namespace | no change required for rename | Existing Package ID is identity. Consider a new prefix only with a separate package identity/publisher policy, never migrate old IDs by string substitution. |
| Package v0.1 | `osmium.json`, `.osmium`, `osmium-json-0.1` | Keep exact compatibility. |
| future semantic profile | choose a versioned profile when implemented (e.g. `cosmoorder-semantic-v1`) | If old vectors exist, retain old profile and recompute/dual-cache; no current data to migrate. |

`cosmoorder-runtime` as a crate name is only warranted if Runtime is extracted into a distinct crate. Today shared Runtime operations live in `osmium-store::runtime`; do not rename/restructure crate boundaries just to match a wish-list. Prefer `cosmoorder-store` initially unless the architecture later separates Runtime from Store.

## 5. Legacy compatibility policy

### Package and schema

Brand and format are separate. `.osmium`, manifest basenames, schema v0.1, package IDs, distribution canonicalization and event/schema identifiers stay readable. A future new extension (whether `.cosmoorder`, `.cosmo`, or another) is **not decided in this audit**. If justified by independent format requirements, a future version should accept old `.osmium` as import alias, declare canonical output by new format version, and preserve digest semantics for existing artifacts. Do not mutate the v0.1 canonicalization profile.

No brand-specific schema URI exists today. If an external URI is introduced in future, keep it resolvable and version it; cosmetic domain/brand change alone does not justify changing schema compatibility.

### CLI and app data

Keep `osmium` as an alias/wrapper for `cosmoorder` for at least two release cycles. `OSMIUM_HOME` remains accepted. If introducing `COSMOORDER_HOME`, define precedence (explicit `--home` > new variable > old variable > legacy OS default) and avoid silently selecting a new empty folder. The implemented Desktop uses Package `default_home()`, not Tauri's `app_data_dir()`, so current default storage paths are based on `LOCALAPPDATA`/XDG and brand folder names, not `org.osmium.desktop` directly. Bundle-ID changes still affect installer identity/update/signing and must be tested independently.

### Package IDs

`package_id` is the current install/store/CLI identity. Generated `org.osmium.generated/<title-slug>-<UUIDv4>` reduces local collisions but embeds both slug and a namespace-like prefix. Treat this as v0.1 legacy machine identity, not publisher verification. Never rewrite existing IDs; a brand rename is not a fork/rename of the learning Package. Future identity/slug/publisher decomposition belongs with a new format version and migration design, not this rename.

## 6. Desktop / mobile naming decisions

- **Windows**: Tauri NSIS package/product metadata changes installer branding. `%LOCALAPPDATA%\\Osmium` is current default data root, independently computed by `default_home()`. Keep/locate it after cosmetic rename. If choosing a new root, migrate with verified copy and preserve rollback.
- **macOS/Linux**: currently no checked-in macOS target. Linux default `XDG_DATA_HOME/osmium` or `~/.local/share/osmium`; preserve. If bundle ID participates in a later Tauri platform data path, do not assume current store uses it.
- **Android/iOS**: no generated app projects or app IDs currently. Pick stable reverse-domain application ID before first public release; validate Tauri plugins, Store path, filesystem picker, backup, signing, store listing and update identity on-device. Do not rename a shipped application ID casually.
- **Deep links/protocols**: none currently configured. If added, register a new `cosmoorder:` scheme but optionally continue old `osmium:` links during transition; validate OS handler conflict/ownership before launch.
- **Signing**: no checked-in signing identifiers/credentials/config found. Never commit secrets. Plan certificate/app-store transfer separately from source rename.
- **Icon**: keep crystal icon/mark. Existing Tauri assets and React mark are brand-neutral enough; only wordmarks/accessibility strings need CosmoOrder.

## 7. Recommended rename sequence

### R0 — Brand at the presentation layer

Update current UI/README and Tauri product/window metadata to CosmoOrder; retain current icon. Keep package/schema/DB/app identifiers, paths, CLI and all machine values. Update screenshot/title/text assertions. This makes product identity CosmoOrder without data migration.

### R1 — Source naming

In a dedicated change, rename Cargo packages/crates/imports/library symbols and private npm package. Regenerate locks via Cargo/npm; update build/test/docs references. Keep app data root and protocol/package identifiers untouched. Add Rust re-exports only if there is an actual downstream API consumer or published compatibility promise.

### R2 — CLI transition / desktop installer

Introduce `cosmoorder` binary and `osmium` shim/alias; preserve JSON output and `--home`. Keep both env var names. Validate NSIS upgrade/uninstall, executable collisions, and prior-install discoverability before changing `identifier`. Handle app-data migration only if the existing default path is intentionally moved; current CLI/Desktop explicit data root means it need not move.

### R3 — Persistent/package identifiers only if independently justified

Handle a new schema/distribution/profile version with explicit old readers, test vectors, migration and export/import policy. Keep all v0.1 identifiers. Do not include this phase in a brand release absent concrete interoperability/security need.

## 8. Explicit rename non-goals

Brand update alone must not change package format version, `.osmium`, manifest filename, schema URI/version, distribution canonicalization/digests, SQLite tables/application_id/migration history, state export record type, stable Package IDs, `OSM_*` diagnostic codes, user data directory, or stable external API fields. Do not redraw the icon just to match the new wordmark. Do not turn brand rename into Package identity/publisher redesign.

## 9. Estimated blast radius

| Change | Risk | Why |
|---|---:|---|
| UI text, README current heading, Tauri product/window name | Low | Presentation only. Must update screenshot/E2E expectations. |
| crate/import/npm rename | Medium | Build graph, Rust importers, local scripts, lockfiles and tests. No current crates.io publishing because workspace `publish=false`. |
| CLI executable switch | Medium–High | User scripts and automation may call `osmium`; alias/wrapper controls risk. |
| Tauri bundle identifier | High | Installer/update/signing/application identity; OS integration; future Android/iOS identifiers. Not currently the Store data path, but may affect distribution continuity. |
| Default home path, env var, localStorage key | High | Existing library, state, language preference can appear lost. Use alias/read-copy migration. |
| `.osmium`, manifest filename, canonicalization profile, Package IDs, schema/event/diagnostic identifier, SQLite app ID | Very High | Existing Package loading, package digest, history, consumers, migrations and archived artifacts. Retain legacy; only change under an independently versioned compatibility plan. |

## 10. Recommended decision

### Change now (when the product rename itself is approved)

- Human-facing current product labels and newly edited README/product descriptions → **CosmoOrder**.
- Tauri `productName` and window/HTML title → **CosmoOrder**.
- Keep the existing crystal icon.

### Schedule with a transition period

- Cargo crate/module/import names, private npm package, frontend `OsmiumError` symbols.
- New `cosmoorder` CLI with old `osmium` command wrapper for at least two releases.
- Tauri bundle identifier only after a packaging/update migration test; keep old package discoverability and existing data root.
- Optional `COSMOORDER_HOME` while reading `OSMIUM_HOME` indefinitely or through a documented deprecation period.
- localStorage dual-read/copy if a new key is used.

### Keep as Osmium legacy for v0.1

- `.osmium`, `.osmium/`, `osmium.json` / `osmium.yaml`, `osmium-json-0.1`, `osmium-state`.
- `org.osmium.*` IDs and extensions already in use; SQLite `OSMI` application_id / state DB migration history; `OSM_*` diagnostics.
- Existing OS default data folders and `OSMIUM_HOME` support.

## 11. Final rename answer

**K. What can break?** Rust callers and Cargo selectors, npm tooling, command invocations, Tauri installer/update identity, app data defaults, localStorage settings, state DB open checks, package/archive parsing, hashes/digests, Package references/history, diagnostic-code consumers, test fixtures and generated release names. The highest-risk items are persistent identity/protocols, not UI labels.

**L. What now vs legacy?** CosmoOrder now for visible brand only. Source identifiers can migrate in a separate R1 with locks/tests updated. Keep all v0.1 Package, schema, Store, state-export, diagnostic and existing-ID values as Osmium legacy. Changing the Tauri identifier does not currently relocate the database because Desktop and CLI explicitly use `default_home()`; changing that home computation would.

**Recommended sequence:** R0 cosmetic → R1 source names → R2 CLI aliases and installer/data-path verification → R3 persistent identifiers only under a new format/version with justification. This sequence allows CosmoOrder to become the product name immediately while keeping Osmium-era machine contracts usable.
