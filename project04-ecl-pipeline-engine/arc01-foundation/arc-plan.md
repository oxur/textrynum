# Pipeline Phase 1 — Specification, state and topology

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 81-210. Original bytes are recoverable from import commit 3a39aea. -->

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


## Reconstructed slice locations

- [Milestone 1.1: Specification Layer (`ecl-pipeline-spec`)](slice01-specification-layer/slice-plan.md)
- [Milestone 1.2: State Layer (`ecl-pipeline-state`)](slice02-state-layer/slice-plan.md)
- [Milestone 1.3: Topology Layer (`ecl-pipeline-topo`)](slice03-topology-layer/slice-plan.md)
