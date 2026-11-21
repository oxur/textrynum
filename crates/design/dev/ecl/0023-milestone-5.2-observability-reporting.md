# Milestone 5.2: Observability & Reporting

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline):`

## 1. Goal

Add observability instrumentation to the pipeline runner and human-readable
`Display` implementations to the state types. When done:

1. Every level of pipeline execution (run, batch, stage, item) emits
   structured `tracing` spans with relevant fields.
2. Stage and item lifecycle events (`stage_started`, `stage_completed`,
   `item_completed`, `item_failed`) are emitted as `tracing` events with
   timing and outcome data.
3. `PipelineStatus`, `ItemStatus`, and `StageStatus` all implement `Display`
   with human-readable output.
4. The runner populates timing fields (`StageState.started_at`,
   `StageState.completed_at`, `CompletedStageRecord.duration_ms`,
   `PipelineState.last_checkpoint`) during execution.
5. The serialized `PipelineState` JSON matches the structure defined in
   design doc 0017, section 6 (Observability: What Claude Sees).

## 2. Context

### 2.1 Project Conventions

- **Edition:** 2024, **Rust version:** 1.85
- **Lints** (in every crate's `Cargo.toml`):

  ```toml
  [lints.rust]
  unsafe_code = "forbid"
  missing_docs = "warn"

  [lints.clippy]
  unwrap_used = "deny"
  expect_used = "warn"
  panic = "deny"
  ```

- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, Error>;`
- **Test naming:** `test_<fn>_<scenario>_<expectation>`
- **Tests:** Inline `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { ... }`
- **Maps:** Always `BTreeMap` (not `HashMap`) for deterministic serialization
- **Params:** Use `&str` not `&String`, `&[T]` not `&Vec<T>` in function signatures
- **No `unwrap()`** in library code — use `?` or `.ok_or()`
- **Doc comments** on all public items

### 2.2 Rust Guides to Load

Before writing any code, read these files (paths relative to workspace root):

1. `assets/ai/ai-rust/guides/11-anti-patterns.md` (ALWAYS first)
2. `assets/ai/ai-rust/guides/01-core-idioms.md`
3. `assets/ai/ai-rust/guides/13-documentation.md`
4. `assets/ai/ai-rust/guides/05-type-design.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments
- `crates/ecl-pipeline/src/runner.rs` — Runner implementation (where tracing spans will be added)
- `crates/ecl-pipeline-state/src/types.rs` — State types (where Display impls will be added)

## 3. Prior Art / Dependencies

This milestone modifies two existing crates. It depends on:

### From `ecl-pipeline` (Milestone 2.2)

```rust
// --- crate: ecl-pipeline ---

/// The pipeline runner: orchestrates enumeration, incrementality,
/// batch execution, checkpointing, and resume.
pub struct PipelineRunner {
    topology: PipelineTopology,
    state: PipelineState,
    store: Box<dyn StateStore>,
    checkpoint_sequence: u64,
}

impl PipelineRunner {
    pub async fn new(topology: PipelineTopology, store: Box<dyn StateStore>) -> Result<Self>;
    pub async fn run(&mut self) -> Result<&PipelineState>;
    async fn execute_batch(&mut self, batch_idx: usize, stages: &[StageId]) -> Result<()>;
    async fn enumerate_sources(&mut self) -> Result<()>;
    async fn apply_incrementality(&mut self) -> Result<()>;
    async fn checkpoint(&mut self) -> Result<()>;
    fn merge_stage_result(&mut self, result: StageResult) -> Result<()>;
    fn collect_items_for_stage(&self, stage_id: &StageId) -> Vec<PipelineItem>;
    fn build_stage_context(&self, stage_name: &str) -> StageContext;
    pub fn state(&self) -> &PipelineState;
}

/// Execute a single stage's items with bounded concurrency.
pub async fn execute_stage_items(
    stage: ResolvedStage,
    items: Vec<PipelineItem>,
    ctx: StageContext,
    concurrency: usize,
) -> std::result::Result<StageResult, PipelineError>;

/// Execute a stage handler with retry and exponential backoff.
pub async fn execute_with_retry(
    handler: &Arc<dyn Stage>,
    item: PipelineItem,
    ctx: &StageContext,
    retry: &RetryPolicy,
) -> std::result::Result<Vec<PipelineItem>, StageError>;

/// The result of executing a single stage across all its items.
pub struct StageResult {
    pub stage_id: StageId,
    pub successes: Vec<StageItemSuccess>,
    pub skipped: Vec<StageItemSkipped>,
    pub failures: Vec<StageItemFailure>,
}
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
// --- crate: ecl-pipeline-state ---

