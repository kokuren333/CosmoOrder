# Agent integration

The CLI and future MCP server are capability adapters over shared Core/Package operations. Skills describe workflow and policy; they do not replace tools. Models reason over content and evidence. Agent capabilities (search, browser, filesystem, shell, MCP) are runtime facts and must not alter Package semantics.

Authoring starts by inventorying actually available capabilities. Source acquisition preference: user-provided, package-contained, accessible local files, then available search/browser. If evidence remains unavailable, avoid unsupported claims and return structured missing-source information. Research and authoring can be split across agents by passing normalized source records, language, claims/evidence, and unresolved needs.

Long-lived authoring evaluations should be independent of a particular generation prompt/model. A future fixture benchmark can score concept coverage, resource completeness, grounding, language consistency, assessment quality, redundancy, source traceability, private-source leakage, schema validity, and lint diagnostics. Current pressure-test fixtures are examples, not a benchmark runner.
