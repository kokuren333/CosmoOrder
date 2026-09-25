# Osmium Post-P0/P1 Adversarial Re-audit — 2026-09-24

## Verdict

**Do not freeze/tag this working tree as a release candidate yet.** The main
P0/P1 implementation paths pass the recorded Windows E2E suite (250 checks),
but this re-audit found two user-visible/data-integrity gaps that should be
resolved and retested first.

The pre-implementation inventory is preserved as
[`AUDIT_INVENTORY_2026-09-24_PRE_P0_P1.md`](AUDIT_INVENTORY_2026-09-24_PRE_P0_P1.md).
This document records the post-implementation state and remaining gaps; it
does not replace that baseline.

## Scope and verification evidence

Reviewed the Authoring workspace save path, Core validation boundary, library
inventory/read/install/uninstall paths, Store event/progress semantics, Desktop
History/Library routing, distribution compilation, and the tests covering
these paths. The working tree remains user-owned WIP; this re-audit did not
change runtime/schema behavior.

Verification recorded for the same working tree:

- `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo clippy --workspace`, and `cargo test --workspace`: pass.
- Desktop `typecheck`, `lint`, `npm test` (22/22), and normal build: pass.
- Windows Tauri/WebView E2E: 250 checks pass, including explicit versions,
  multi-entity authoring, save/check/export/import, learner answers, restart,
  version-specific uninstall, and broken-source diagnostics.
- `git diff --check`: pass.
- Native Windows folder/file dialogs are replaced with E2E-only path injection;
  those dialogs have not been manually pressure-tested in this audit.

## Findings

### P1 — One corrupt payload prevents opening other healthy packages

Runtime startup and the Library inventory tolerate a damaged payload, so the
user can see it and remove it. However, opening any package still goes through
`Library::read`, which calls the strict `packages()` scan. If package B fails
verification, that scan returns an error before package A or C can be read.
The UI leaves A/C open buttons enabled, but their opens fail while B remains.

Evidence: `Library::inventory` and `valid_packages` skip/mark damaged entries,
while `Library::read` at `crates/osmium-package/src/library.rs` calls
`packages()`. The current damaged-package test proves B can be removed and its
history retained; it does not install A/B/C and open A/C with B still present.
A disposable copy of the completed E2E Library confirmed the failure: reading
healthy `org.example/arithmetic@0.1.0` succeeded before tampering with
`org.osmium.pressure/mathematics`; after changing one payload file in B, the
same healthy read failed with `OSM_HASH` for B.

**Required before freeze:** make healthy package reads independent of unrelated
damaged entries, then test A/B/C list, open A, open C, and remove B.

### P2 — Deleting a Resource leaves an orphan Markdown file in its source folder

`save_workspace` builds `candidate_files` by cloning every loaded source file,
then overwrites files for Resources still present in the draft. Removing a
Resource removes its entity record but does not remove its old Markdown path.
`load_source` only returns files declared by the package, so `compile_source`
does not include this orphan in the `.osmium` archive. A disposable-copy probe
confirmed the file remains in the source folder but is absent from the built
archive. This is a cleanup/author expectation issue, not an artifact disclosure.

Evidence: `crates/osmium-package/src/authoring/workspace.rs` around
`candidate_files` and the final `writes` loop; `crates/osmium-package/src/lib.rs`
loads only declared files. A temporary-source build confirmed the orphan is
absent from the archive.

Define source-file cleanup semantics and, if deletion should remove the file,
delete it safely after reference validation. Add a test that checks both the
source folder and the artifact; preserve the current Package behavior of
excluding undeclared files from distributions.

### P1 — Uninstall preserves history in Store but the Desktop cannot show it

The event rows remain queryable through `Runtime::history(package_id, version)`
after uninstall. The current Desktop History route is coupled to an installed
`lesson`: `showHistory` returns when `lesson === null`, the panel renders only
when `lesson !== null`, and uninstalling the active version clears `lesson`.
After removal the user therefore cannot reach the retained events from the
Library. The event contains an assessment snapshot, but the panel currently
gets question text and Objective labels from the installed lesson.