/// Complete pipeline execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub run_id: RunId,
    pub pipeline_name: String,
    pub started_at: DateTime<Utc>,
    pub last_checkpoint: DateTime<Utc>,
    pub status: PipelineStatus,
    pub current_batch: usize,
    pub sources: BTreeMap<String, SourceState>,
    pub stages: BTreeMap<StageId, StageState>,
    pub stats: PipelineStats,
}

/// Overall pipeline execution status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    Pending,
    Running { current_stage: String },
    Completed { finished_at: DateTime<Utc> },
    Failed { error: String, failed_at: DateTime<Utc> },
    Interrupted { interrupted_at: DateTime<Utc> },
}

/// Processing status of a single pipeline item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemStatus {
    Pending,
    Processing { stage: String },
    Completed,
    Failed { stage: String, error: String, attempts: u32 },
    Skipped { stage: String, reason: String },
    Unchanged,
}

/// Execution status of a pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped { reason: String },
    Failed { error: String },
}

/// Per-stage aggregate state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    pub status: StageStatus,
    pub items_processed: usize,
    pub items_failed: usize,
    pub items_skipped: usize,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Record of a completed stage for an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStageRecord {
    pub stage: StageId,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
}

/// Per-source state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceState {
    pub items_discovered: usize,
    pub items_accepted: usize,
    pub items_skipped_unchanged: usize,
    pub items: BTreeMap<String, ItemState>,
}

/// Per-item state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState {
    pub display_name: String,
    pub source_id: String,
    pub source_name: String,
    pub content_hash: Blake3Hash,
    pub status: ItemStatus,
    pub completed_stages: Vec<CompletedStageRecord>,
    pub provenance: ItemProvenance,
}

/// Summary statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_items_discovered: usize,
    pub total_items_processed: usize,
    pub total_items_skipped_unchanged: usize,
    pub total_items_failed: usize,
}

// Newtypes
pub struct RunId(String);
pub struct StageId(String);
pub struct Blake3Hash(String);
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/ecl-pipeline-state/Cargo.toml` | Modify | Add `tracing-test` to `[dev-dependencies]` if not present |
| `crates/ecl-pipeline-state/src/types.rs` | Modify | Add `Display` impls for `PipelineStatus`, `ItemStatus`, `StageStatus` |
| `crates/ecl-pipeline/Cargo.toml` | Modify | Add `tracing-test` to `[dev-dependencies]` if not present |
| `crates/ecl-pipeline/src/runner.rs` | Modify | Add tracing spans to `run()`, `execute_batch()`, `enumerate_sources()`; populate timing fields; emit lifecycle events |
| `crates/ecl-pipeline/src/batch.rs` | Modify | Add tracing spans to `execute_stage_items()` and `execute_with_retry()`; emit item-level events; track duration for `CompletedStageRecord.duration_ms` |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml` (if not already present):

```toml
tracing-test = "0.2"
```

**Note:** `tracing`, `chrono`, `serde`, `serde_json`, `tokio`, and `thiserror`
are already in `[workspace.dependencies]`.

### Crate Cargo.toml Changes

#### `crates/ecl-pipeline-state/Cargo.toml`

Add to `[dev-dependencies]`:

```toml
tracing-test = { workspace = true }
```

#### `crates/ecl-pipeline/Cargo.toml`

Add to `[dev-dependencies]`:

```toml
tracing-test = { workspace = true }
```

No new production dependencies are needed. The `tracing` crate is already a
dependency of `ecl-pipeline`.

## 6. Type Definitions and Signatures

This milestone adds no new types. It adds `Display` implementations and
tracing instrumentation to existing types and functions.

### `Display` Implementations (in `ecl-pipeline-state/src/types.rs`)

#### `Display for PipelineStatus`

```rust
impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running { current_stage } => write!(f, "Running ({current_stage})"),
            Self::Completed { finished_at } => {
                write!(f, "Completed ({})", finished_at.format("%H:%M:%S"))
            }
            Self::Failed { error, .. } => write!(f, "Failed: {error}"),
            Self::Interrupted { interrupted_at } => {
                write!(f, "Interrupted ({})", interrupted_at.format("%H:%M:%S"))
            }
        }
    }
}
```

#### `Display for ItemStatus`

```rust
impl std::fmt::Display for ItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Processing { stage } => write!(f, "Processing ({stage})"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed { stage, error, .. } => write!(f, "Failed ({stage}: {error})"),
            Self::Skipped { reason, .. } => write!(f, "Skipped"),
            Self::Unchanged => write!(f, "Unchanged"),
        }
    }
}
```

#### `Display for StageStatus`

```rust
impl std::fmt::Display for StageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Skipped { reason } => write!(f, "Skipped: {reason}"),
            Self::Failed { error } => write!(f, "Failed: {error}"),
        }
    }
}
```

### Tracing Spans (in `ecl-pipeline/src/runner.rs` and `src/batch.rs`)

