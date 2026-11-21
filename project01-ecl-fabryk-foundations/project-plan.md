---
project: project01-ecl-fabryk-foundations
status: historical-unverified
depends-on: []
blocks: []
related: []
release-branch: release/0.1.x
release-tip: ef305124c902369dcb173c4eb4936ac2220646b7
---

# ECL and Fabryk foundations

> Historical project record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

This is a reconstruction of historical planning, not a new implementation project. Empty dependency fields mean no new planning dependency has been assigned by this migration. Release lineage follows Git history; it does not prove the plans were delivered. Only source-backed plans, prompts and criteria are reconstructed. No acceptance, closing report or independent verification is backfilled.

## Release boundary

`0.1.2`; branch `release/0.1.x`.

- 4c217e2 first commit
- 27f5b1a Added project files.
- 2d32858 Prooject metadata updates.
- b0aa827 Formatting.
- da71ce6 Added project plan.
- b0dcf59 Added CLAUDE.md, etc.
- dd10f9f Added phase 1 dev plan.
- c11d612 Implemented Phase 1 Stages 1.1 and 1.2: Project scaffolding and core types
- 62e58d7 Implemented Phase 1 Stage 1.3: LLM Abstraction Layer
- a10b64a Implemented Phase 1 Stage 1.4: Basic Workflow (Pre-Restate)
- 1d2fca7 Implemented Phase 1 Stage 1.5: Feedback Loop with Bounded Iteration
- 2bd92c3 feat: Complete Stage 1.6 - Phase 1 Integration Test Suite
- b5ffa13 Added more design docs.
- 9bba4ee Add Fabryk placeholder crates for knowledge fabric implementation
- 4ffd1c6 Update PUBLISH_ORDER in Makefile for ECL and Fabryk crates
- bb1522e Add version specifications to path dependencies for publishing
- 2211212 Fix publish-dry-run to use cargo package instead of cargo publish
- 9db5fa8 Add description fields to ECL crates for crates.io publishing
- d72c19b Improve publish script to handle crates.io rate limits
- 3ed7926 Fix grep command for BSD/macOS compatibility
- ec4fdc8 Update oxur-odm dependency to version 0.3.0
- fd84e46 Finally all published.
- 7e8e0b9 Added first version of file.
- 9748d5e Updated docs.
- ddcc22c Updated docs.
- b915a5b More docs updates.
- 5eda3ac Added milestone docs from Claude Desktop.
- fe3d131 docs(fabryk): revise milestone 1.0 & 1.1 for ECL workspace approach
- fe55083 docs(fabryk): revise milestones 1.0-1.2 for consistent thiserror approach
- cc24aad docs(fabryk): add checkpoint clarification and revise milestone 1.3
- 229b5a4 docs(fabryk): revise milestones 1.4-1.7 for correct scope
- dab3814 chore(fabryk): clean up ECL workspace for Fabryk extraction
- e658ada feat(core): extract full error types from music-theory
- 61d575f docs(fabryk): add bootstrap navigation document
- ad56783 Moved BOOTSTRAP doc.
- bcb0cdf feat(core): extract file and path utilities
- a4f0da7 docs(fabryk): update bootstrap document paths and formatting
- f41b492 feat(core): add ID utilities and PathResolver
- ae93a57 feat(core): define ConfigProvider trait and generic AppState
- 92dd30a Added new dev docs on concept cards.
- 41d2011 chore(core): verify Phase 1 completion
- ef6adff Updated docs.
- e087fdf Added another workbench/scratch file.
- 47476e4 Added doc for integrating lancedb.
- 9c0ccf5 Added phases.
- a1080b8 Moved docs around.
- a410812 feat(content): implement fabryk-content crate (Phase 2)
- 9cfe23b feat(fts): implement fabryk-fts crate milestones 3.1-3.4
- 2b5b22d Renamed papers.
- 464fa14 feat(fts): implement QueryBuilder and TantivySearch backend
- df49418 style(fabryk): apply cargo fmt across all fabryk crates
- 16cc1cf feat(graph): implement fabryk-graph crate (Phase 4, milestones 4.1-4.6)
- 2497bc2 Docs updates.
- f89a37f feat(vector): implement fabryk-vector crate (Phase 4.5, milestones 4.5.1-4.5.5)
- ed45039 Updated make.
- 4af057d feat(mcp): implement fabryk-mcp core infrastructure (Phase 5.1)
- 51a76c7 feat(mcp): add fabryk-mcp-content with ContentItemProvider and SourceProvider (Phase 5.2)
- 19c07b9 feat(mcp): add fabryk-mcp-fts with search tools (Phase 5.4)
- 12525f0 feat(mcp): add fabryk-mcp-graph with graph query tools (Phase 5.5)
- 5efa7c8 feat(cli): implement fabryk-cli framework with graph commands (Phase 6)
- 8d07490 feat(cli): add configuration infrastructure with TOML config and CLI commands (Phase 6.5)
- 2373950 fix(build): run coverage across all workspace crates, not just ecl
- f5dd1e4 feat: Add fabryk-auth crates and multi-path indexing for ai-kasu
- aa0ae70 docs: update README to highlight Fabryk knowledge fabric
- fa6f63b docs: Add YAML quoting rule for aliases containing colons
- 7f9c940 feat: Add built-in caching, service lifecycle, and readiness gating
- 1ceb96e Added current architecture doc.
- 57c43c2 feat: Add ConfigManager trait and generic config handlers
- b4a8b2f fix(build): use cargo-edit for check-deps with strict semver filtering
- b50e912 chore: Remove unused restate-sdk dependency
- 085720d style: Apply rustfmt formatting
- a7b1687 Updated version in preparation for release.
- 93f3602 ci: Add GitHub Actions CI/CD workflow
- 0c5c9d9 fix(ci): Drop path override for oxur-odm, use crates.io version
- 033e134 ci: Add check-deps-cicd target for compatible-only dep checking
- 8537487 chore(deps): Upgrade blake3 1.5 → 1.8
- f5d026c feat: Rebrand project as Textrynum — The Knowledge Fabric Workshop
- 717be36 docs: Split README into top-level overview + Fabryk and ECL detail pages
- ab56c2b Updated branding.
- 63f3de3 Makefile updates.
- 6eb5648 fix(ci): Install protoc for lance-encoding build dependency
- 0282418 More badages/links.
- 315740a fix(ci): Replace broken caching with Swatinem/rust-cache, reorder steps
- cc8627b Replace ASCII architecture diagram with rendered PNG
- af71e27 fix(ci): Use runtime checks for cargo tool availability
- aa933e9 fix(ci): Force-install cargo-edit to override stale cached metadata
- 53f852e Fix spelling of 'multi-model' in README
- bd4b458 fix(make): Correct PUBLISH_ORDER to match actual workspace crates
- 8c69716 fix(publish): Add version to all Fabryk internal path dependencies
- 561da8e chore(deps): Upgrade 11 major dependencies, remove 3 unused
- 19be85a chore: Bump version to 0.1.1
- dbc1b4a feat(mcp): Add DiscoverableRegistry for MCP tool discoverability (#2)
- 80094c7 feat(mcp): Add HTTP transport support behind optional feature flag (#3)
- 39f93d4 chore(deps): Upgrade dirs from 5 to 6
- beac63b chore(deps): Upgrade async-walkdir from 1 to 2
- 24329ce chore(deps): Upgrade rkyv from 0.7 to 0.8
- a89cee1 chore(deps): Upgrade jsonwebtoken from 9 to 10
- fe5a8a8 chore(deps): Upgrade fastembed from 4 to 5
- d06d39c chore(deps): Upgrade tantivy from 0.22 to 0.25
- 6da00bd feat(util): Add textyl workspace version management utility
- 05d0e39 chore: Upgrade to Rust edition 2024, MSRV 1.85, and thiserror 2
- 808e788 fix(deps): Sync internal path dep versions from 0.1.0 to 0.1.1
- 9437d20 chore(deps): Upgrade toml from 0.9 to 1.0 and update source files
- 7ecbd2b Bumped to version 0.1.2 for the next release.
- 6cb5fa9 chore: Update Cargo.lock for textrynum-util crate
- 16cb42a feat(taproot): Add fabryk-redis crate, deploy module, and auth middleware updates
- 139bb4c feat(util): Add publish-order and exclude flags to textyl crates
- 4e8cbd5 feat(ai-kasu): Add fabryk-mcp-semantic crate and MCP diagnostics tools
- 65f892f Formatting.
- ef30512 fix: Address 3 minor code quality issues from audit

<!-- Historical source: crates/design/docs/10-superseded/0004-ecl-project-plan.md, lines 1-89. Original bytes are recoverable from import commit 3a39aea. -->

---
number: 4
title: "ECL Project Plan"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-01-22
updated: 2026-03-13
state: Superseded
supersedes: null
superseded-by: 18
version: 1.0
---

# ECL Project Plan

## Rust-Based AI Workflow Orchestration System

**Version**: 1.0
**Date**: January 2026
**Status**: Implementation Ready
**Audience**: Claude Code (Plan Mode → Implementation)

---

## Document Purpose

This project plan breaks down the ECL system into implementable phases and stages. Each phase is designed to be:

1. **Self-contained**: Produces working, testable functionality
2. **Incrementally valuable**: Builds on prior phases without requiring rework
3. **Claude Code ready**: Detailed enough for Plan Mode to generate implementation specs

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          ECL System                             │
├─────────────────────────────────────────────────────────────────┤
│  Workflow Layer         │  Restate workflows + Step trait       │
│  LLM Layer              │  llm crate (Claude provider)          │
│  Persistence Layer      │  SQLx (SQLite dev / Postgres prod)    │
│  Resilience Layer       │  backon (retry), failsafe (circuit)   │
│  Config Layer           │  confyg                               │
│  Observability Layer    │  twyg                                 │
├─────────────────────────────────────────────────────────────────┤
│  External Dependencies  │  Restate Server, Claude API, Database │
└─────────────────────────────────────────────────────────────────┘
```

---

## Reference Workflows

### Toy Workflow: "Critique-Revise Loop"

Used for Phase 1-2 validation. Simple 3-step workflow demonstrating core patterns.

| Step | Name | Input | Output | Validation |
|------|------|-------|--------|------------|
| 1 | **Generate** | Topic string | Draft text | Non-empty, <2000 chars |
| 2 | **Critique** | Draft text | Critique + pass/revise decision | Valid JSON with decision field |
| 3 | **Revise** | Draft + Critique | Final text | Bounded: max 3 revision cycles |

**Key patterns exercised**:

- Sequential step execution
- Bounded feedback loop (Step 3 → Step 2, max 3 iterations)
- Durable Promise for revision signaling
- Basic validation gates

### Target Workflow: "Codebase Migration Analyzer"

The real workflow ECL is built for. Full implementation targeted by Phase 3+.

| Step | Name | Inputs | Outputs |
|------|------|--------|---------|
| 0 | **Intake** | Code path, intended use, vision doc, deploy configs | Validated input bundle |
| 1 | **Codebase Analysis** | Input bundle | Implementation grade report, current component breakdown, recommended refactor breakdown |
| 2 | **Architecture Design** | Step 1 docs + codebase access | Language-agnostic ideal architecture doc |
| 3 | **Project Planning** | Architecture doc + target config + SKILL.md | Phased project plan (phases → stages) |
| 4 | **Implementation Specs** | Project plan + docs | Per-phase implementation docs |
| 5 | **Quality Assessment** | New code repo + docs | Standards analysis, plan adherence report |
| 6 | **Delivery** | All artifacts | Final writeup, presentation, code package |

---


Historical section moved to [Phase 1: Foundation](arc01-ecl-foundation/arc-plan.md).

Historical section moved to [Phase 2: Step Abstraction Framework](arc02-ecl-step-framework/arc-plan.md).

Historical section moved to [Phase 3: API & Tooling](arc03-ecl-api-tooling/arc-plan.md).

Historical section moved to [Phase 4: Hardening](arc04-ecl-hardening/arc-plan.md).

Historical section moved to [Phase 5: Target Workflow Implementation](arc05-ecl-target-workflows/arc-plan.md).

<!-- Historical source: crates/design/docs/10-superseded/0004-ecl-project-plan.md, lines 1717-1768. Original bytes are recoverable from import commit 3a39aea. -->

## Appendices

### Appendix A: Dependency Versions

| Crate | Version | Purpose |
|-------|---------|---------|
| restate-sdk | 0.7.x | Workflow orchestration |
| tokio | 1.x | Async runtime |
| sqlx | 0.8.x | Database access |
| llm | 0.2.x | LLM abstraction |
| backon | 1.x | Retry logic |
| failsafe | 0.x | Circuit breaker |
| confyg | 0.3.x | Configuration |
| twyg | 0.6.x | Logging |
| axum | 0.7.x | HTTP API |
| clap | 4.x | CLI |
| serde | 1.x | Serialization |
| uuid | 1.x | ID generation |
| chrono | 0.4.x | Date/time |
| thiserror | 2.x | Error handling |
| anyhow | 1.x | Error context |

### Appendix B: Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ECL_ENV` | Environment (development/production) | development |
| `ECL_LOG_LEVEL` | Log level (trace/debug/info/warn/error) | info |
| `ECL_DATABASE_URL` | Database connection URL | sqlite://ecl.db |
| `ECL_CLAUDE_API_KEY` | Anthropic API key | (required) |
| `ECL_RESTATE_URL` | Restate server URL | <http://localhost:8080> |
| `ECL_ARTIFACT_PATH` | Artifact storage base path | ./artifacts |
| `ECL_API_PORT` | HTTP API port | 3000 |
| `ECL_API_KEY` | API authentication key | (required in prod) |

### Appendix C: Success Criteria Checklist

From the original proposal:

- [ ] **Durability**: Workflow survives process restart mid-execution
- [ ] **Feedback Loops**: Step N+1 can request revision from Step N with bounded iteration
- [ ] **Observability**: Full trace of all steps, LLM calls, and decisions
- [ ] **Clean Separation**: LLM provider swappable via trait implementation
- [ ] **Developer Experience**: Simple workflow definition in Rust code

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01 | Claude | Initial project plan |

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 1-41. Original bytes are recoverable from import commit 3a39aea. -->

---
number: 13
title: "Fabryk Extraction: Project Plan Overview"
author: "increasing complexity"
component: All
tags: [change-me]
created: 2026-02-03
updated: 2026-02-03
state: Active
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Extraction: Project Plan Overview

## Overview

The Fabryk extraction project refactors the domain-agnostic infrastructure out
of the music-theory MCP server (~12,000 lines of Rust) into a set of reusable
crates. Post-extraction, ~87% of the code lives in Fabryk as shared
infrastructure; ~2,800 lines remain in ai-music-theory as domain-specific
implementations.

The extraction proceeds in seven phases, ordered by increasing complexity and
dependency depth. Each phase is decomposed into milestones — discrete units of
work, each suitable for a single Claude Code session. Milestones within a phase
are sequential; some early milestones across adjacent phases may overlap.

**Governing documents:**

- Doc 0009 — Fabryk Extraction Audit (file inventory, classifications, trait designs)
- Doc 0010 — Audit Amendment (six refinements that override parts of 0009)
- Doc 0011 — Session Bootstrap (cold-start prompt for new sessions)

**Key constraint:** The music-theory MCP server must remain fully functional
after every milestone. All 25 MCP tools must work. `cargo test --all-features`
must pass in both repos at every merge point.

---


Historical section moved to [Phase 1: fabryk-core (1–2 weeks)](arc06-fabryk-core/arc-plan.md).

Historical section moved to [Phase 2: fabryk-content (1 week)](arc07-fabryk-content/arc-plan.md).

Historical section moved to [Phase 3: fabryk-fts (2 weeks)](arc08-fabryk-search/arc-plan.md).

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 120-136. Original bytes are recoverable from import commit 3a39aea. -->

## v0.1-alpha Checkpoint

**Gate criteria (all must pass before proceeding to Phase 4):**

1. `fabryk-core`, `fabryk-content`, `fabryk-fts` compile and pass all tests
2. ai-music-theory depends on these three crates
3. All non-graph MCP tools work identically (search, content listing, guides)
4. Graph tools still work via in-repo code (unchanged)
5. `cargo test --all-features` passes in both repos
6. `cargo clippy --all-features` clean in both repos
7. Performance: no measurable regression in search latency or startup time

**Decision point:** If the extraction pattern has revealed problems, address
them before proceeding to Phase 4 (highest risk). If clean, continue.

---


Historical section moved to [Phase 4: fabryk-graph (3 weeks)](arc09-fabryk-graph/arc-plan.md).

Historical section moved to [Phase 5: fabryk-mcp + fabryk-mcp-* (1–2 weeks)](arc11-fabryk-mcp/arc-plan.md).

Historical section moved to [Phase 6: fabryk-cli (1 week)](arc12-fabryk-cli/arc-plan.md).

Historical section moved to [Phase 7: Integration & Documentation (1 week)](arc14-fabryk-integration/arc-plan.md).

<!-- Historical source: crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md, lines 242-282. Original bytes are recoverable from import commit 3a39aea. -->

## Summary

| Phase | Crate(s) | Milestones | Risk | Estimated Duration |
|-------|----------|------------|------|--------------------|
| 1 | fabryk-core | 1.1 – 1.7 | Low | 1–2 weeks |
| 2 | fabryk-content | 2.1 – 2.4 | Low | 1 week |
| 3 | fabryk-fts | 3.1 – 3.5 | Low–Med | 2 weeks |
| — | *v0.1-alpha checkpoint* | — | — | — |
| 4 | fabryk-graph | 4.1 – 4.8 | **High** | 3 weeks |
| 5 | fabryk-mcp, fabryk-mcp-* | 5.1 – 5.6 | Medium | 1–2 weeks |
| 6 | fabryk-cli | 6.1 – 6.3 | Low | 1 week |
| 7 | Integration & docs | 7.1 – 7.4 | Low | 1 week |

**Total milestones:** 37
**Total estimated duration:** 10–14 weeks (side project pace)
**v0.1-alpha checkpoint:** ~4 weeks (after Phase 3)

---

## Appendix: Milestone → CC Prompt Index

Each milestone will have a corresponding Claude Code prompt document. Prompts
are created as needed, not all upfront. The naming convention is:

```
cc-fabryk-{phase}.{milestone}-{short-name}.md
```

Examples:

```
cc-fabryk-1.1-workspace-scaffold.md
cc-fabryk-1.5-config-provider-trait.md
cc-fabryk-4.2-graph-extractor-trait.md
cc-fabryk-4.7-music-theory-extractor.md
cc-fabryk-5.2-content-source-traits.md
```

The bootstrap document (Doc 0011) should be updated with current status before
creating each prompt. The prompt itself should reference this overview document
for context on what came before and what comes after the current milestone.

## Reconstructed arc locations

- [ECL Phase 1 — Foundation](arc01-ecl-foundation/arc-plan.md)
- [ECL Phase 2 — Step abstraction framework](arc02-ecl-step-framework/arc-plan.md)
- [ECL Phase 3 — API and tooling (historical proposal)](arc03-ecl-api-tooling/arc-plan.md)
- [ECL Phase 4 — Hardening (historical proposal)](arc04-ecl-hardening/arc-plan.md)
- [ECL Phase 5 — Target workflows (historical proposal)](arc05-ecl-target-workflows/arc-plan.md)
- [Fabryk Phase 1 — Core](arc06-fabryk-core/arc-plan.md)
- [Fabryk Phase 2 — Content](arc07-fabryk-content/arc-plan.md)
- [Fabryk Phase 3 — Full-text search](arc08-fabryk-search/arc-plan.md)
- [Fabryk Phase 4 — Graph](arc09-fabryk-graph/arc-plan.md)
- [Fabryk Phase 4.5 — Vector search](arc10-fabryk-vector/arc-plan.md)
- [Fabryk Phase 5 — MCP tools](arc11-fabryk-mcp/arc-plan.md)
- [Fabryk Phase 6 — CLI](arc12-fabryk-cli/arc-plan.md)
- [Fabryk Phase 6.5 — Configuration](arc13-fabryk-config/arc-plan.md)
- [Fabryk Phase 7 — Integration and documentation](arc14-fabryk-integration/arc-plan.md)

## Historical supporting documents

- [0039-taproot-graphengine-evaluation-phase-4.md](arc09-fabryk-graph/artifacts/0039-taproot-graphengine-evaluation-phase-4.md)
- [phase-6.md](arc12-fabryk-cli/artifacts/phase-6.md)
- [0001-concept-card-framework-immediate-improvements-report.md](artifacts/0001-concept-card-framework-immediate-improvements-report.md)
- [0001-ecl-architecture-proposal.md](artifacts/0001-ecl-architecture-proposal.md)
- [0002-a-guide-for-parallel-concept-card-extraction-v3.md](artifacts/0002-a-guide-for-parallel-concept-card-extraction-v3.md)
- [0002-ecl-library-research-recommendations.md](artifacts/0002-ecl-library-research-recommendations.md)
- [0003-ecl-project-proposal.md](artifacts/0003-ecl-project-proposal.md)
- [0003-howto-concept-card-extraction-with-claude-code.md](artifacts/0003-howto-concept-card-extraction-with-claude-code.md)
- [0005-ecl-ecosystem-vision-summary.md](artifacts/0005-ecl-ecosystem-vision-summary.md)
- [0006-fabryk-project-proposal.md](artifacts/0006-fabryk-project-proposal.md)
- [0007-fabryk-file-management-addendum.md](artifacts/0007-fabryk-file-management-addendum.md)
- [0008-fabryk-mcp-project-proposal.md](artifacts/0008-fabryk-mcp-project-proposal.md)
- [0009-unified-ecosystem-vision-ecl-fabryk-skill-framework.md](artifacts/0009-unified-ecosystem-vision-ecl-fabryk-skill-framework.md)
- [0010-knowledge-graph-query-capabilities-demo.md](artifacts/0010-knowledge-graph-query-capabilities-demo.md)
- [0011-claude-code-music-theory-mcp-server-extraction-audit.md](artifacts/0011-claude-code-music-theory-mcp-server-extraction-audit.md)
- [0011-fabryk-extraction-audit-music-theory-mcp-server.md](artifacts/0011-fabryk-extraction-audit-music-theory-mcp-server.md)
- [0012-fabryk-extraction-audit-amendment-refinements.md](artifacts/0012-fabryk-extraction-audit-amendment-refinements.md)
- [0012-fabryk-extraction-session-bootstrap-phases-1-7.md](artifacts/0012-fabryk-extraction-session-bootstrap-phases-1-7.md)
- [0013-lancedb-integration-guide-for-rust-mcp-servers.md](artifacts/0013-lancedb-integration-guide-for-rust-mcp-servers.md)
- [0014-fabryk-architecture-2026-02-24.md](artifacts/0014-fabryk-architecture-2026-02-24.md)
- [meta-prompt-generate-phase-milestones.md](artifacts/meta-prompt-generate-phase-milestones.md)
- [workbench-BOOTSTRAP.md](artifacts/workbench-BOOTSTRAP.md)
- [workbench-PROMPTS.md](artifacts/workbench-PROMPTS.md)
- [workbench-mcp-orchestration-best-practices-audit.md](artifacts/workbench-mcp-orchestration-best-practices-audit.md)
- [workbench-mcp-orchestration-best-practices.md](artifacts/workbench-mcp-orchestration-best-practices.md)

## Reconstruction history

- 2026-11-21: Reorganized original documents under the Operator-requested release/project mapping. Source identities, workbench mtimes, duplicate decisions and exact split ranges are recorded in [migration-map.json](../migration-map.json). This records documentary provenance, not implementation acceptance.

Original criteria are also available in the [historical ledger](ledger.md).
