# Package Identity

Status: v0.1 compatibility note; no schema change proposed for the current
Authoring work.

## v0.1 behavior

The stable v0.1 package schema requires `package_id` and gives it a
`namespace/name` syntax. Runtime and Store use the package ID together with the
package version to identify an installed package version; a conflicting
content digest for the same pair is rejected. Package distribution preserves
the ID. The learner-facing title is separate and can change without changing
the ID.

Desktop users do not choose this identifier. The package authoring API
generates an immutable, collision-resistant ID when it scaffolds a source.
Desktop may show and copy the ID in Advanced details, but must not offer an
ordinary rename operation for it. A future Duplicate/Fork action should create
a new identity rather than change the identity of an installed package.

## Known limitation

The current identifier combines runtime identity with namespace/name-like
syntax. That syntax does not establish ownership of a namespace. There is no
central registration or publisher verification implied by an ID such as
`org.example/course`.

Treat this behavior as a v0.1 compatibility layer, not as the final publishing
or identity model. Keep the current schema and CLI behavior stable while
Authoring is developed.

## Future consideration

If publishing or a Hub is designed, consider separating these concepts:

- **Immutable package identity**: opaque machine identifier, unchanged by title,
  slug, or publisher changes.
- **Slug**: optional human-readable name that may change.
- **Publisher identity**: optional Hub-managed publisher handle or verified
  identity; not inferred from reverse-domain text.
- **Release/version**: the package's explicit release number.
- **Content digest**: the integrity identity of a built package.

Design that model together with publishing ownership, migration, references,
and backward compatibility. Do not add these fields to v0.1 speculatively or
change the stable schema as part of routine Desktop Authoring work.