#### Pipeline run span (`runner.rs` — in `run()`)

```rust
pub async fn run(&mut self) -> Result<&PipelineState> {
    let span = tracing::info_span!(
        "pipeline_run",
        run_id = %self.state.run_id,
        pipeline = %self.state.pipeline_name,
    );
    let _enter = span.enter();

    // ... existing lifecycle logic ...
}
```

#### Batch span (`runner.rs` — in `execute_batch()`)

```rust
async fn execute_batch(
    &mut self,
    batch_idx: usize,
    stages: &[StageId],
) -> Result<()> {
    let span = tracing::info_span!(
        "batch",
        batch_idx = batch_idx,
        stages = ?stages.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );
    let _enter = span.enter();

    // ... existing batch logic ...
}
```

#### Stage span (`runner.rs` — in `execute_batch()`, per stage)

```rust
// Inside the per-stage loop in execute_batch(), before spawning:
let stage_span = tracing::info_span!(
    "stage",
    stage_id = %stage_id,
);

tracing::info!(stage_id = %stage_id, "stage_started");

// After stage completes (in the result-merge loop):
tracing::info!(
    stage_id = %stage_result.stage_id.as_str(),
    duration_ms = duration_ms,
    items_processed = stage_result.successes.len(),
    items_failed = stage_result.failures.len(),
    "stage_completed",
);
```

#### Item span (`batch.rs` — in `execute_stage_items()`, per item)

```rust
// Inside the per-item loop, within the spawned task:
let item_span = tracing::debug_span!(
    "item",
    item_id = %item.id,
    source = %item.source_name,
);
let _enter = item_span.enter();

// After item completes:
tracing::debug!(item_id = %item_id, "item_completed");

// On item failure:
tracing::warn!(item_id = %item_id, error = %e, "item_failed");
```

### Timing Field Population (in `ecl-pipeline/src/runner.rs`)

#### `StageState.started_at` and `completed_at`

The existing code in `execute_batch()` already sets `started_at = Some(Utc::now())`
when marking a stage as Running. This milestone ensures `completed_at` is also
set in `merge_stage_result()` (the existing code already does this). Verify and
keep.

#### `CompletedStageRecord.duration_ms`

Currently hardcoded to `0` in `merge_stage_result()`. This milestone changes
the approach: capture `Instant::now()` before stage execution, compute
elapsed duration after, and pass it through `StageResult` or compute it
per-item in `execute_stage_items()`.

The per-item approach: in `execute_stage_items()`, wrap each item's processing
call with timing:

```rust
// In the spawned task for each item:
let start = std::time::Instant::now();
let result = execute_with_retry(&handler, item.clone(), &ctx, &retry).await;
let duration_ms = start.elapsed().as_millis() as u64;
(item.id.clone(), result, skip_on_error, duration_ms)
```

Then propagate `duration_ms` into `StageItemSuccess`:

```rust
pub struct StageItemSuccess {
    pub item_id: String,
    pub outputs: Vec<PipelineItem>,
    /// How long the item took to process, in milliseconds.
    pub duration_ms: u64,
}
```

And in `merge_stage_result()`, use `success.duration_ms` when constructing
`CompletedStageRecord`:

```rust
item_state.completed_stages.push(CompletedStageRecord {
    stage: stage_id.clone(),
    completed_at: Utc::now(),
    duration_ms: success.duration_ms,
});
```

#### `PipelineState.last_checkpoint`

Already updated in `checkpoint()` after `store.save_checkpoint()`. Verify
and keep.

### JSON Structure Verification

The serialized `PipelineState` must produce JSON with this structure (from
design doc 0017, section 6). The test constructs a `PipelineState` that
matches this example and verifies the JSON keys:

