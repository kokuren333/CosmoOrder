# Desktop Authoring and Package Lifecycle

Status: implementation note for the v0.1 compatible authoring surface.

## Authoring

`osmium_package::authoring::workspace` exposes typed drafts for Concept,
Objective, Resource, Curriculum, and Assessment. The Desktop renderer sends
those drafts through Tauri IPC; it never writes entity JSON or allocates stable
entity IDs. Package maps draft keys to IDs, mutates known fields on existing
JSON objects, preserves fields outside the editor, writes Markdown payloads,
and validates the candidate source through `load_source` before saving.

The schema relation remains:

```text
Concept <- Objective <- Resource.teaches
                   <- Assessment.measures
Curriculum.objectives orders Objective references
Concept.requires relates prerequisite Concepts
```

YAML source remains read-only until the shared Package layer has a serializer.
Package ID remains the v0.1 `namespace/name` runtime identity. New Desktop
sources use the package init API's generated ID; no schema fields were added
for publisher, slug, or immutable opaque identity.

## Distribution and install

Authoring export delegates to `osmium_package::distribution::build`. Desktop
import delegates archive verification and installation to `Runtime::install`,
which reads the existing `.osmium` format, checks Store identity conflicts,
and calls `Library::install_distribution`. Source is never passed to Learner:
the learner opens only an installed, digest-verified distribution.

## Removal

Uninstall removes one `(package_id, package_version)` payload from Library.
Store event history is append-only and is intentionally retained; progress is
digest-scoped and becomes available again if the same digest/version is
reinstalled. A new version is not selected automatically or substituted for a
removed version. Damaged payloads remain visible in a bounded management
inventory when their distribution manifest still identifies the package, so
they can be removed without weakening the verification path for reads.

CLI `osmium uninstall <package-id> --package-version <version>` and Desktop
Library both call `Runtime::uninstall`.
