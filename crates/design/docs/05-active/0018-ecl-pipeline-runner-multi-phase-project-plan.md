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

## Phase 1: Foundation (Types & Serialization)

The goal of Phase 1 is to establish the type system for all three layers. No
execution logic — just types that compile, serialize, deserialize, and
round-trip correctly. This is the foundation everything else builds on.

### Milestone 1.1: Specification Layer (`ecl-pipeline-spec`)

**Goal:** TOML config parses into Rust types and round-trips through
serde correctly.

**Crate:** `crates/ecl-pipeline-spec/`

**Key files to create:**

- `Cargo.toml` — workspace member with serde, toml, serde_json, chrono, thiserror deps
- `src/lib.rs` — `PipelineSpec`, re-exports
- `src/source.rs` — `SourceSpec` enum (GoogleDrive, Slack, Filesystem variants), `CredentialRef`, `FilterRule`, `FileTypeFilter`
- `src/stage.rs` — `StageSpec`, `ResourceSpec`
- `src/defaults.rs` — `DefaultsSpec`, `RetrySpec`, `CheckpointStrategy` with `Default` impls and serde default functions
- `src/validation.rs` — spec-level validation (duplicate stage names, empty sources, etc.)
- `src/error.rs` — `SpecError` enum with thiserror

**Key deliverables:**

- [ ] All spec types derive `Debug, Clone, Serialize, Deserialize`
- [ ] `BTreeMap` used for all maps (deterministic serialization)
- [ ] `#[serde(tag = "kind")]` on `SourceSpec`, `#[serde(tag = "type")]` on `CredentialRef`
- [ ] `serde_json::Value` for `StageSpec.params` (not `toml::Value`)
- [ ] `#[serde(default)]` on all optional fields with sensible defaults
- [ ] Round-trip test: parse example TOML from design doc, serialize to JSON, deserialize back, assert equality
- [ ] Validation tests: duplicate stage names, missing source references, empty pipeline
- [ ] Register crate as workspace member in root `Cargo.toml`

**Test strategy:**

- Unit tests for each type's serde round-trip
- Test the full example TOML from design doc section 1
- Property tests (proptest) for RetrySpec bounds
- Validation error tests

---

### Milestone 1.2: State Layer Types (`ecl-pipeline-state`)

**Goal:** State types compile and serialize. StateStore trait defined.
Checkpoint round-trips through JSON. No persistence implementation yet.

**Crate:** `crates/ecl-pipeline-state/`

**Key files to create:**

- `Cargo.toml` — depends on `ecl-pipeline-spec`, plus redb, blake3, chrono, serde, serde_json, serde_bytes, thiserror
- `src/lib.rs` — re-exports
- `src/types.rs` — `PipelineState`, `SourceState`, `ItemState`, `ItemStatus`, `StageState`, `StageStatus`, `PipelineStatus`, `PipelineStats`, `CompletedStageRecord`, `ItemProvenance`
- `src/ids.rs` — `RunId`, `StageId`, `Blake3Hash` newtypes with `new()`, `as_str()`, `Display`
- `src/checkpoint.rs` — `Checkpoint` struct, `prepare_for_resume()`, `config_drifted()`
- `src/store.rs` — `StateStore` trait (async_trait), `StateError`
- `src/memory_store.rs` — `InMemoryStateStore` (for tests)
- `src/error.rs` — `StateError` enum

**Key deliverables:**

- [ ] All state types derive `Debug, Clone, Serialize, Deserialize`
- [ ] `StageId`, `RunId`, `Blake3Hash` are newtypes with private internals
- [ ] `Checkpoint.prepare_for_resume()` resets Processing items to Pending
- [ ] `Checkpoint.config_drifted()` compares spec hashes
- [ ] `InMemoryStateStore` implements `StateStore` trait
- [ ] `PipelineState::new()` constructor from topology info
- [ ] `PipelineState::update_stats()` recomputes aggregate statistics
- [ ] Round-trip test: create state, serialize to JSON, deserialize, assert equality
- [ ] Test `prepare_for_resume` with items in various states
- [ ] Register crate as workspace member

**Test strategy:**

- Checkpoint round-trip through JSON
- `prepare_for_resume` correctness (Processing -> Pending, Completed unchanged)
- `config_drifted` with matching and non-matching hashes
- `InMemoryStateStore` save/load cycle
- `update_stats` aggregation correctness