```json
{
  "run_id": "run-2026-03-13-001",
  "pipeline_name": "q1-knowledge-sync",
  "started_at": "2026-03-13T14:30:00Z",
  "last_checkpoint": "2026-03-13T14:47:22Z",
  "status": { "Running": { "current_stage": "normalize-gdrive" } },
  "current_batch": 1,
  "sources": {
    "engineering-drive": {
      "items_discovered": 200,
      "items_accepted": 187,
      "items_skipped_unchanged": 142,
      "items": {
        "1abc...": {
          "display_name": "Q1 Architecture Review.docx",
          "source_id": "1abc...",
          "source_name": "engineering-drive",
          "content_hash": "a7f3b2...",
          "status": "Completed",
          "completed_stages": [
            { "stage": "fetch-gdrive", "completed_at": "...", "duration_ms": 1200 },
            { "stage": "normalize-gdrive", "completed_at": "...", "duration_ms": 340 }
          ],
          "provenance": {
            "source_kind": "google_drive",
            "metadata": {
              "file_id": "1abc...",
              "path": "/Engineering/Architecture/Q1 Architecture Review.docx",
              "owner": "alice@company.com"
            },
            "source_modified": "2026-03-10T09:15:00Z",
            "extracted_at": "2026-03-13T14:31:12Z"
          }
        },
        "2def...": {
          "display_name": "Meeting Notes (old).pdf",
          "status": {
            "Failed": {
              "stage": "normalize-gdrive",
              "error": "PDF conversion failed: encrypted document",
              "attempts": 3
            }
          }
        }
      }
    },
    "team-slack": {
      "items_discovered": 847,
      "items_accepted": 312,
      "items_skipped_unchanged": 280,
      "items": { "..." : "..." }
    }
  },
  "stages": {
    "fetch-gdrive": { "status": "Completed", "items_processed": 45, "items_failed": 0 },
    "fetch-slack": { "status": "Completed", "items_processed": 32, "items_failed": 0 },
    "normalize-gdrive": { "status": "Running", "items_processed": 38, "items_failed": 2 },
    "normalize-slack": { "status": "Completed", "items_processed": 32, "items_failed": 0 },
    "emit": { "status": "Pending", "items_processed": 0, "items_failed": 0 }
  },
  "stats": {
    "total_items_discovered": 1047,
    "total_items_processed": 147,
    "total_items_skipped_unchanged": 422,
    "total_items_failed": 2
  }
}
```

**Key structural requirements verified by tests:**

1. `status` for `PipelineStatus::Running` serializes as `{ "Running": { "current_stage": "..." } }`
2. `status` for `ItemStatus::Completed` serializes as the string `"Completed"`
3. `status` for `ItemStatus::Failed` serializes as `{ "Failed": { "stage": "...", "error": "...", "attempts": N } }`
4. `stages` map keys are stage name strings
5. `sources` map keys are source name strings
6. `completed_stages` is an array of objects with `stage`, `completed_at`, `duration_ms`
7. `provenance.metadata` is a map of string to JSON value
8. `stats` contains exactly `total_items_discovered`, `total_items_processed`, `total_items_skipped_unchanged`, `total_items_failed`

## 7. Implementation Steps (TDD Order)

### Step 1: Add workspace dependency and verify compilation

- [ ] Add `tracing-test = "0.2"` to root `Cargo.toml` `[workspace.dependencies]` (if not already present)
- [ ] Add `tracing-test = { workspace = true }` to `crates/ecl-pipeline-state/Cargo.toml` `[dev-dependencies]`
- [ ] Add `tracing-test = { workspace = true }` to `crates/ecl-pipeline/Cargo.toml` `[dev-dependencies]`
- [ ] Run `cargo check -p ecl-pipeline-state -p ecl-pipeline` — must pass
- [ ] Commit: `feat(ecl-pipeline): add tracing-test dev dependency`

### Step 2: Display impl for PipelineStatus

- [ ] Write tests first in `ecl-pipeline-state/src/types.rs`:
  - `test_pipeline_status_display_pending` — `PipelineStatus::Pending` displays as `"Pending"`
  - `test_pipeline_status_display_running` — `PipelineStatus::Running { current_stage: "normalize-gdrive".into() }` displays as `"Running (normalize-gdrive)"`
  - `test_pipeline_status_display_completed` — `PipelineStatus::Completed { finished_at }` displays as `"Completed (HH:MM:SS)"` where the time is formatted from `finished_at`
  - `test_pipeline_status_display_failed` — `PipelineStatus::Failed { error: "out of memory".into(), .. }` displays as `"Failed: out of memory"`
  - `test_pipeline_status_display_interrupted` — `PipelineStatus::Interrupted { interrupted_at }` displays as `"Interrupted (HH:MM:SS)"`
- [ ] Run tests — should fail (no Display impl)
- [ ] Implement `Display for PipelineStatus` (from section 6)
- [ ] Run tests — should pass
- [ ] Commit: `feat(ecl-pipeline-state): add Display impl for PipelineStatus`

### Step 3: Display impl for ItemStatus

- [ ] Write tests first in `ecl-pipeline-state/src/types.rs`:
  - `test_item_status_display_pending` — displays as `"Pending"`
  - `test_item_status_display_processing` — `ItemStatus::Processing { stage: "normalize-gdrive".into() }` displays as `"Processing (normalize-gdrive)"`
  - `test_item_status_display_completed` — displays as `"Completed"`
  - `test_item_status_display_failed` — `ItemStatus::Failed { stage: "normalize".into(), error: "bad format".into(), attempts: 3 }` displays as `"Failed (normalize: bad format)"`
  - `test_item_status_display_skipped` — displays as `"Skipped"`
  - `test_item_status_display_unchanged` — displays as `"Unchanged"`
- [ ] Run tests — should fail
- [ ] Implement `Display for ItemStatus` (from section 6)
- [ ] Run tests — should pass
- [ ] Commit: `feat(ecl-pipeline-state): add Display impl for ItemStatus`

