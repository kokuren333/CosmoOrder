# References, Evidence and Authoring Provenance

Status: development contract for Package format 0.1. The Reference registry and
the Evidence relation are optional, so packages written before this change stay
readable.

## Terminology

The single word "Source" was doing too much work. It now has five separate
names, and each names a different thing.

| Term | Meaning |
| --- | --- |
| **Reference** | A knowledge resource that a learner or a distribution can reach: a guideline, a paper, a textbook, an official document, a packaged excerpt. |
| **Evidence** | The relation from a Resource or an Assessment to the References that support its content. Evidence is a *link*, not a record. |
| **Authoring Provenance** | Inputs and history used to *produce* the package: a local PDF, a private URL, an author note, an agent workflow log. This is never medical or scholarly evidence. |
| **Citation** | A reference to a Reference from a specific place in the body text. Not implemented; see below. |
| **Locator** | The URL or package-relative path that reaches a Reference. |
| **Asset** | A file carried inside a Package (`kind: package_asset`). |

"Source" survives only as the legacy field name (`manifest.sources`,
`source_ids`). Nothing writes it any more.

## Reference model

`manifest.references` is the package-level registry. Each record has a stable
`id`, a `kind` (`url`, `doi`, `isbn`, `citation`, `local_file`, `package_asset`,
`manual`) and a `visibility`. The record describes a *resource*, never a claim
about whether a statement is true or licensed.

```json
{
  "id": "nice-cg174",
  "kind": "url",
  "type": "guideline",
  "title": "成人の体液状態評価（NICE CG174）",
  "locator": "https://www.nice.org.uk/guidance/cg174/chapter/recommendations",
  "citation": "英国国立医療技術評価機構（NICE）.「成人入院患者の静脈内輸液療法」臨床指針CG174・推奨事項.",
  "publisher": "NICE",
  "version": "CG174",
  "visibility": "public",
  "record_visibility": "public",
  "locator_visibility": "public"
}
```

### `kind` is not `type`

`kind` is the *addressing* form: how a reader resolves the record. `type` is the
*material* kind. Mixing them was a known weakness of the first draft, so both
exist now and neither replaces the other:

| `kind` | Resolves through |
| --- | --- |
| `url` | an absolute URL locator |
| `doi` / `isbn` | an identifier, conventionally in the locator |
| `citation` | human-readable citation text only |
| `local_file` | a path that exists only on the author's machine; authoring-only |
| `package_asset` | a validated package-relative path |
| `manual` | no locator and no citation required |

| `type` | Meaning |
| --- | --- |
| `webpage`, `article`, `book`, `guideline`, `dataset`, `document`, `other` | the material the reader is being pointed at |

### Time axis

Web pages and guidelines are updated, so the schema carries optional
`published_at`, `updated_at`, `accessed_at`, `edition` and `version` alongside
`publisher`, `authors` and `identifiers` (`doi`, `isbn`). Values are free-form
strings: Osmium does not own a date grammar and does not infer a date from a
locator. An author who does not know a date leaves it out; an absent date is
honest, an invented one is not.

### Records still to be split

The current `kind` enum is retained for compatibility. The direction is a
canonical shape where identity, material type, bibliographic metadata and
locators are separate groups:

```yaml
reference:
  type: webpage | article | book | guideline | dataset | document | other
  title: ...
  publisher: ...
  authors: [...]
  published_at: ...
  updated_at: ...
  accessed_at: ...
  edition: ...
  version: ...
  identifiers:
    doi: ...
    isbn: ...
  locators:
    - type: url
      value: ...
```

`identifiers` already exists in that shape. `locators` as a list does not, and
is deliberately not introduced in this phase: one addressable locator per record
is what every current fixture needs, and a list without a consumer would be
guesswork. A future revision can add `locators` and derive the singular
`locator` from its first entry.

## Evidence

Resource and Assessment each carry `evidence_reference_ids`. The link is
resource-level: it says "this lesson draws on these references". It does **not**
say which sentence or paragraph draws on which reference.

