---
project: project04-ecl-pipeline-engine
status: historical-unverified
depends-on: []
blocks: []
related: ["project03-mcp-guidance-resources"]
release-branch: release/0.4.x
release-tip: 1a248c455f85435c38324b3861a3b6f7594a0974
---

# ECL pipeline engine

> Historical project record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

This is a reconstruction of historical planning, not a new implementation project. Empty dependency fields mean no new planning dependency has been assigned by this migration. Release lineage follows Git history; it does not prove the plans were delivered. Only source-backed plans, prompts and criteria are reconstructed. No acceptance, closing report or independent verification is backfilled.

## Release boundary

`0.3.0..0.4.2`; branch `release/0.4.x`.

- 3de2b93 docs: Add research doc on Rust workflow/pipeline ecosystem (2026)
- e0a8372 docs: Add ECL pipeline redesign research and crate analysis
- 78c7f65 docs: Rust quality pass on ECL pipeline runner design (0017 v1.1)
- ac368b5 Formatting for ASCII diagram.
- bed4b16 State transitions.
- c59aca0 docs: Add Phase 1 milestone implementation plans (1.1, 1.2, 1.3)
- 5ab2b77 docs: Add Phase 2-4 milestone implementation plans (2.1-4.2)
- 0f43630 docs: Add Phase 5-6 milestone implementation plans (5.1, 5.2, 6.1)
- 92b30f8 feat(ecl-pipeline-spec): Implement milestone 1.1 — specification layer
- 3281a81 feat(ecl-pipeline-state): Implement milestone 1.2 — state layer types
- a75ea66 feat(ecl-pipeline-topo): Implement milestone 1.3 — topology layer
- 9075b24 feat(ecl-pipeline-state): Add RedbStateStore for crash-safe persistence (milestone 2.1)
- d1940fa docs: Add comprehensive Fabryk MCP server howto
- 228e780 Diagram formatting.
- 4ed25ef fix: Migrate serde_yaml references to yaml_serde
- fdc8776 Deps bump.
- 668593d Bumped textrynum version for curent dev cycle.
- d4344b3 fix(ci): Upgrade actions/checkout v4 to v6 for Node.js 24 support
- 9d635c2 fix(fts): Prevent panics on multi-byte UTF-8 in snippet extraction
- 5e2a215 Fix UTF-8 char boundary panic in find_snippet()
- 45e7f0c Add with_names()/with_descriptions() to all MCP tool registries
- 3d89c34 Re-export ErrorCode and AnnotateAble from fabryk_mcp::model
- ea6be15 Add serde backward compatibility for older graph JSON formats
- 3da7dd0 fix(fabryk-fts): Use floor_char_boundary to prevent UTF-8 panics in snippet truncation
- ae63f44 Added research on the current state of theorem provers.
- ac0811d feat: Add shared config infrastructure to fabryk-cli
- d371d43 feat(ecl-pipeline): Pipeline runner core (milestone 2.2)
- c7b946c feat(ecl-pipeline-topo): Implement full topology resolution (milestone 2.3)
- 35f5aff feat(ecl-adapter-fs, ecl-stages): Filesystem adapter and built-in stages (milestone 3.1)
- 041c8ce feat(ecl-pipeline): Integration test suite (milestone 3.2)
- c635867 feat(ecl-adapter-gdrive): Google Drive adapter — auth & enumerate (milestone 4.1)
- 0636803 feat(ecl-adapter-gdrive): Google Drive adapter — fetch & content download (milestone 4.2)
- f6562ad Log service state transitions at appropriate severity levels
- e96c266 feat(ecl-cli): Pipeline CLI commands — run, resume, status, inspect, items, diff (milestone 5.1)
- aa4130b feat(ecl-pipeline): Observability & reporting — tracing spans, timing, Display impls (milestone 5.2)
- ff731f6 feat(ecl-adapter-slack): Slack stub adapter validates SourceAdapter abstractions (milestone 6.1)
- 32eedda feat(ecl-adapter-zapier): Zapier webhook push source adapter (milestone 7.1 + 7.2)
- e092e34 feat(ecl-stages): Record type & CSV parse stage (Affinity Phase 1, milestone 7.1)
- 659de70 feat(ecl-stages): Field mapping stage (Affinity Phase 1, milestone 7.2)
- bf0b0f0 feat(ecl-adapter-gcs): GCS source adapter (Affinity Phase 1, milestone 7.3)
- f47ac77 feat(ecl-stages): Validation stage (Affinity Phase 1, milestone 7.4)
- 548d578 feat(ecl-sink-kafka): Kafka sink stage with Avro + Schema Registry (Affinity Phase 1, milestone 7.5)
- ad1a3d1 feat(ecl-sink-gcs): GCS sink stage for error/rejected item output (Affinity Phase 1, milestone 7.6)
- f3f45b8 feat(ecl-pipeline): File lifecycle management for GCS pipelines (Affinity Phase 1, milestone 7.7)
- 898b16c feat(ecl-pipeline): Affinity end-to-end integration tests (Phase 1, milestone 7.8)
- 6a18ce4 feat(ecl-pipeline): Named data streams for multi-stream routing (Phase 2, milestone 8.1)
- 1f43851 docs: Iterate MCP discoverability guide for better agent-driven results
- 3bebdec Formatting.
- ec23872 feat(ecl-stages): Batch stage trait and JoinStage (Phase 2, milestone 8.2)
- ffc448a feat(ecl-stages): AggregateStage for batch grouping and aggregation (Phase 2, milestone 8.3)
- 24875bb feat(ecl-stages): LookupStage for static value mapping (Phase 2, milestone 8.4)
- 6301204 feat(ecl-stages): DateParseStage and TimezoneStage (Phase 2, milestone 8.5)
- 5ff9ff4 feat(ecl-stages): DecompressStage for ZIP/GZIP extraction (Phase 2, milestone 8.6)
- bcf4a09 feat(ecl-stages): AssembleStage for receipt assembly (Phase 2, milestone 8.7)
- 4d164e4 feat(ecl-pipeline): Giant Eagle end-to-end integration tests (Phase 2, milestone 8.8)
- e903288 feat(ecl-secrets): Secret management foundation (Phase 3, milestone 9.1)
- b47aae2 feat(ecl-adapter-sftp): SFTP source adapter (Phase 3, milestone 9.2)
- c31f637 Deps update.
- d9977b8 Formatting.
- b0dc53e fix: Collapse nested if-let statements into let-chains (clippy)
- b95d63a chore: Bump outdated dependencies
- 0276c66 fix(ci): Install libcurl-dev for rdkafka-sys librdkafka build
- 84e9511 fix(ci): Restore protobuf-compiler for lance-encoding under --all-features
- 541a652 chore: Add missing package metadata across all crates
- 0d2c17b fix(deps): Bump redis from 1.0 to 1.1
- 675fea2 feat(fabryk-mcp-core): Add domain model, ID conventions, backend status, task strategies, and filter summary to DiscoverableRegistry
- 4d08fd2 Bumped micro version for contributions from another project.
- d97b041 Updated lock file.
- 4f086ee fix(fabryk-mcp-core): Add properties field to empty_input_schema for MCP client compatibility
- 51553d5 Bumped minor version number.
- 1a248c4 chore: Bump russh 0.46 → 0.54, russh-keys 0.46 → 0.49

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 1-80. Original bytes are recoverable from import commit 3a39aea. -->