### Step 4: Display impl for StageStatus

- [ ] Write tests first in `ecl-pipeline-state/src/types.rs`:
  - `test_stage_status_display_pending` — displays as `"Pending"`
  - `test_stage_status_display_running` — displays as `"Running"`
  - `test_stage_status_display_completed` — displays as `"Completed"`
  - `test_stage_status_display_skipped` — `StageStatus::Skipped { reason: "condition false".into() }` displays as `"Skipped: condition false"`
  - `test_stage_status_display_failed` — `StageStatus::Failed { error: "timeout".into() }` displays as `"Failed: timeout"`
- [ ] Run tests — should fail
- [ ] Implement `Display for StageStatus` (from section 6)
- [ ] Run tests — should pass
- [ ] Commit: `feat(ecl-pipeline-state): add Display impl for StageStatus`

### Step 5: JSON structure verification test

- [ ] Write test in `ecl-pipeline-state/src/types.rs` (or a new test file `ecl-pipeline-state/tests/json_structure.rs`):
  - `test_pipeline_state_json_matches_design_doc` — Construct a `PipelineState` that matches the design doc example from section 6. Serialize to JSON. Parse the JSON as `serde_json::Value`. Assert:
    - Top-level keys: `run_id`, `pipeline_name`, `started_at`, `last_checkpoint`, `status`, `current_batch`, `sources`, `stages`, `stats`
    - `status` is an object `{ "Running": { "current_stage": "normalize-gdrive" } }`
    - `sources.engineering-drive.items.1abc....status` is the string `"Completed"`
    - `sources.engineering-drive.items.2def....status` is an object `{ "Failed": { ... } }` with keys `stage`, `error`, `attempts`
    - `completed_stages[0]` has keys `stage`, `completed_at`, `duration_ms`
    - `provenance` has keys `source_kind`, `metadata`, `source_modified`, `extracted_at`
    - `stages.fetch-gdrive` has keys `status`, `items_processed`, `items_failed`
    - `stats` has keys `total_items_discovered`, `total_items_processed`, `total_items_skipped_unchanged`, `total_items_failed`
- [ ] Run test — should pass (types already serialize correctly via serde; this test validates the structure matches the design doc)
- [ ] Commit: `test(ecl-pipeline-state): verify JSON structure matches design doc section 6`

### Step 6: Add duration_ms tracking to StageItemSuccess

- [ ] Write test in `ecl-pipeline/src/batch.rs`:
  - `test_stage_item_success_has_duration_ms` — Verify `StageItemSuccess` has a `duration_ms` field
- [ ] Add `pub duration_ms: u64` field to `StageItemSuccess` in `batch.rs`
- [ ] Update `StageResult::record_success()` to accept and store `duration_ms`
- [ ] Update `execute_stage_items()` to wrap each item processing with `Instant::now()` / `elapsed()` and pass `duration_ms` to `record_success()`
- [ ] Update `merge_stage_result()` in `runner.rs` to use `success.duration_ms` instead of hardcoded `0` for `CompletedStageRecord.duration_ms`
- [ ] Run `cargo test -p ecl-pipeline` — must pass
- [ ] Commit: `feat(ecl-pipeline): track per-item duration_ms in stage execution`

### Step 7: Add tracing spans to runner.rs

- [ ] Write test in `ecl-pipeline/src/runner.rs` (using `tracing-test`):
  - `test_run_emits_pipeline_run_span` — Run a pipeline with a mock topology and verify the `pipeline_run` span is emitted with `run_id` and `pipeline` fields
  - `test_run_emits_stage_started_event` — Verify `stage_started` event is emitted for each stage
  - `test_run_emits_stage_completed_event` — Verify `stage_completed` event is emitted with `duration_ms`, `items_processed`, `items_failed` fields
- [ ] Add tracing spans to `run()`:
  ```rust
  let span = tracing::info_span!(
      "pipeline_run",
      run_id = %self.state.run_id,
      pipeline = %self.state.pipeline_name,
  );
  let _enter = span.enter();
  ```
- [ ] Add tracing spans to `execute_batch()`:
  ```rust
  let span = tracing::info_span!(
      "batch",
      batch_idx = batch_idx,
      stages = ?stages.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
  );
  let _enter = span.enter();
  ```
- [ ] Add `stage_started` event before spawning each stage task:
  ```rust
  tracing::info!(stage_id = %stage_id, "stage_started");
  ```
- [ ] Add `stage_completed` event in the result-merge loop after `merge_stage_result()`:
  ```rust
  let duration_ms = stage_state.started_at
      .and_then(|s| stage_state.completed_at.map(|c| (c - s).num_milliseconds() as u64))
      .unwrap_or(0);
  tracing::info!(
      stage_id = %stage_result.stage_id.as_str(),
      duration_ms = duration_ms,
      items_processed = stage_result.successes.len(),
      items_failed = stage_result.failures.len(),
      "stage_completed",
  );
  ```
