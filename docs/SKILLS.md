# Skills as authoring workflows

Skills are agent-readable SOPs above the CLI/MCP capability layer. They record inputs, goal, capability discovery, procedure, hard constraints, quality heuristics, output contract, validation, and graceful-degradation behavior. They should be model-neutral and avoid fixed word/question counts or mandatory use of a specific search tool.

The initial reference workflow is [`research-and-create-resource`](../skills/research-and-create-resource/SKILL.md). It now separates three things explicitly, because conflating them was the most common authoring mistake:

```text
Reference            — a knowledge resource a learner or distribution can reach
Evidence             — the relation from a Resource/Assessment to its References
Authoring Provenance — inputs and history used to produce the lesson (.osmium/)
```

The rule the Skill states in its own words: reading a PDF to write a lesson is not the same act as publishing that PDF as the lesson's evidence. The first belongs in `.osmium/`, the second in `manifest.references` reached from `evidence_reference_ids`. The Skill also forbids claiming claim-level verifiability, because only resource-level traceability is implemented.

Candidate future workflows are `build-curriculum`, `generate-assessments`, `review-package`, and `localize-package`. Add these only when repeated workflows need independent policy.

The capability surface a Skill may call:

```text
init, validate, lint, build, inspect, query, context
reference add | attach | list      (the only Source-writing operations)
```

Everything else is an ordinary JSON/YAML/Markdown edit. A Skill must not assume a capability it has not checked, and must not treat an unavailable search or MCP tool as a change in package semantics.