---
number: 18
title: "ECL Pipeline Runner — Multi-Phase Project Plan"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-03-13
updated: 2026-03-13
state: Active
supersedes: 4
superseded-by: null
version: 1.0
---

# ECL Pipeline Runner — Multi-Phase Project Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development
> (if subagents available) or superpowers:executing-plans to implement each
> milestone. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Important:** This is a **project plan**, not an implementation plan. Each
> milestone below should be expanded into a detailed implementation plan (with
> TDD steps, exact file paths, and code) before execution. Use
> superpowers:writing-plans on each milestone individually.

**Goal:** Build a CLI pipeline runner for the ECL/Fabryk ecosystem that
extracts documents from external sources (Google Drive, Slack, etc.),
normalizes them to markdown, and emits them for downstream knowledge synthesis
— with checkpointing, incrementality, and full observability.

**Architecture:** Three-layer design (Specification / Topology / State) across
four core crates (`ecl-pipeline-spec`, `ecl-pipeline-topo`,
`ecl-pipeline-state`, `ecl-pipeline`) plus adapter and stage crates. The
pipeline runner executes stages in resource-scheduled parallel batches,
checkpoints to redb after each batch, and supports crash-resume. State is the
single source of truth — serializable, inspectable, and resumable.

**Design Doc:** `crates/design/docs/05-active/0017-ecl-pipeline-runner-unified-design-vision.md`