- [ ] Run tests — should pass
- [ ] Commit: `feat(ecl-pipeline): add tracing spans to runner lifecycle`

### Step 8: Add tracing spans to batch.rs

- [ ] Write test in `ecl-pipeline/src/batch.rs` (using `tracing-test`):
  - `test_execute_stage_items_emits_item_spans` — Verify `item` debug spans are emitted with `item_id` and `source` fields
  - `test_execute_stage_items_emits_item_completed_event` — Verify `item_completed` event on success
  - `test_execute_stage_items_emits_item_failed_event` — Verify `item_failed` event on failure with `error` field
- [ ] Add stage span to `execute_stage_items()`:
  ```rust
  let stage_span = tracing::info_span!(
      "stage",
      stage_id = %stage.id,
  );
  let _enter = stage_span.enter();
  ```
- [ ] Add item span and events inside the spawned task:
  ```rust
  let item_span = tracing::debug_span!(
      "item",
      item_id = %item.id,
      source = %item.source_name,
  );
  let _enter = item_span.enter();

  let start = std::time::Instant::now();
  let result = execute_with_retry(&handler, item.clone(), &ctx, &retry).await;
  let duration_ms = start.elapsed().as_millis() as u64;

  match &result {
      Ok(_) => tracing::debug!(item_id = %item_id, duration_ms = duration_ms, "item_completed"),
      Err(e) => tracing::warn!(item_id = %item_id, error = %e, "item_failed"),
  }
  ```
- [ ] Run tests — should pass
- [ ] Commit: `feat(ecl-pipeline): add tracing spans to batch execution`

### Step 9: Verify timing fields are populated

- [ ] Write integration-style test in `ecl-pipeline/src/runner.rs`:
  - `test_stage_state_timing_populated_after_execution` — Run a pipeline with a mock stage; verify `StageState.started_at` is `Some(...)`, `StageState.completed_at` is `Some(...)`, and `completed_at > started_at`
  - `test_completed_stage_record_duration_ms_nonzero` — Run a pipeline with a mock stage that sleeps briefly; verify `CompletedStageRecord.duration_ms > 0` in the item's completed_stages list
  - `test_last_checkpoint_updated_after_checkpoint` — Run a pipeline; verify `PipelineState.last_checkpoint` is after `PipelineState.started_at`
- [ ] Run tests — should pass (timing population was implemented in steps 6-8)
- [ ] Commit: `test(ecl-pipeline): verify timing fields populated during execution`

### Step 10: Final polish and coverage

- [ ] Run `make test` — all tests pass
- [ ] Run `make lint` — no warnings
- [ ] Run `make format` — no changes
- [ ] Run `make coverage` — verify 95% or better for modified files
- [ ] Verify all public items have doc comments (no `missing_docs` warnings)
- [ ] Verify no compiler warnings
- [ ] Commit: `feat(ecl-pipeline): observability and reporting complete`

## 8. Test Fixtures

### Minimal PipelineState for JSON Structure Test

