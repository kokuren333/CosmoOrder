# Authoring Workspace

Status: development contract for Package format 0.1.

## Why a separate directory

Keeping private authoring notes in the manifest and deleting them at build time
is not a sufficient boundary. Build-time deletion protects the *archive*. It does
not protect the *repository*: a local path, a private URL or an unpublished
draft read into `manifest.sources` is already a committed secret by the time
build runs. The failure mode is a Git commit, not a distribution.

So the package directory and the authoring workspace are separate things.

```text
Distributable Package
!=
Authoring Workspace
```

```text
package/                     ← everything here may be distributed
  osmium.json
  entities/
  content/
  assets/

  .osmium/                   ← never distributed, gitignored
    authoring.json
    provenance.jsonl
    notes/
```

`osmium-package` skips `.osmium` at the top level of a Source before it
inventories anything. Its contents are not counted against the source file or
byte limits, are not path-validated as package payload, and never reach
`LoadedSource::files`, so they cannot appear in a build even by mistake.

`.osmium/` is listed in the repository `.gitignore`, so authoring input stays
out of Git by default rather than by discipline.

## What belongs there

| Category | Example |
| --- | --- |
| Local input paths | `/Users/author/papers/draft.pdf`, `D:\notes\fluid.md` |
| Private or signed URLs | an institutional repository, a pre-publication DOI with a token |
| Author-only notes | "the second paragraph of the lesson is still uncited" |
| Agent workflow metadata | which model read which input, in what order |
| Acquisition metadata | when a source was fetched, with what tool |
| Temporary research state | search result sets, comparison tables, drafts |

None of this is medical or scholarly evidence. A PDF that an agent read while
researching a lesson is *not* a Reference, and making the agent's reading of it
visible to a learner would be a category error. Conversely, a guideline that the
lesson actually rests on *is* a Reference even if an agent was the one who
fetched it.

The distinction to hold:

> Reading a PDF to write a lesson
> ≠
> publishing that PDF as the lesson's evidence

Both may be true for the same document. They are still different records, and
they live in different places.

## What the workspace is not

- It is not a second package format. Nothing in `.osmium/` is validated, versioned
  or migrated by Osmium, and no reader consumes it.
- It is not secret storage. It is a *boundary* that keeps authoring input out of
  the distributable artifact and out of version control by default. An author who
  copies a credential into `.osmium/` has still put a credential on disk.
- It is not a replacement for `visibility`. A `visibility: private` Reference is
  a legacy escape hatch for a package that must stay loadable; it is sanitized on
  build, but it was still authored *inside* the package directory.
- It is not required. A package with no authoring workspace is normal.

## Traceability

The workspace may reference a Resource so an author can find the provenance of a
lesson later. That link is authoring-side only and is never serialized into the
package. If a piece of authoring input genuinely is evidence for the content,
promote it: register it as a Reference with `osmium reference add` and attach it
with `osmium reference attach`.

The promotion rule:

```text
private input (in .osmium/)
  → author decides it is evidence
  → Reference record in the manifest (public or hidden locator)
  → Evidence relation from the Resource or Assessment
```

## Verification

`crates/osmium-package/tests/distribution.rs` writes an `.osmium/` directory
containing a local path, a credential-bearing URL and an author-only note into a
temporary copy of the provenance fixture, builds it, and asserts that no byte of
any of them appears anywhere in the distribution. Combined with the skip in
`inventory`, that is the enforced guarantee:

```text
authoring workspace contents never enter a build
authoring workspace contents never enter a runtime DTO
authoring workspace contents are gitignored by default
```

## Related

- [`SOURCES_AND_PROVENANCE.md`](SOURCES_AND_PROVENANCE.md) — the Reference and
  Evidence model the workspace feeds into.
- [`AI_AUTHORING.md`](AI_AUTHORING.md) — the agent workflow that produces and
  consumes the workspace.