```json
{
  "id": "findings.lesson",
  "path": "content/findings.md",
  "teaches": ["findings.interpret"],
  "evidence_reference_ids": ["merck-volume-depletion", "nice-cg174"]
}
```

An Assessment uses the same field, which makes the evidence for an item
auditable independently of the resource that teaches it.

## Visibility: record visibility and locator visibility

The old `visibility` enum answered two independent questions with one value.
Both axes now exist as separate fields, with the enum kept as a compatibility
view that must stay consistent with them.

| legacy `visibility` | `record_visibility` | `locator_visibility` | Distribution |
| --- | --- | --- | --- |
| `public` | `public` | `public` | record and locator |
| `attribution_only` | `public` | `hidden` | record and citation only |
| `private` | `private` | absent | nothing; the record is removed |

Precedence when both spellings are present: if either axis says private, the
record is private. Otherwise, if either says the locator is hidden, the locator
is hidden. `crates/osmium-core/src/reference.rs` implements this in one place and
`crates/osmium-core/tests/reference.rs` pins the mapping.

`private` is a legacy escape hatch, not a supported place to keep authoring
input. It exists so an old package can still load and still be sanitized on
build. New authoring input belongs in the Authoring Workspace, outside the
package directory entirely — see [`AUTHORING_WORKSPACE.md`](AUTHORING_WORKSPACE.md).

## Distribution boundary

Build removes, in this order:

1. every Reference record that is not record-public, and every Evidence ID that
   pointed at one;
2. the locator of every record whose locator is not public;
3. the bytes of every `package_asset` whose locator is not public;
4. the legacy unclassified `source` and free-form `provenance` values on
   Resources.

The distribution reader then rejects a built archive that still contains a
private record or a hidden-locator record that kept its locator. The Runtime
projects only learner-visible fields, and only for records the current Resource
actually cites. Three independent places apply the boundary on purpose: the
builder, the reader and the DTO projector.

The Desktop Reader shows a closed **References / 参考資料** disclosure for the
References that Resource cites. Public URLs render as inert text, because the
Desktop has no approved external-link opener; the view creates no anchor and
never navigates the WebView. A Resource whose only evidence is hidden shows the
record and its citation, never the URL.

## URL validation

The first draft rejected any query string. That was wrong: `?id=123`,
`?lang=ja`, `?article=foo` and `?q=fluid+balance` are ordinary publication
queries and rejecting them pushed authors to hide real, useful locators.

What is rejected today, for a locator that distributes at all:

- an authority with embedded user information (`https://user:secret@host/...`);
- a query parameter whose name is a credential or a signature, including
  `token`, `access_token`, `refresh_token`, `id_token`, `api_key`, `apikey`,
  `key`, `secret`, `client_secret`, `password`, `auth`, `signature`, `sig`,
  `sas`, and anything starting with `x-amz-` (which covers `X-Amz-Signature`,
  `X-Amz-Credential` and `X-Amz-Security-Token`);
- a parameter name ending in `_token`, `-token`, `_secret` or `_key`;
- percent-encoded spellings of the above (`access%5Ftoken`).

This is a heuristic, not a guarantee. It will not catch an opaque parameter such
as `?s=...` that happens to be a secret, and it is documented that way on
purpose: claiming complete secret detection would be false. The check is a
guardrail against committing a signed URL, not a security control.

Credential checks apply even to locators that will be stripped from the
distribution. A hidden or private locator still lives in the source manifest
and may be committed, so recognizable credentials and signed-query parameters
are rejected before visibility sanitization. Keep genuinely private locators in
the `.osmium/` Authoring Workspace instead.

## Reuse policy

A field that exists but says nothing is worse than an absent field, because both
a reader and a tool treat presence as information. `"license": "再利用条件は未設定。"`
satisfied the old field-presence rule while carrying no meaning.

The model is now:

```json
{ "license_status": "unknown" }
```

```json
{ "license_status": "known", "license": "CC BY 4.0" }
```