**Tech Stack:** Rust 2024 (edition), tokio, serde, toml, redb, blake3, backon,
async-trait, chrono, clap, tracing, serde_bytes

---

## Phase Overview

| Phase | Milestone | Focus | Crate(s) | Outcome |
|-------|-----------|-------|----------|---------|
| 1 | **1.1** | Specification Layer | `ecl-pipeline-spec` | TOML config parses, validates, and round-trips through serde |
|| **1.2** | State Layer Types | `ecl-pipeline-state` | State/checkpoint types serialize; `InMemoryStateStore` works |
|| **1.3** | Topology Layer Types | `ecl-pipeline-topo` | Resource graph builds schedule; traits (`SourceAdapter`, `Stage`) defined |
| 2 | **2.1** | RedbStateStore | `ecl-pipeline-state` (extend) | Persistent checkpoint storage with atomic writes via redb |
|| **2.2** | Pipeline Runner Core | `ecl-pipeline` | Runner executes batches with retry, concurrency, and checkpointing |
|| **2.3** | Topology Resolution | `ecl-pipeline-topo` + `ecl-pipeline` (extend) | Spec resolves to wired topology via adapter/stage registries |
| 3 | **3.1** | Filesystem Adapter + Stages | `ecl-adapter-fs`, `ecl-stages` | End-to-end pipeline with local files (extract, normalize, filter, emit) |
|| **3.2** | Integration Test Suite | `ecl-pipeline` (tests) | Comprehensive scenarios: resume, incrementality, errors, fan-in |
| 4 | **4.1** | Google Drive — Auth & Enumerate | `ecl-adapter-gdrive` | OAuth2 auth, folder traversal, pagination, filtering |
|| **4.2** | Google Drive — Fetch & Pipeline | `ecl-adapter-gdrive` (extend) | Content download, format export, full Drive-to-markdown pipeline |
| 5 | **5.1** | CLI Commands | `ecl-cli` (extend) | `run`, `resume`, `status`, `inspect`, `items`, `diff` commands |
|| **5.2** | Observability & Reporting | `ecl-pipeline` + `ecl-pipeline-state` (extend) | Tracing spans, structured JSON output matching design doc section 6 |
| 6 | **6.1** | Slack Stub Validation | `ecl-adapter-slack` | Second adapter proves abstractions hold without trait changes |

---

## Workspace Dependencies to Add

Before any milestone begins, these workspace dependencies need to be added to
the root `Cargo.toml` under `[workspace.dependencies]`:

```toml
redb = "3"
serde_bytes = "0.11"
```

`blake3`, `async-trait`, `backon`, `tokio`, `serde`, `serde_json`, `toml`,
`chrono`, `thiserror`, `tracing`, `clap`, `tempfile`, and `proptest` are
already in the workspace.

---


Historical section moved to [Phase 1: Foundation (Types & Serialization)](arc01-foundation/arc-plan.md).

Historical section moved to [Phase 2: Engine (Runner & Persistence)](arc02-engine/arc-plan.md).

Historical section moved to [Phase 3: First Adapter (Filesystem — Test Harness)](arc03-filesystem/arc-plan.md).

Historical section moved to [Phase 4: Real Adapter (Google Drive)](arc04-google-drive/arc-plan.md).

Historical section moved to [Phase 5: CLI & Polish](arc05-cli-observability/arc-plan.md).

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 534-649. Original bytes are recoverable from import commit 3a39aea. -->

## Phase 6: Validation

### Milestone 6.1: Second Adapter Validation (Slack Stub)

**Goal:** Add a minimal Slack adapter stub to validate that the
abstractions hold. If adding Slack requires changing trait signatures,
the abstractions are wrong.

**Crate:** `crates/ecl-adapter-slack/`

**Key files to create:**

- `Cargo.toml`
- `src/lib.rs` — `SlackAdapter` implementing `SourceAdapter`
- `src/enumerate.rs` — stub enumerate (returns hardcoded items or reads from fixture)
- `src/fetch.rs` — stub fetch (returns fixture content)

**Key deliverables:**

- [ ] `SlackAdapter` implements `SourceAdapter` without trait changes
- [ ] TOML config with `kind = "slack"` resolves correctly
- [ ] Pipeline runs with both filesystem and Slack sources in same config
- [ ] If any trait change is needed: document it and update design doc
- [ ] Register crate as workspace member

**Test strategy:**

