# Sources and provenance

Status: development contract for Package format 0.1. Source records are optional so existing packages remain readable.

## Record and relationship

`manifest.sources` is the package-level registry. Records use a stable `id`, one of `url`, `doi`, `isbn`, `citation`, `local_file`, `package_asset`, or `manual`, and a `visibility` of `public`, `attribution_only`, or `private`. `title`, `citation`, `locator`, and a SHA-256 content hash are optional metadata. Resource `source_ids` identifies evidence for that teaching resource; Assessments may also refer to sources. No acquisition-tool identity is required. The record describes evidence, not whether a claim is true or licensed.

`manifest.language` is the default BCP 47 content language. Resource `language` optionally overrides it. UI locale is presentation preference and never translates or selects different content. Assessment prompt language follows the Package default. Concept titles currently follow the package language. Multiple localized Resources can teach the same Objective and carry their own language/source references.

## Visibility and distribution boundary

- `public`: distributable title and locator; public locators must be portable and must not contain a query string.
- `attribution_only`: distributable identifying metadata/citation, with locator removed during build.
- `private`: authoring provenance only. The entire record and each Resource reference to it are removed from the built Package.

The distribution reader rejects private records, attribution-only locators, and legacy unclassified Resource `source`/`provenance` values. The legacy free-form provenance object has no portability/privacy contract, so build removes it rather than risk leaking author paths or private values.

The Desktop Reader shows a closed **References / 参考資料** disclosure for sources referenced by that Resource only. The shared Runtime projects `public` title, citation, and locator, and `attribution_only` title and citation; private records are omitted before the Desktop DTO is created. Attribution-only locators are removed at both the Runtime and DTO boundaries. Public URLs are rendered as inert text because the Desktop currently has no approved external-link opener; the reference view does not create an anchor or navigate the WebView. A Resource with no learner-visible sources has no disclosure, so private-only provenance is not signaled by a count or empty heading.

## Local inputs and portable locators

A local file can be consulted during authoring but its absolute path is not a Package locator. Record it as `local_file`, `private`, with optional content hash and non-sensitive title. If the file should travel, copy it into a Package-owned relative asset and register it as `package_asset` with a validated forward-slash relative locator. Build rejects unsafe source locators for distributable records, including OS absolute paths and query strings. Do not put credentials in source URLs. CLI does not fetch URLs or copy local source files.

The authoring boundary is: acquisition input → normalized source metadata → visibility decision → resource-specific source IDs → sanitized distribution. Research method (search, browser, shell, MCP, human paste) is separate from the portable record.

## CLI, MCP, and authoring API

Current CLI commands provide `init`, `validate`, `lint`, `build`, `inspect`, `query`, `context`, and local `install/read`. Package edits remain ordinary JSON/YAML/Markdown files; there is no `source add` command yet. `osmium-core` owns schema and semantic validation, while `osmium-package` owns filesystem snapshots and distribution sanitization. Future MCP tools should be thin adapters over these same operations and DTOs, not a second validator or format implementation. A shared application-operation facade can be extracted when the first MCP adapter is added.

## Compatibility

The manifest source registry and Resource `language`/`source_ids` are optional. Existing `source`, creator, license, attribution, and provenance fields remain parseable for authoring compatibility. The builder removes ambiguous legacy `source` and free-form `provenance` values from distributions. New required semantics should wait for evidence from additional packages; format 0.1 is not declared stable.