---

### Milestone 1.3: Topology Layer Types (`ecl-pipeline-topo`)

**Goal:** Topology types compile. Resource graph builds from spec and
computes a parallel schedule. Missing input detection works.

**Crate:** `crates/ecl-pipeline-topo/`

**Key files to create:**

- `Cargo.toml` — depends on `ecl-pipeline-spec`, plus blake3, tokio, async-trait, thiserror, tracing
- `src/lib.rs` — `PipelineTopology`, re-exports
- `src/resolve.rs` — topology resolution logic (spec -> topology)
- `src/resource_graph.rs` — `ResourceGraph`, constraint building, validation
- `src/schedule.rs` — topological sort, layer computation, batch splitting
- `src/traits.rs` — `SourceAdapter` trait, `Stage` trait, `StageContext`, `PipelineItem`, `SourceItem`, `ExtractedDocument`
- `src/error.rs` — `ResolveError`, `SourceError`, `StageError`

**Key deliverables:**

- [ ] `SourceAdapter` trait: `source_kind()`, `enumerate()`, `fetch()` — object-safe with async_trait
- [ ] `Stage` trait: `name()`, `process(item, ctx) -> Result<Vec<PipelineItem>>` — object-safe with async_trait
- [ ] `StageContext` with immutable `Arc<PipelineSpec>`, output_dir, params, tracing span
- [ ] `PipelineItem` with `Arc<[u8]>` content, serde_bytes, metadata map
- [ ] `RetryPolicy` type (resolved, `Duration`-based form of `RetrySpec`) — used in `ResolvedStage`
- [ ] `ConditionExpr` type (string wrapper for now; expression evaluator is deferred — see design doc section 11)
- [ ] `ResourceGraph::build()` from stage specs
- [ ] `ResourceGraph::validate_no_cycles()` via topological sort
- [ ] `ResourceGraph::compute_schedule()` producing `Vec<Vec<StageId>>`
- [ ] Missing input detection (warn, don't fail — externals are implicit)
- [ ] Test: design doc example produces expected 3-batch schedule
- [ ] Test: cycle detection catches circular resource dependencies
- [ ] Test: independent resources land in same batch
- [ ] Register crate as workspace member

**Test strategy:**

- Schedule computation with the design doc's 5-stage example
- Cycle detection (A creates X, B reads X and creates Y, A reads Y)
- Independent stages in same batch
- Single-stage batches for fully sequential pipelines
- Empty pipeline produces empty schedule
- Missing input warning for typo'd resource names
- Property tests (proptest): random valid resource graphs produce valid schedules

---

## Phase 2: Engine (Runner & Persistence)

The goal of Phase 2 is to build the execution engine: the runner that
orchestrates batches, the redb persistence layer, and retry logic.

### Milestone 2.1: RedbStateStore

**Goal:** Persistent state storage via redb with atomic checkpoint writes.

**Crate:** `crates/ecl-pipeline-state/` (extend)

**Key files to create/modify:**

- Create: `src/redb_store.rs` — `RedbStateStore` implementing `StateStore`

**Key deliverables:**

- [ ] `RedbStateStore::open(path)` — opens/creates redb database
- [ ] `save_checkpoint()` — atomic write of serialized checkpoint
- [ ] `load_checkpoint()` — load most recent checkpoint
- [ ] `load_previous_hashes()` — load content hashes from last completed run
- [ ] `save_completed_hashes()` — save hashes at end of successful run
- [ ] Table design: `checkpoints` table (run_id -> bytes), `hashes` table (item_id -> hash)
- [ ] Test: save and load checkpoint round-trip through redb
- [ ] Test: multiple checkpoints, load returns most recent
- [ ] Test: crash simulation (drop store mid-write, reopen, verify last good checkpoint)

**Test strategy:**

- Round-trip checkpoint through redb (save, close, reopen, load)
- Hash persistence across "runs" (save hashes, load in new session)
- Concurrent read safety (not needed yet, but verify redb handles it)
- Use tempfile for test databases

---

### Milestone 2.2: Pipeline Runner Core

**Goal:** The runner executes a pipeline with batch-based scheduling,
stage execution, and checkpointing. Uses InMemoryStateStore for testing.

**Crate:** `crates/ecl-pipeline/`

**Key files to create:**

- `Cargo.toml` — depends on spec, topo, state, plus tokio, tracing, backon, async-trait, thiserror
- `src/lib.rs` — `PipelineRunner`, re-exports of all pipeline types
- `src/runner.rs` — `PipelineRunner` struct, `new()`, `run()`, `execute_batch()`, `enumerate_sources()`, `checkpoint()`
- `src/retry.rs` — `execute_with_retry()` using backon
- `src/stage_exec.rs` — `execute_stage_items()` with semaphore-bounded concurrency
- `src/error.rs` — `PipelineError` enum

**Key deliverables:**

- [ ] `PipelineRunner::new(spec)` — resolves topology, opens state store, loads checkpoint if available
- [ ] `PipelineRunner::run()` — enumerate, apply incrementality, execute batches, finalize
- [ ] Batch execution: stages in a batch run concurrently via `JoinSet`
- [ ] Item execution: bounded concurrency within a stage via semaphore
- [ ] Retry with exponential backoff via `backon`
- [ ] `skip_on_error` support at item level
- [ ] Checkpoint after each batch
- [ ] Resume: skip completed batches, reset stuck items
- [ ] Config drift detection on resume
- [ ] Test: full pipeline with mock stages (InMemoryStateStore + mock adapters)
- [ ] Test: resume after simulated crash (save checkpoint, create new runner, verify skip)
- [ ] Test: retry behavior (stage fails N-1 times, succeeds on Nth)
- [ ] Test: skip_on_error (item failure doesn't abort pipeline)
- [ ] Register crate as workspace member

**Test strategy:**

- Mock `SourceAdapter` that returns fixed items
- Mock `Stage` that transforms items (append suffix to content)
- Mock `Stage` that fails N times then succeeds (retry test)
- Full pipeline: enumerate -> fetch -> transform -> emit
- Resume: run half, checkpoint, create new runner, run rest
- Concurrency: verify semaphore limits concurrent tasks

---

### Milestone 2.3: Topology Resolution (Wire It Together)

**Goal:** `PipelineTopology::resolve()` takes a `PipelineSpec` plus a
stage/adapter registry and produces a fully wired topology.

**Crate:** `crates/ecl-pipeline-topo/` (extend) and `crates/ecl-pipeline/` (extend)

**Key files to modify:**

- `ecl-pipeline-topo/src/resolve.rs` — implement full resolution logic
- `ecl-pipeline/src/lib.rs` — adapter/stage registry, `register_adapter()`, `register_stage()`

**Key deliverables:**

- [ ] Adapter registry: map of `kind` string -> adapter factory function
- [ ] Stage registry: map of adapter string -> stage factory function
- [ ] `resolve()` resolves sources to adapters, stages to handlers
- [ ] Merge stage-level retry overrides with global defaults
- [ ] Resolve timeout, skip_on_error, condition from spec
- [ ] Test: resolve the design doc example with mock adapters/stages
- [ ] Test: unknown adapter kind produces clear error
- [ ] Test: missing source reference produces clear error

**Test strategy:**

- Register mock adapters/stages, resolve full spec
- Error cases: unknown adapter, missing source, invalid resource
- Verify merged retry policy (stage override > global default)

---

## Phase 3: First Adapter (Filesystem — Test Harness)

The filesystem adapter is the simplest possible adapter. It turns a local
directory into a source, letting us test the full pipeline end-to-end without
network calls or credentials.

### Milestone 3.1: Filesystem Adapter + Built-in Stages

**Goal:** `ecl-adapter-fs` enumerates/fetches local files. `ecl-stages`
provides extract, normalize (passthrough), and emit stages. Full end-to-end
pipeline works with local files.

**Crates:** `crates/ecl-adapter-fs/`, `crates/ecl-stages/`

**Key files to create:**

- `ecl-adapter-fs/Cargo.toml`
- `ecl-adapter-fs/src/lib.rs` — `FilesystemAdapter` implementing `SourceAdapter`
- `ecl-stages/Cargo.toml`
- `ecl-stages/src/lib.rs` — re-exports, stage registration
- `ecl-stages/src/extract.rs` — `ExtractStage` (delegates to SourceAdapter)
- `ecl-stages/src/normalize.rs` — `NormalizeStage` (passthrough for PoC; later: format conversion)
- `ecl-stages/src/filter.rs` — `FilterStage` (glob-based include/exclude filtering)
- `ecl-stages/src/emit.rs` — `EmitStage` (writes PipelineItem content to output_dir)

**Key deliverables:**

- [ ] `FilesystemAdapter::enumerate()` — walks directory, returns `SourceItem` per file
- [ ] `FilesystemAdapter::fetch()` — reads file content, computes blake3 hash
- [ ] Filter support: extension filter, glob patterns
- [ ] `ExtractStage::process()` — calls adapter.fetch() for each item
- [ ] `NormalizeStage::process()` — passes through content (placeholder)
- [ ] `FilterStage::process()` — applies glob include/exclude rules, returns empty vec for excluded items
- [ ] `EmitStage::process()` — writes content to `output_dir/{subdir}/{filename}`
- [ ] End-to-end test: create temp dir with files, write TOML config, run pipeline, verify output files
- [ ] Test: incrementality — run twice, second run skips unchanged files
- [ ] Test: modified file detected and re-processed
- [ ] Register both crates as workspace members

**Test strategy:**

- Create tempdir with 5 text files
- Write a TOML config pointing at it
- Run pipeline, verify output files match input
- Modify one file, run again, verify only that file is re-processed
- Add a new file, run again, verify it's picked up
- Delete a file, run again, verify it's not in output

---

### Milestone 3.2: Integration Test Suite

**Goal:** Comprehensive integration tests exercising the full pipeline
through realistic scenarios.

**Crate:** `crates/ecl-pipeline/` (integration tests)

**Key files to create:**

- `crates/ecl-pipeline/tests/integration/` — integration test directory
- `tests/integration/full_pipeline.rs` — end-to-end with filesystem adapter
- `tests/integration/checkpoint_resume.rs` — crash and resume scenarios
- `tests/integration/incrementality.rs` — cross-run hash comparison
- `tests/integration/error_handling.rs` — retry, skip_on_error, stage failure

**Key deliverables:**

- [ ] Happy path: 10 files, 3 stages, verify all items completed
- [ ] Checkpoint/resume: interrupt after batch 0, resume, verify batch 1+ runs
- [ ] Incrementality: run 1 processes all, run 2 skips unchanged
- [ ] Error handling: one file causes stage failure, skip_on_error continues
- [ ] Concurrent stages: two independent extract stages run in parallel
- [ ] Fan-in aggregation: stage reading multiple resources receives combined item set
- [ ] State inspection: verify JSON output matches expected structure

---

## Phase 4: Real Adapter (Google Drive)

### Milestone 4.1: Google Drive Adapter — Auth & Enumerate

**Goal:** Authenticate with Google Drive API and enumerate files in a
folder. No fetching yet.

**Crate:** `crates/ecl-adapter-gdrive/`

**Key files to create:**

- `Cargo.toml` — depends on ecl-pipeline-topo (for `SourceAdapter` trait), plus reqwest, serde, serde_json, tokio, yup-oauth2 (or google-authz)
- `src/lib.rs` — `GoogleDriveAdapter` struct, SourceAdapter impl
- `src/auth.rs` — OAuth2 token management (service account + user credentials)
- `src/enumerate.rs` — Drive Files.list API, pagination, folder traversal
- `src/types.rs` — Drive API response types

**Key deliverables:**

- [ ] `CredentialRef::File` resolves to service account JSON
- [ ] `CredentialRef::EnvVar` resolves to token from environment
- [ ] `CredentialRef::ApplicationDefault` uses ADC
- [ ] `enumerate()` calls Drive Files.list with folder filter
- [ ] Pagination: handles `nextPageToken` for large folders
- [ ] Recursive folder traversal (respects root_folders config)
- [ ] Filter application: file_types, glob patterns, modified_after
- [ ] `SourceItem.source_hash` populated from Drive's `md5Checksum`
- [ ] Test: mock HTTP responses for enumerate (use wiremock or similar)
- [ ] Register crate as workspace member

**Test strategy:**

- Mock Drive API responses with wiremock
- Test pagination (multiple pages)
- Test filter application (file type, glob, modified_after)
- Test auth credential resolution (file, env, ADC)
- Error handling: invalid credentials, API errors, rate limits

---

### Milestone 4.2: Google Drive Adapter — Fetch & Full Pipeline

**Goal:** Fetch document content from Drive. Run a full pipeline:
Drive folder -> extract -> normalize -> emit to local files.

**Files to modify:**

- `ecl-adapter-gdrive/src/fetch.rs` — content download, format detection
- `ecl-stages/src/normalize.rs` — extend with basic format handling

**Key deliverables:**

- [ ] `fetch()` downloads file content via Drive Files.get with `alt=media`
- [ ] Google Docs export: `application/vnd.google-apps.document` -> markdown via export API
- [ ] PDF handling: extract text (basic, via external tool or crate)
- [ ] Compute blake3 hash of fetched content
- [ ] Rate limiting: respect Drive API quotas (429 handling via retry)
- [ ] `NormalizeStage` handles plain text and markdown passthrough
- [ ] Test: mock HTTP for fetch, verify content and hash
- [ ] Integration test: filesystem adapter simulating Drive output structure

**Test strategy:**

- Mock HTTP for document download
- Test Google Docs export MIME type mapping
- Test content hash computation
- Rate limit handling (429 -> retry with backoff)
- End-to-end with mocked HTTP server

---

## Phase 5: CLI & Polish

### Milestone 5.1: CLI Commands

**Goal:** `ecl pipeline run|resume|status|inspect|items` commands work.

**Crate:** `crates/ecl-cli/` (extend)

**Key files to create/modify:**

- Modify: `src/main.rs` — add pipeline subcommand group
- Create: `src/pipeline.rs` — pipeline CLI handler
- Create: `src/pipeline/run.rs` — `ecl pipeline run <config.toml>`
- Create: `src/pipeline/resume.rs` — `ecl pipeline resume [--force] <output-dir>`
- Create: `src/pipeline/inspect.rs` — `ecl pipeline inspect <output-dir>` (full JSON state)
- Create: `src/pipeline/status.rs` — `ecl pipeline status <output-dir>` (human-readable summary)
- Create: `src/pipeline/items.rs` — `ecl pipeline items <output-dir>` (item listing)
- Create: `src/pipeline/diff.rs` — `ecl pipeline diff <dir1> <dir2>` (compare two runs)

**Key deliverables:**

- [ ] `ecl pipeline run <config.toml>` — parse TOML, run pipeline, print summary
- [ ] `ecl pipeline resume <output-dir>` — load checkpoint, resume execution
- [ ] `ecl pipeline resume --force <output-dir>` — resume despite config drift
- [ ] `ecl pipeline inspect <output-dir>` — pretty-print full state as JSON
- [ ] `ecl pipeline status <output-dir>` — human-readable one-line-per-stage summary
- [ ] `ecl pipeline items <output-dir>` — table of items with status, source, hash
- [ ] `ecl pipeline diff <dir1> <dir2>` — compare two runs (new/changed/removed items, stage outcome differences)
- [ ] Structured tracing output during execution (stage started/completed/failed)
- [ ] Exit codes: 0 = success, 1 = failure, 2 = partial (some items failed)
- [ ] Test: CLI integration tests using assert_cmd or similar

**Test strategy:**

- CLI flag parsing tests
- End-to-end: write TOML, run CLI, verify output dir
- Inspect output matches expected JSON structure
- Status output is human-readable
- Resume after ctrl-c simulation

---

### Milestone 5.2: Observability & Reporting

**Goal:** Pipeline state output is rich enough for Claude to analyze.
Tracing integration provides structured execution logs.

**Files to modify:**

- `ecl-pipeline/src/runner.rs` — add tracing spans and events
- `ecl-pipeline-state/src/types.rs` — add timing fields, improve Display impls

**Key deliverables:**

- [ ] Tracing spans: one per batch, one per stage, one per item
- [ ] Structured fields: stage_id, item_id, duration_ms, status, error
- [ ] `PipelineState` JSON output matches design doc section 6 exactly
- [ ] Timing: per-stage started_at/completed_at, per-item duration_ms
- [ ] Stats: totals computed and cached in `PipelineStats`
- [ ] Human-readable Display impl for `PipelineStatus`, `ItemStatus`, `StageStatus`
- [ ] Test: verify JSON output structure matches design doc example

---

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