`license_status` is `known`, `unknown`, or `unspecified`. `license_is_known` in
`crates/osmium-core/src/reference.rs` accepts a resource only when the status is
`known` *and* the license text is non-empty and is not placeholder prose
(`未設定`, `未定`, `unknown`, `TBD`, `N/A`). Lint (`OSM_LINT_LICENSE_UNKNOWN`)
reports everything else. It is a warning, never a validation error: an author
may legitimately publish with an unresolved reuse question, and the repository's
own fixtures do exactly that.

## Resource-level traceability and claim-level verifiability

These are different levels of assurance and the documentation must not conflate
them.

**Resource-level traceability — implemented.** A Resource or Assessment names
the References it draws on, those References travel in the distribution, and the
learner can see them. A reader can check that a lesson is *associated* with the
right guidance.

**Claim-level verifiability — not implemented.** Nothing today says that a
particular sentence is supported by a particular Reference. A package can carry
perfect references and still contain an unsupported claim between them.

The intended future shape is a typed Citation in the Content IR:

```markdown
血清Na濃度は体内Na総量そのものを表さない。[^ref:nice-cg174]
```

```text
Markdown
  → Content IR (Citation / EvidenceAnchor / ReferenceMark)
  → Runtime Renderer
```

`EvidenceAnchor` would pin the claim to a span in the compiled IR,
`ReferenceMark` would carry the Reference ID, and the Renderer would decide how
to present it. Adding that requires a Content IR node, a compile-time resolution
step against the Reference registry, and a Renderer decision — none of which
exist. Until then, describe an Osmium package as *traceable at resource level*,
never as *claim-verified*.

## Local inputs

A local file consulted during authoring is not a package locator. Its absolute
path must not enter the package directory at all, because the package directory
is usually a Git repository and a committed path is a leak.

- If the file is authoring input only: record it in the Authoring Workspace
  (`.osmium/`), which is never part of a package and is gitignored.
- If the file should travel: copy it into a Package-owned relative path and
  register it as `package_asset` with a validated forward-slash relative locator
  and a `content_hash`.
- `kind: local_file` with `visibility: private` remains readable for old
  packages and is removed at build, but it is no longer the recommended place to
  put authoring input.

CLI does not fetch URLs and does not copy local files.

## CLI, MCP and the authoring API

`osmium reference add`, `osmium reference attach` and `osmium reference list`
exist because the end-to-end authoring benchmark showed that a Reference and the
Evidence relation pointing at it had to be edited in two separate JSON documents,
and nothing checked that they agreed until the whole package was re-read. Those
three operations are the only part of the CLI that writes to a Source. There is
no general CRUD surface and there will not be one.

Everything else remains an ordinary JSON/YAML/Markdown edit, validated by
`osmium validate` and reviewed with `osmium lint`. `osmium-core` owns schema and
semantic validation, `osmium-package` owns filesystem snapshots, distribution
sanitization and the three authoring edits, `osmium-store` owns the runtime
projection. Future MCP tools should be thin adapters over these same operations
and DTOs, not a second validator.

## Compatibility and migration

Old → new:

| Old | New | Note |
| --- | --- | --- |
| `manifest.sources` | `manifest.references` | Both are read. A package that has `sources` keeps using it until migrated; tooling never creates both. |
| Resource/Assessment `source_ids` | `evidence_reference_ids` | Both are read. The canonical name wins when both are present, so a half-migrated file cannot fall back to stale IDs. |
| `visibility` | `record_visibility` + `locator_visibility` | The enum is still required on a record and still authoritative for a package that never adopted the axes. |
| `license: "<placeholder>"` | `license_status: unknown` | Lint now measures meaning, not presence. |

A package does not have to migrate. Every fixture except `arithmetic`,
`arithmetic-expanded` and the legacy-path tests uses the canonical names, and
`crates/osmium-package/tests/distribution.rs` keeps a legacy package building
end-to-end so the compatibility path stays exercised.

The registry is never renamed by tooling, only by an author, so a half-migrated
package cannot end up with two registries and two sets of IDs.

Format 0.1 is not declared stable. New required semantics should wait for
evidence from further packages.