Evidence: `apps/desktop/src/App.tsx` history/uninstall branches,
`apps/desktop/src/components/HistoryPanel.tsx`, and Store history query in
`crates/osmium-store/src/lib.rs`.

**Required before freeze:** either provide a history view that can render
orphaned events from their snapshots, or change the removal promise and make
the retention/retrieval behavior explicit. Prefer preserving user-visible
history.

### P2 — Multi-file source save is validated first but not committed atomically

Candidate files are validated before source writes, which prevents invalid
edits from being written. The accepted files are then persisted with separate
`fs::write` calls. A process exit or disk error partway through can leave a
partially updated source spanning manifest, entity JSON, and Markdown.

Evidence: `save_workspace` in
`crates/osmium-package/src/authoring/workspace.rs`, candidate validation before
the final writes loop.

This was not fault-injected during the E2E run. Treat atomic replacement or a
recoverable journal as a reliability follow-up; add a failure test before
depending on multi-file save for important authoring work.

### P2 — Delete behavior is safe against dangling references, but is implicit

Entity deletion is represented by removing an item from the workspace
snapshot. Core validates the complete candidate before any writes, and the
existing Concept deletion test confirms a dangling Objective reference is
rejected without changing the source. The user must manually repair references
before saving a deletion; there is no delete-impact preview or cascade. The
same rejection/no-write behavior is not directly tested for Objective deletion
with Curriculum/Resource/Assessment references.

This is safe by default and does not create a dangling graph on successful
save. Before freeze, extend tests to Objective, Resource, Curriculum, and
Assessment deletion, including the relationships each deletion can invalidate.

### P2 — Round-trip preservation is plausible by construction, but pressure
coverage is too narrow

Existing entity JSON objects are cloned and known fields updated in place, and
the current test verifies a Resource-level `extensions` value survives a title
edit. The test does not exercise Concept, Objective, Curriculum, Assessment,
manifest extensions, Reference/Evidence metadata, or provenance. These are
important because the editor intentionally does not expose them.

The schema rejects unknown properties at several nested Assessment structures;
those are not valid extension locations. Test the declared extension points
and currently preserved top-level Reference/provenance values, then verify an
export round-trip as well as a source save.

### P2 — Digest-scoped progress has clear code semantics, but exact reinstall
restoration is not a direct regression test

Learning events store package ID, version, digest, and an assessment snapshot.
Progress is keyed by `(package_digest, objective_id)`, so a changed distribution
gets a separate projection; the test `new_package_version_does_not_reinterpret_old_attempts`
confirms the new version begins at zero while v1 history remains. Uninstall
marks the package unavailable without deleting its package row or events, so
reinstalling the identical digest should restore the old projection. The code
supports this, but no test explicitly performs answer → uninstall → reinstall
same artifact → verify progress/history.

Record this as a Store semantic/ADR before adding update or migration behavior:
**progress identity is distribution digest plus Objective ID; no cross-digest
progress migration occurs automatically.**

## Confirmed post-implementation behavior

- Package/version selection flows explicitly from Library card through IPC to
  Runtime. E2E opens v0.1.0 and v0.2.0 separately; it does not infer “latest”.
- Package ID/version conflicts with a different digest are rejected. New
  versions coexist; old attempts are not reinterpreted.
- Prerequisite cycles and dangling relations are rejected by Core validation.
- Distribution generation has canonical/reproducible Package-level tests;
  Desktop E2E exports a `.osmium` file and imports it through Runtime. Exact
  digest equality between a GUI export and a separately invoked CLI build is
  not asserted by the GUI E2E.
- Library loading/empty/error/retry, Assessment authoring, and learner
  restart/history restoration have UI/unit or E2E coverage.
- No graph editor, YAML write support, automatic update, Hub/publisher model,
  AI authoring, SRS, or schema identity split was added by this phase.

## Release recommendation

Do not tag `v0.1.0-alpha.1` from this working tree yet. Resolve the two P1
findings, add regression coverage for the P2 semantics that will be relied on,
then rerun the full Rust/Desktop/E2E suite and repeat the corrupt-library,
Resource-delete artifact, and uninstall-history scenarios. Keep the original
inventory as the before snapshot and update this post report with the retest
result.
