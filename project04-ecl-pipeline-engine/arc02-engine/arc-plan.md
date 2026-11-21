# Pipeline Phase 2 — Persistence and execution engine

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 211-321. Original bytes are recoverable from import commit 3a39aea. -->

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


## Reconstructed slice locations

- [Milestone 2.1: RedbStateStore (`ecl-pipeline-state`)](slice01-redbstatestore/slice-plan.md)
- [Milestone 2.2: Pipeline Runner Core (`ecl-pipeline`)](slice02-pipeline-runner-core/slice-plan.md)
- [Milestone 2.3: Topology Resolution](slice03-topology-resolution/slice-plan.md)
