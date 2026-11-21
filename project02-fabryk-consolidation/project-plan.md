---
project: project02-fabryk-consolidation
status: historical-unverified
depends-on: []
blocks: []
related: ["project01-ecl-fabryk-foundations"]
release-branch: release/0.2.x
release-tip: 08503430f20fcb4dea5ef2e68dfc25bb5821944f
---

# Fabryk consolidation and lifecycle

> Historical project record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

This is a reconstruction of historical planning, not a new implementation project. Empty dependency fields mean no new planning dependency has been assigned by this migration. Release lineage follows Git history; it does not prove the plans were delivered. Only source-backed plans, prompts and criteria are reconstructed. No acceptance, closing report or independent verification is backfilled.

## Release boundary

`0.1.2..0.2.0`; branch `release/0.2.x`.

- 932b7e4 feat(fabryk-mcp): Improve DiscoverableRegistry and add metadata how-to
- ae4540a Added async startup HOWTO.
- fe98809 Bumped version for current dev cycle.
- d111e63 feat: Service lifecycle improvements — parallel wait, retry, health router, audit trail
- 0d92313 feat(fabryk-mcp-semantic): Add VectorSlot for deferred backend wiring
- 5eb7a39 Formatting.
- eba8ca7 feat: Make health_router generic over state type + add health howto
- c55f63f feat(textyl): Support any Rust project with --project-version/--deps-version
- 062d25c Formatting.
- 6589bed feat(fabryk-mcp): Add MCP schema validation and fix empty inputSchema
- e4c6820 fix(mcp): Clarify concepts_get id parameter is a slug from list results
- 1945aa6 feat(fabryk-cli): Add vectordb get-model command with vector-fastembed feature
- dc8e37a docs: Add fabryk MCP + Claude Code integration howto
- 640f800 Formatting.
- 8e998e9 feat(fabryk-gcp): Add GCP credential detection crate + log_error_chain
- 74032b0 fix: Serialize env-var tests with mutex, reduce log_error_chain noise
- f142357 chore: Trim dependency tree, upgrade rmcp to 1.1, bump to v0.2.0
- ccb078c refactor: Restructure fabryk crate hierarchy for better downstream UX
- bba8729 Updated formatting.
- 7447adb fix(ci): Replace arduino/setup-protoc with apt to avoid GitHub API rate limits
- b2fe4a1 docs: Move READMEs and images into crate directories
- 8dd9072 fix: Serialize env-var tests in fabryk-cli with mutex
- 23f750d docs: Move howtos to docs/howtos/ and add README section
- 04d1f2c docs: Add AI prompt for updating architecture diagrams
- 0850343 docs: Update architecture diagram for v0.2.0 restructure

<!-- Overview excerpt from slice01-crate-hierarchy/slice-plan.md. -->

## Context

Downstream users currently face a fragmented dependency experience. The goal is two clean entry points:

- `fabryk` — core knowledge fabric (auth, content, fts, graph, vector, acl)
- `fabryk-mcp` — MCP tools facade (all MCP sub-crates)

Vendor-specific crates (fabryk-gcp, fabryk-auth-google) remain separate opt-in.


<!-- Overview excerpt from slice01-crate-hierarchy/slice-plan.md. -->

## Key Decisions

- **Create `fabryk-mcp-core`** to resolve circular dependency (mirrors fabryk/fabryk-core pattern)
- **Rename `fabryk-auth-mcp` → `fabryk-mcp-auth`** for consistent naming
- **fts/graph/vector are always-on** in the facade; backend features remain gated
- **fabryk-acl included** in facade; fabryk-redis excluded


## Reconstructed slice locations

- [Restructure Fabryk Crate Hierarchy for Better Downstream UX](slice01-crate-hierarchy/slice-plan.md)

## Historical supporting documents

- [RELEASE-NOTES-0.2.0.md](artifacts/RELEASE-NOTES-0.2.0.md)
- [TODO.md](artifacts/TODO.md)
- [mcp-server-as-knowledge-source.md](artifacts/mcp-server-as-knowledge-source.md)

## Reconstruction history

- 2026-11-21: Reorganized original documents under the Operator-requested release/project mapping. Source identities, workbench mtimes, duplicate decisions and exact split ranges are recorded in [migration-map.json](../migration-map.json). This records documentary provenance, not implementation acceptance.

Original criteria are also available in the [historical ledger](ledger.md).