```rust
fn build_design_doc_example_state() -> PipelineState {
    use chrono::TimeZone;
    use std::collections::BTreeMap;

    let started_at = Utc.with_ymd_and_hms(2026, 3, 13, 14, 30, 0).unwrap();
    let last_checkpoint = Utc.with_ymd_and_hms(2026, 3, 13, 14, 47, 22).unwrap();

    let mut eng_items = BTreeMap::new();

    // Completed item
    eng_items.insert(
        "1abc...".to_string(),
        ItemState {
            display_name: "Q1 Architecture Review.docx".to_string(),
            source_id: "1abc...".to_string(),
            source_name: "engineering-drive".to_string(),
            content_hash: Blake3Hash::new("a7f3b2..."),
            status: ItemStatus::Completed,
            completed_stages: vec![
                CompletedStageRecord {
                    stage: StageId::new("fetch-gdrive"),
                    completed_at: Utc.with_ymd_and_hms(2026, 3, 13, 14, 31, 12).unwrap(),
                    duration_ms: 1200,
                },
                CompletedStageRecord {
                    stage: StageId::new("normalize-gdrive"),
                    completed_at: Utc.with_ymd_and_hms(2026, 3, 13, 14, 35, 0).unwrap(),
                    duration_ms: 340,
                },
            ],
            provenance: ItemProvenance {
                source_kind: "google_drive".to_string(),
                metadata: {
                    let mut m = BTreeMap::new();
                    m.insert("file_id".to_string(), serde_json::json!("1abc..."));
                    m.insert(
                        "path".to_string(),
                        serde_json::json!("/Engineering/Architecture/Q1 Architecture Review.docx"),
                    );
                    m.insert("owner".to_string(), serde_json::json!("alice@company.com"));
                    m
                },
                source_modified: Some(
                    Utc.with_ymd_and_hms(2026, 3, 10, 9, 15, 0).unwrap(),
                ),
                extracted_at: Utc.with_ymd_and_hms(2026, 3, 13, 14, 31, 12).unwrap(),
            },
        },
    );

    // Failed item
    eng_items.insert(
        "2def...".to_string(),
        ItemState {
            display_name: "Meeting Notes (old).pdf".to_string(),
            source_id: "2def...".to_string(),
            source_name: "engineering-drive".to_string(),
            content_hash: Blake3Hash::new(""),
            status: ItemStatus::Failed {
                stage: "normalize-gdrive".to_string(),
                error: "PDF conversion failed: encrypted document".to_string(),
                attempts: 3,
            },
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "google_drive".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: Utc.with_ymd_and_hms(2026, 3, 13, 14, 31, 15).unwrap(),
            },
        },
    );

    let mut sources = BTreeMap::new();
    sources.insert(
        "engineering-drive".to_string(),
        SourceState {
            items_discovered: 200,
            items_accepted: 187,
            items_skipped_unchanged: 142,
            items: eng_items,
        },
    );
    sources.insert(
        "team-slack".to_string(),
        SourceState {
            items_discovered: 847,
            items_accepted: 312,
            items_skipped_unchanged: 280,
            items: BTreeMap::new(),
        },
    );

    let mut stages = BTreeMap::new();
    stages.insert(
        StageId::new("fetch-gdrive"),
        StageState {
            status: StageStatus::Completed,
            items_processed: 45,
            items_failed: 0,
            items_skipped: 0,
            started_at: Some(started_at),
            completed_at: Some(Utc.with_ymd_and_hms(2026, 3, 13, 14, 35, 0).unwrap()),
        },
    );
    stages.insert(
        StageId::new("fetch-slack"),
        StageState {
            status: StageStatus::Completed,
            items_processed: 32,
            items_failed: 0,
            items_skipped: 0,
            started_at: Some(started_at),
            completed_at: Some(Utc.with_ymd_and_hms(2026, 3, 13, 14, 33, 0).unwrap()),
        },
    );
    stages.insert(
        StageId::new("normalize-gdrive"),
        StageState {
            status: StageStatus::Running,
            items_processed: 38,
            items_failed: 2,
            items_skipped: 0,
            started_at: Some(Utc.with_ymd_and_hms(2026, 3, 13, 14, 35, 1).unwrap()),
            completed_at: None,
        },
    );
    stages.insert(
        StageId::new("normalize-slack"),
        StageState {
            status: StageStatus::Completed,
            items_processed: 32,
            items_failed: 0,
            items_skipped: 0,
            started_at: Some(Utc.with_ymd_and_hms(2026, 3, 13, 14, 33, 1).unwrap()),
            completed_at: Some(Utc.with_ymd_and_hms(2026, 3, 13, 14, 40, 0).unwrap()),
        },
    );
    stages.insert(
        StageId::new("emit"),
        StageState {
            status: StageStatus::Pending,
            items_processed: 0,
            items_failed: 0,
            items_skipped: 0,
            started_at: None,
            completed_at: None,
        },
    );

    PipelineState {
        run_id: RunId::new("run-2026-03-13-001"),
        pipeline_name: "q1-knowledge-sync".to_string(),
        started_at,
        last_checkpoint,
        status: PipelineStatus::Running {
            current_stage: "normalize-gdrive".to_string(),
        },
        current_batch: 1,
        sources,
        stages,
        stats: PipelineStats {
            total_items_discovered: 1047,
            total_items_processed: 147,
            total_items_skipped_unchanged: 422,
            total_items_failed: 2,
        },
    }
}
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_pipeline_status_display_pending` | `ecl-pipeline-state::types` | `PipelineStatus::Pending` displays as `"Pending"` |
| `test_pipeline_status_display_running` | `ecl-pipeline-state::types` | `PipelineStatus::Running` displays as `"Running (stage-name)"` |
| `test_pipeline_status_display_completed` | `ecl-pipeline-state::types` | `PipelineStatus::Completed` displays as `"Completed (HH:MM:SS)"` |
| `test_pipeline_status_display_failed` | `ecl-pipeline-state::types` | `PipelineStatus::Failed` displays as `"Failed: error msg"` |
| `test_pipeline_status_display_interrupted` | `ecl-pipeline-state::types` | `PipelineStatus::Interrupted` displays as `"Interrupted (HH:MM:SS)"` |
| `test_item_status_display_pending` | `ecl-pipeline-state::types` | `ItemStatus::Pending` displays as `"Pending"` |
| `test_item_status_display_processing` | `ecl-pipeline-state::types` | `ItemStatus::Processing` displays as `"Processing (stage-name)"` |
| `test_item_status_display_completed` | `ecl-pipeline-state::types` | `ItemStatus::Completed` displays as `"Completed"` |
| `test_item_status_display_failed` | `ecl-pipeline-state::types` | `ItemStatus::Failed` displays as `"Failed (stage: error)"` |
| `test_item_status_display_skipped` | `ecl-pipeline-state::types` | `ItemStatus::Skipped` displays as `"Skipped"` |
| `test_item_status_display_unchanged` | `ecl-pipeline-state::types` | `ItemStatus::Unchanged` displays as `"Unchanged"` |
| `test_stage_status_display_pending` | `ecl-pipeline-state::types` | `StageStatus::Pending` displays as `"Pending"` |
| `test_stage_status_display_running` | `ecl-pipeline-state::types` | `StageStatus::Running` displays as `"Running"` |
| `test_stage_status_display_completed` | `ecl-pipeline-state::types` | `StageStatus::Completed` displays as `"Completed"` |
| `test_stage_status_display_skipped` | `ecl-pipeline-state::types` | `StageStatus::Skipped` displays as `"Skipped: reason"` |
| `test_stage_status_display_failed` | `ecl-pipeline-state::types` | `StageStatus::Failed` displays as `"Failed: error"` |
| `test_pipeline_state_json_matches_design_doc` | `ecl-pipeline-state::types` | Serialized JSON keys/structure match design doc section 6 exactly |
| `test_pipeline_state_json_status_running_structure` | `ecl-pipeline-state::types` | `PipelineStatus::Running` serializes as `{ "Running": { "current_stage": "..." } }` |
| `test_pipeline_state_json_item_status_completed_string` | `ecl-pipeline-state::types` | `ItemStatus::Completed` serializes as the string `"Completed"` |
| `test_pipeline_state_json_item_status_failed_structure` | `ecl-pipeline-state::types` | `ItemStatus::Failed` serializes as `{ "Failed": { "stage": ..., "error": ..., "attempts": ... } }` |
| `test_stage_item_success_has_duration_ms` | `ecl-pipeline::batch` | `StageItemSuccess` contains `duration_ms` field |
| `test_execute_stage_items_tracks_duration` | `ecl-pipeline::batch` | `StageItemSuccess.duration_ms > 0` after processing |
| `test_run_emits_pipeline_run_span` | `ecl-pipeline::runner` | `pipeline_run` span emitted with `run_id` and `pipeline` fields |
| `test_run_emits_stage_started_event` | `ecl-pipeline::runner` | `stage_started` event emitted per stage |
| `test_run_emits_stage_completed_event` | `ecl-pipeline::runner` | `stage_completed` event emitted with `duration_ms`, `items_processed`, `items_failed` |
| `test_execute_stage_items_emits_item_spans` | `ecl-pipeline::batch` | `item` spans emitted with `item_id` and `source` fields |
| `test_execute_stage_items_emits_item_completed_event` | `ecl-pipeline::batch` | `item_completed` event emitted on success |
| `test_execute_stage_items_emits_item_failed_event` | `ecl-pipeline::batch` | `item_failed` event emitted on failure with `error` field |
| `test_stage_state_timing_populated_after_execution` | `ecl-pipeline::runner` | `started_at` is `Some`, `completed_at` is `Some`, `completed_at > started_at` |
| `test_completed_stage_record_duration_ms_nonzero` | `ecl-pipeline::runner` | `CompletedStageRecord.duration_ms > 0` after pipeline run |
| `test_last_checkpoint_updated_after_checkpoint` | `ecl-pipeline::runner` | `last_checkpoint >= started_at` after pipeline run |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline-state -p ecl-pipeline` passes
- [ ] `cargo test -p ecl-pipeline-state` passes (all tests green)
- [ ] `cargo test -p ecl-pipeline` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`
- [ ] `Display` impls produce the exact output specified in section 6
- [ ] JSON serialization of `PipelineState` matches design doc section 6 structure
- [ ] Tracing spans have correct names and fields at correct levels (info vs debug)
- [ ] Timing fields (`started_at`, `completed_at`, `duration_ms`, `last_checkpoint`) are populated by the runner

### What NOT to Do

- Do NOT add new CLI commands (that is milestone 5.1)
- Do NOT add new adapters or stages
- Do NOT implement a web dashboard or log aggregation
- Do NOT change the runner's execution logic — only ADD tracing instrumentation
- Do NOT add new state types or fields beyond what is defined in milestone 1.2
- Do NOT modify the `StateStore` trait or checkpoint format
- Do NOT add metrics collection, Prometheus, or OpenTelemetry exports
- Do NOT add log file output configuration (that belongs in the CLI layer)