- Stub adapter with fixture data
- Mixed-source pipeline (filesystem + Slack stub)
- Verify no trait signature changes were needed
- If changes were needed, document what and why

---

## Explicitly Deferred (Design Doc Section 11)

These are documented in the design doc as future work and are NOT in scope
for this project plan:

- **AI-assisted stages** — concept extraction, classification via LlmProvider
- **Fabryk integration** — direct API integration (emit stage writes files Fabryk can consume)
- **ACL / multi-tenancy** — deferred to Fabryk's fabryk-acl timeline
- **Orchestrator / hub-and-spoke MCP** — orthogonal concern
- **Web UI / monitoring dashboard** — `inspect` + Claude is the observability story
- **Condition expression language** — `ConditionExpr` is a string wrapper for now;
  evaluator TBD (stages with conditions are always-run until evaluator exists)
- **Fan-in aggregation stage implementations** — the trait supports it but no
  built-in aggregation stage exists yet (the `emit` stage is the closest)

---

## Cross-Cutting Concerns

These apply to every milestone and should be verified at each review checkpoint:

### Code Quality

- All crates: `#[lints.rust] unsafe_code = "forbid"`, `#[lints.clippy] unwrap_used = "deny"`
- Follow patterns from `ecl-core/Cargo.toml` for lint configuration
- No `unwrap()` in library code — use `?` or `.ok_or()`
- `&str` not `&String`, `&[T]` not `&Vec<T>` in function params
- All public types implement `Debug` (and `Clone` where appropriate)
- All serializable types use `BTreeMap` (not `HashMap`) for deterministic output

### Testing

- Test naming: `test_<fn>_<scenario>_<expectation>`
- Coverage target: >= 95% per crate
- Property tests for spec parsing (proptest)
- All error paths tested

### Documentation

- Doc comments on all public items
- Module-level docs explaining purpose
- Example in doc comments for key types

### Commits

- One commit per logical change
- Reference design doc in commit messages where relevant
- Format: `feat(ecl-pipeline-spec): add PipelineSpec TOML parsing`

---

## Dependency Summary

| Milestone | Creates | Depends On |
|-----------|---------|------------|
| 1.1 | `ecl-pipeline-spec` | workspace deps only |
| 1.2 | `ecl-pipeline-state` | `ecl-pipeline-spec` |
| 1.3 | `ecl-pipeline-topo` | `ecl-pipeline-spec` |
| 2.1 | (extends state) | `ecl-pipeline-state` |
| 2.2 | `ecl-pipeline` | spec + topo + state |
| 2.3 | (extends topo + pipeline) | all core crates |
| 3.1 | `ecl-adapter-fs`, `ecl-stages` | `ecl-pipeline` |
| 3.2 | (integration tests) | all crates |
| 4.1 | `ecl-adapter-gdrive` | `ecl-pipeline-topo` (for traits) + HTTP deps |
| 4.2 | (extends gdrive) | `ecl-adapter-gdrive` |
| 5.1 | (extends `ecl-cli`) | `ecl-pipeline` |
| 5.2 | (extends pipeline + state) | `ecl-pipeline` |
| 6.1 | `ecl-adapter-slack` | `ecl-pipeline-topo` (for traits) |

---

## Estimated Scope

- **Phase 1:** ~800-1000 lines of Rust (types + tests)
- **Phase 2:** ~1200-1500 lines (runner + persistence + tests)
- **Phase 3:** ~600-800 lines (fs adapter + stages + integration tests)
- **Phase 4:** ~800-1000 lines (Drive adapter + auth + tests)
- **Phase 5:** ~500-700 lines (CLI + tracing)
- **Phase 6:** ~200-300 lines (Slack stub + validation)

**Total:** ~4100-5300 lines — within the design doc's "~1000-2000 lines of
custom Rust atop proven crates" target when you subtract test code.

## Reconstructed slice locations

- [Milestone 6.1: Slack Stub Validation (`ecl-adapter-slack`)](slice01-slack-stub-validation/slice-plan.md)
- [Zapier Webhook Adapter — Implementation Plan](slice02-zapier-webhooks/slice-plan.md)

## Reconstructed arc locations

- [Pipeline Phase 1 — Specification, state and topology](arc01-foundation/arc-plan.md)
- [Pipeline Phase 2 — Persistence and execution engine](arc02-engine/arc-plan.md)
- [Pipeline Phase 3 — Filesystem pipeline](arc03-filesystem/arc-plan.md)
- [Pipeline Phase 4 — Google Drive](arc04-google-drive/arc-plan.md)
- [Pipeline Phase 5 — CLI and observability](arc05-cli-observability/arc-plan.md)
- [Banyan Phase 1 — Make Affinity work](arc06-affinity/arc-plan.md)
- [Banyan Phase 2 — Make Giant Eagle work](arc07-giant-eagle/arc-plan.md)
- [Banyan Phase 3 — Make Acme Inc. work](arc08-acme-inc/arc-plan.md)
- [Banyan Phase 4 — Full parity](arc09-full-parity/arc-plan.md)

## Historical supporting documents

- [0001-ecl-pipeline-redesign-2026.02.13-session-bootstrap.md](artifacts/0001-ecl-pipeline-redesign-2026.02.13-session-bootstrap.md)
- [0002-pipeline-crate-deep-analysis-claude-code-prompts.md](artifacts/0002-pipeline-crate-deep-analysis-claude-code-prompts.md)
- [0003-deep-analysis-dagrs.md](artifacts/0003-deep-analysis-dagrs.md)
- [0004-deep-analysis-erio-workflow.md](artifacts/0004-deep-analysis-erio-workflow.md)
- [0005-deep-analysis-oxigdal-workflow.md](artifacts/0005-deep-analysis-oxigdal-workflow.md)
- [0006-deep-analysis-dagx.md](artifacts/0006-deep-analysis-dagx.md)
- [0007-deep-analysis-dagga.md](artifacts/0007-deep-analysis-dagga.md)
- [0008-deep-analysis-rettle.md](artifacts/0008-deep-analysis-rettle.md)
- [0009-pipeline-crate-synthesis.md](artifacts/0009-pipeline-crate-synthesis.md)
- [0010-ecl-pipeline-runner-trait-based-design-sketch.md](artifacts/0010-ecl-pipeline-runner-trait-based-design-sketch.md)
- [0011-pipeline-crate-survey-analysis-synthesis.md](artifacts/0011-pipeline-crate-survey-analysis-synthesis.md)
- [0016-research-2026-state-of-rust-ecosystem-for-workflowpipeline-frameworks.md](artifacts/0016-research-2026-state-of-rust-ecosystem-for-workflowpipeline-frameworks.md)
- [0017-ecl-pipeline-runner-unified-design-vision.md](artifacts/0017-ecl-pipeline-runner-unified-design-vision.md)
- [0019-theorem-provers-for-rust-based-mcp-integration.md](artifacts/0019-theorem-provers-for-rust-based-mcp-integration.md)
- [0026-banyan-data-pipeline-requirements-analysis.md](artifacts/0026-banyan-data-pipeline-requirements-analysis.md)
- [0027-ecl-banyan-pipeline-implementation-plan.md](artifacts/0027-ecl-banyan-pipeline-implementation-plan.md)
- [RELEASE-NOTES-0.4.0.md](artifacts/RELEASE-NOTES-0.4.0.md)
- [fabryk-mcp-implementation-guide-ai-kasu.md](artifacts/fabryk-mcp-implementation-guide-ai-kasu.md)
- [howto-analysis-notes.md](artifacts/howto-analysis-notes.md)
- [mcp-server-implementation-taproot.md](artifacts/mcp-server-implementation-taproot.md)

- [analysis-dagrs.md](artifacts/analysis-dagrs.md)

- [analysis-erio-workflow.md](artifacts/analysis-erio-workflow.md)

- [analysis-oxigdal-workflow.md](artifacts/analysis-oxigdal-workflow.md)

- [analysis-dagx.md](artifacts/analysis-dagx.md)

- [analysis-dagga.md](artifacts/analysis-dagga.md)

- [analysis-rettle.md](artifacts/analysis-rettle.md)

## Reconstruction history

- 2026-11-21: Reorganized original documents under the Operator-requested release/project mapping. Source identities, workbench mtimes, duplicate decisions and exact split ranges are recorded in [migration-map.json](../migration-map.json). This records documentary provenance, not implementation acceptance.

Original criteria are also available in the [historical ledger](ledger.md).

## Current scope ownership — 2026-11-21

Remaining pipeline/CLI and later data-pipeline milestones belong to
project06-pipeline, which is deferred. Authorization and multi-tenancy required
by those efforts also belong to project06. These historical release records
remain here to preserve what was planned and what evidence actually exists;
this transfer does not assert that an old milestone was delivered or accepted.
