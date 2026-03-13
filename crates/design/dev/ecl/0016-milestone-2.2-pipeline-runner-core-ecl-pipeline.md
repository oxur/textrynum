# Milestone 2.2: Pipeline Runner Core (`ecl-pipeline`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline):`

## 1. Goal

Create the `ecl-pipeline` crate containing the pipeline execution engine. When
done:

1. `PipelineRunner` struct compiles and owns a `PipelineTopology`,
   `PipelineState`, and `Box<dyn StateStore>`.
2. `PipelineRunner::new()` accepts a pre-resolved topology and a state store,
   loads checkpoint if available, detects config drift, and prepares state for
   resume.
3. `PipelineRunner::run()` executes the full pipeline lifecycle: enumerate
   sources, apply incrementality, execute batches in order, checkpoint at batch
   boundaries, and finalize.
4. `execute_batch()` runs stages concurrently via `tokio::task::JoinSet` with
   snapshot isolation (`StageContext`), then merges results.
5. `enumerate_sources()` calls `SourceAdapter::enumerate()` for each source and
   populates the item list in `PipelineState`.
6. `apply_incrementality()` compares content hashes against the previous run and
   marks unchanged items.
7. `checkpoint()` builds a `Checkpoint` and persists it via `StateStore`.
8. `execute_stage_items()` processes items through a stage with
   semaphore-bounded concurrency.
9. `execute_with_retry()` retries failed stage operations with exponential
   backoff via the `backon` crate.
10. `StageResult` collects per-item outcomes within a stage.
11. `PipelineError` provides comprehensive error variants.
12. All tests pass using `InMemoryStateStore` and mock adapters/stages.

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
- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, PipelineError>;`
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
3. `assets/ai/ai-rust/guides/07-concurrency-async.md`
4. `assets/ai/ai-rust/guides/03-error-handling.md`
5. `assets/ai/ai-rust/guides/08-performance.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments

## 3. Prior Art / Dependencies

This crate depends on three prior milestone crates. Their complete public APIs
are listed here so you do not need to read any other files.

### From `ecl-pipeline-spec` (Milestone 1.1)

```rust
// --- crate: ecl-pipeline-spec ---

/// The root configuration, deserialized from TOML.
/// Immutable after load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub name: String,
    pub version: u32,
    pub output_dir: PathBuf,
    pub sources: BTreeMap<String, SourceSpec>,
    pub stages: BTreeMap<String, StageSpec>,
    #[serde(default)]
    pub defaults: DefaultsSpec,
}

impl PipelineSpec {
    pub fn from_toml(toml_str: &str) -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
}

/// Global defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSpec {
    #[serde(default = "default_concurrency")]  // default: 4
    pub concurrency: usize,
    #[serde(default)]
    pub retry: RetrySpec,
    #[serde(default)]
    pub checkpoint: CheckpointStrategy,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    pub max_attempts: u32,       // default: 3
    pub initial_backoff_ms: u64, // default: 1000
    pub backoff_multiplier: f64, // default: 2.0
    pub max_backoff_ms: u64,     // default: 30_000
}

/// When to write checkpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointStrategy { Batch, Items { count: usize }, Seconds { duration: u64 } }

/// A source specification (internally-tagged by `kind`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    #[serde(rename = "google_drive")] GoogleDrive(GoogleDriveSourceSpec),
    #[serde(rename = "slack")]        Slack(SlackSourceSpec),
    #[serde(rename = "filesystem")]   Filesystem(FilesystemSourceSpec),
}

/// A stage specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    pub adapter: String,
    pub source: Option<String>,
    #[serde(default)]
    pub resources: ResourceSpec,
    #[serde(default)]
    pub params: serde_json::Value,
    pub retry: Option<RetrySpec>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub skip_on_error: bool,
    pub condition: Option<String>,
}

/// Resource access declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    #[serde(default)] pub reads: Vec<String>,
    #[serde(default)] pub creates: Vec<String>,
    #[serde(default)] pub writes: Vec<String>,
}

pub type Result<T> = std::result::Result<T, SpecError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SpecError {
    ParseError { message: String },
    UnknownSource { stage: String, source: String },
    DuplicateStage { name: String },
    EmptyPipeline,
    EmptySources,
    ValidationError { message: String },
}
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
// --- crate: ecl-pipeline-state ---

/// Deterministic, name-based stage identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StageId(String);
impl StageId {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for StageId { /* writes inner string */ }

/// Blake3 content hash, stored as hex string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);
impl Blake3Hash {
    pub fn new(hex: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    pub fn is_empty(&self) -> bool;
}
impl std::fmt::Display for Blake3Hash { /* writes inner string */ }

/// Unique identifier for a pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(String);
impl RunId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for RunId { /* writes inner string */ }

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

impl PipelineState {
    /// Recompute PipelineStats from source/item data.
    pub fn update_stats(&mut self);
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

/// Record of a completed stage for an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStageRecord {
    pub stage: StageId,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
}

/// Provenance information for a pipeline item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProvenance {
    pub source_kind: String,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub source_modified: Option<DateTime<Utc>>,
    pub extracted_at: DateTime<Utc>,
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

/// Execution status of a pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped { reason: String },
    Failed { error: String },
}

/// Summary statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_items_discovered: usize,
    pub total_items_processed: usize,
    pub total_items_skipped_unchanged: usize,
    pub total_items_failed: usize,
}

/// Self-contained recovery artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub version: u32,
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
    pub spec: PipelineSpec,
    pub schedule: Vec<Vec<StageId>>,
    pub spec_hash: Blake3Hash,
    pub state: PipelineState,
}

impl Checkpoint {
    pub fn prepare_for_resume(&mut self);
    pub fn config_drifted(&self, current_spec_hash: &Blake3Hash) -> bool;
}

/// Persistence backend for pipeline state.
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> std::result::Result<(), StateError>;
    async fn load_checkpoint(&self) -> std::result::Result<Option<Checkpoint>, StateError>;
    async fn load_previous_hashes(&self) -> std::result::Result<BTreeMap<String, Blake3Hash>, StateError>;
    async fn save_completed_hashes(
        &self, run_id: &RunId, hashes: &BTreeMap<String, Blake3Hash>,
    ) -> std::result::Result<(), StateError>;
}

/// In-memory state store for testing.
#[derive(Debug, Default)]
pub struct InMemoryStateStore { /* checkpoint: RwLock<Option<Checkpoint>>, hashes: RwLock<BTreeMap<String, Blake3Hash>> */ }
impl InMemoryStateStore {
    pub fn new() -> Self;
}
// Implements StateStore.

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    SerializationError { message: String },
    StoreError { message: String },
    UnsupportedVersion { version: u32 },
    ConfigDrift { expected: String, actual: String },
    NotFound,
}

pub type Result<T> = std::result::Result<T, StateError>;
```

### From `ecl-pipeline-topo` (Milestone 1.3)

```rust
// --- crate: ecl-pipeline-topo ---

/// The resolved pipeline, ready to execute.
#[derive(Debug, Clone)]
pub struct PipelineTopology {
    pub spec: Arc<PipelineSpec>,
    pub spec_hash: Blake3Hash,
    pub sources: BTreeMap<String, Arc<dyn SourceAdapter>>,
    pub stages: BTreeMap<String, ResolvedStage>,
    pub schedule: Vec<Vec<StageId>>,
    pub output_dir: PathBuf,
}

/// A resolved stage with concrete implementation.
#[derive(Debug, Clone)]
pub struct ResolvedStage {
    pub id: StageId,
    pub handler: Arc<dyn Stage>,
    pub retry: RetryPolicy,
    pub skip_on_error: bool,
    pub timeout: Option<Duration>,
    pub source: Option<String>,
    pub condition: Option<ConditionExpr>,
}

/// Retry policy with resolved Duration values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub backoff_multiplier: f64,
    pub max_backoff: Duration,
}

impl RetryPolicy {
    pub fn from_spec(spec: &RetrySpec) -> Self;
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_millis(30_000),
        }
    }
}

/// Condition expression (simple string wrapper; evaluator deferred).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionExpr(String);
impl ConditionExpr {
    pub fn new(expr: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

/// A source adapter handles all interaction with an external data service.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    fn source_kind(&self) -> &str;
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor (no content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub id: String,
    pub display_name: String,
    pub mime_type: String,
    pub path: String,
    pub modified_at: Option<DateTime<Utc>>,
    pub source_hash: Option<String>,
}

/// Document extracted from a source, in original format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    pub id: String,
    pub display_name: String,
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,
    pub mime_type: String,
    pub provenance: ItemProvenance,
    pub content_hash: Blake3Hash,
}

/// The intermediate representation flowing between stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    pub id: String,
    pub display_name: String,
    #[serde(with = "serde_bytes")]
    pub content: Arc<[u8]>,
    pub mime_type: String,
    pub source_name: String,
    pub source_content_hash: Blake3Hash,
    pub provenance: ItemProvenance,
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
#[async_trait]
pub trait Stage: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
#[derive(Debug, Clone)]
pub struct StageContext {
    pub spec: Arc<PipelineSpec>,
    pub output_dir: PathBuf,
    pub params: serde_json::Value,
    pub span: tracing::Span,
}

// Error types
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveError {
    SerializeError { message: String },
    UnknownSource { stage: String, source: String },
    MissingResource { stage: String, resource: String },
    CycleDetected { stages: Vec<StageId> },
    DuplicateCreator { resource: String, stages: Vec<StageId> },
    Io(#[from] std::io::Error),
    UnknownAdapter { stage: String, adapter: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SourceError {
    AuthError { source: String, message: String },
    RateLimited { source: String, retry_after_secs: u64 },
    NotFound { source: String, item_id: String },
    Transient { source: String, message: String },
    Permanent { source: String, message: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StageError {
    UnsupportedContent { stage: String, item_id: String, message: String },
    Transient { stage: String, item_id: String, message: String },
    Permanent { stage: String, item_id: String, message: String },
    Timeout { stage: String, item_id: String, timeout_secs: u64 },
}

pub type ResolveResult<T> = std::result::Result<T, ResolveError>;
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add `crates/ecl-pipeline` to workspace members |
| `crates/ecl-pipeline/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-pipeline/src/lib.rs` | Create | Module declarations, re-exports, `PipelineRunner` struct |
| `crates/ecl-pipeline/src/runner.rs` | Create | `PipelineRunner::new()`, `run()`, `enumerate_sources()`, `apply_incrementality()`, `checkpoint()`, helper methods |
| `crates/ecl-pipeline/src/batch.rs` | Create | `execute_batch()`, `execute_stage_items()`, `execute_with_retry()`, `StageResult` |
| `crates/ecl-pipeline/src/error.rs` | Create | `PipelineError` enum |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace] members`:

```toml
"crates/ecl-pipeline",
```

No new workspace-level dependencies are needed. All required dependencies
(`backon`, `tokio`, `serde`, `serde_json`, `chrono`, `thiserror`,
`async-trait`, `tracing`, `blake3`) already exist in `[workspace.dependencies]`.

### Crate Cargo.toml

```toml
[package]
name = "ecl-pipeline"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Pipeline execution engine for ECL pipeline runner"

[dependencies]
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
backon = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

## 6. Type Definitions and Signatures

All types below must be implemented exactly as shown.

### `src/error.rs`

```rust
//! Error types for the pipeline execution engine.

use ecl_pipeline_state::StateError;
use ecl_pipeline_topo::{ResolveError, SourceError, StageError};
use thiserror::Error;

/// Errors that can occur during pipeline execution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PipelineError {
    /// Source enumeration failed.
    #[error("source enumeration failed for '{source}': {error}")]
    SourceEnumeration {
        /// The source that failed enumeration.
        source: String,
        /// Error detail.
        error: String,
    },

    /// Stage execution failed (non-retryable or exhausted retries).
    #[error("stage execution failed: {0}")]
    StageExecution(#[from] StageError),

    /// State store operation failed.
    #[error("state store error: {0}")]
    StateStore(#[from] StateError),

    /// Topology resolution failed.
    #[error("topology resolution error: {0}")]
    Resolve(#[from] ResolveError),

    /// Source adapter error.
    #[error("source error: {0}")]
    Source(#[from] SourceError),

    /// Config drift detected on resume: the TOML changed since the
    /// checkpoint was created.
    #[error("config drift detected: checkpoint hash '{checkpoint_hash}' != current hash '{current_hash}'")]
    ConfigDrift {
        /// The spec hash stored in the checkpoint.
        checkpoint_hash: String,
        /// The spec hash computed from the current TOML.
        current_hash: String,
    },

    /// A tokio JoinSet task panicked or was cancelled.
    #[error("task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    /// Semaphore acquisition failed (should not happen in normal operation).
    #[error("semaphore acquire error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    /// A batch contained a stage that failed and was not configured with
    /// skip_on_error.
    #[error("stage '{stage}' failed for item '{item_id}': {error}")]
    ItemFailed {
        /// The stage where the failure occurred.
        stage: String,
        /// The item that failed.
        item_id: String,
        /// Error detail.
        error: String,
    },
}

/// Result type for pipeline operations.
pub type Result<T> = std::result::Result<T, PipelineError>;
```

### `src/batch.rs`

```rust
//! Batch execution: concurrent stage processing with bounded concurrency.
//!
//! This module contains the free functions that run as independent tokio tasks.
//! They take owned data (no shared mutable state) and return results.

use std::sync::Arc;

use ecl_pipeline_state::StageId;
use ecl_pipeline_topo::{PipelineItem, ResolvedStage, Stage, StageContext, StageError};

use crate::error::PipelineError;

/// The result of executing a single stage across all its items.
#[derive(Debug)]
pub struct StageResult {
    /// Which stage produced these results.
    pub stage_id: StageId,
    /// Items that completed successfully, with their output items.
    pub successes: Vec<StageItemSuccess>,
    /// Items that were skipped due to `skip_on_error`.
    pub skipped: Vec<StageItemSkipped>,
    /// Items that failed (non-recoverable or exhausted retries).
    pub failures: Vec<StageItemFailure>,
}

/// A successful item processing result.
#[derive(Debug)]
pub struct StageItemSuccess {
    /// The input item ID.
    pub item_id: String,
    /// The output items produced by the stage.
    pub outputs: Vec<PipelineItem>,
}

/// An item that was skipped due to `skip_on_error`.
#[derive(Debug)]
pub struct StageItemSkipped {
    /// The input item ID.
    pub item_id: String,
    /// The error that caused the skip.
    pub error: StageError,
}

/// An item that failed processing.
#[derive(Debug)]
pub struct StageItemFailure {
    /// The input item ID.
    pub item_id: String,
    /// The error.
    pub error: StageError,
}

impl StageResult {
    /// Create a new empty StageResult for the given stage.
    pub fn new(stage_id: StageId) -> Self {
        Self {
            stage_id,
            successes: Vec::new(),
            skipped: Vec::new(),
            failures: Vec::new(),
        }
    }

    /// Record a successful item processing.
    pub fn record_success(&mut self, item_id: String, outputs: Vec<PipelineItem>) {
        self.successes.push(StageItemSuccess { item_id, outputs });
    }

    /// Record a skipped item (due to `skip_on_error`).
    pub fn record_skipped(&mut self, item_id: String, error: StageError) {
        self.skipped.push(StageItemSkipped { item_id, error });
    }

    /// Record a failed item.
    pub fn record_failure(&mut self, item_id: String, error: StageError) {
        self.failures.push(StageItemFailure { item_id, error });
    }

    /// Returns true if any items failed (not skipped — actually failed).
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }
}

/// Execute a single stage's items with bounded concurrency.
///
/// Runs as an independent task — no shared mutable state. The caller
/// provides owned data; the function returns a `StageResult` with
/// per-item outcomes.
///
/// Uses a `tokio::sync::Semaphore` to limit concurrent item processing
/// to `concurrency` parallel operations. Each item is spawned as a
/// separate tokio task within a `JoinSet`.
pub async fn execute_stage_items(
    stage: ResolvedStage,
    items: Vec<PipelineItem>,
    ctx: StageContext,
    concurrency: usize,
) -> std::result::Result<StageResult, PipelineError> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut join_set = tokio::task::JoinSet::new();

    for item in items {
        let permit = semaphore.clone().acquire_owned().await?;
        let handler = stage.handler.clone();
        let ctx = ctx.clone();
        let retry = stage.retry.clone();
        let skip_on_error = stage.skip_on_error;

        join_set.spawn(async move {
            let _permit = permit; // held until task completes
            let result = execute_with_retry(&handler, item.clone(), &ctx, &retry).await;
            (item.id.clone(), result, skip_on_error)
        });
    }

    let mut stage_result = StageResult::new(stage.id.clone());
    while let Some(result) = join_set.join_next().await {
        let (item_id, result, skip_on_error) = result?;
        match result {
            Ok(outputs) => stage_result.record_success(item_id, outputs),
            Err(e) if skip_on_error => stage_result.record_skipped(item_id, e),
            Err(e) => stage_result.record_failure(item_id, e),
        }
    }
    Ok(stage_result)
}

/// Execute a stage handler with retry and exponential backoff.
///
/// Uses the `backon` crate for retry logic. The backoff parameters
/// come from the `RetryPolicy` which was resolved from the stage's
/// retry configuration merged with global defaults.
///
/// Only retries on error — successful results are returned immediately.
pub async fn execute_with_retry(
    handler: &Arc<dyn Stage>,
    item: PipelineItem,
    ctx: &StageContext,
    retry: &ecl_pipeline_topo::RetryPolicy,
) -> std::result::Result<Vec<PipelineItem>, StageError> {
    use backon::{ExponentialBuilder, Retryable};

    let backoff = ExponentialBuilder::default()
        .with_min_delay(retry.initial_backoff)
        .with_factor(retry.backoff_multiplier as f32)
        .with_max_delay(retry.max_backoff)
        .with_max_times(retry.max_attempts.saturating_sub(1) as usize);

    (|| async { handler.process(item.clone(), ctx).await })
        .retry(backoff)
        .await
}
```

### `src/runner.rs`

```rust
//! PipelineRunner: the main execution orchestrator.
//!
//! Owns the topology, state, and state store. Executes the pipeline
//! lifecycle: enumerate sources, apply incrementality, run batches,
//! checkpoint at boundaries, and finalize.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;

use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{
    Blake3Hash, Checkpoint, InMemoryStateStore, ItemProvenance, ItemState, ItemStatus,
    PipelineState, PipelineStats, PipelineStatus, RunId, SourceState, StageId, StageState,
    StageStatus, StateStore,
};
use ecl_pipeline_topo::{PipelineItem, PipelineTopology, StageContext};

use crate::batch::{execute_stage_items, StageResult};
use crate::error::{PipelineError, Result};

/// The pipeline runner: orchestrates enumeration, incrementality,
/// batch execution, checkpointing, and resume.
#[derive(Debug)]
pub struct PipelineRunner {
    /// The resolved pipeline topology (immutable during execution).
    topology: PipelineTopology,
    /// Mutable execution state (checkpointed after each batch).
    state: PipelineState,
    /// Persistence backend for checkpoints and content hashes.
    store: Box<dyn StateStore>,
    /// Monotonically increasing sequence number for checkpoints.
    checkpoint_sequence: u64,
}

impl PipelineRunner {
    /// Create a new runner from a pre-resolved topology and state store.
    ///
    /// If the store contains a checkpoint, loads it and prepares for
    /// resume (resets stuck items/stages, checks for config drift).
    /// If no checkpoint exists, creates fresh state.
    ///
    /// # Errors
    ///
    /// - `PipelineError::ConfigDrift` if the checkpoint's spec hash
    ///   does not match the current topology's spec hash.
    /// - `PipelineError::StateStore` if the store fails to load.
    pub async fn new(
        topology: PipelineTopology,
        store: Box<dyn StateStore>,
    ) -> Result<Self> {
        let state = match store.load_checkpoint().await? {
            Some(mut checkpoint) => {
                // Config drift check.
                if checkpoint.config_drifted(&topology.spec_hash) {
                    return Err(PipelineError::ConfigDrift {
                        checkpoint_hash: checkpoint.spec_hash.as_str().to_string(),
                        current_hash: topology.spec_hash.as_str().to_string(),
                    });
                }
                // Reset any items/stages stuck mid-processing.
                checkpoint.prepare_for_resume();
                tracing::info!(
                    run_id = %checkpoint.state.run_id,
                    sequence = checkpoint.sequence,
                    "Resuming from checkpoint",
                );
                checkpoint.state
            }
            None => {
                let now = Utc::now();
                let run_id = RunId::new(format!(
                    "{}-{}",
                    topology.spec.name,
                    now.format("%Y%m%dT%H%M%S")
                ));

                // Initialize stage states from topology schedule.
                let mut stages = BTreeMap::new();
                for batch in &topology.schedule {
                    for stage_id in batch {
                        stages.insert(
                            stage_id.clone(),
                            StageState {
                                status: StageStatus::Pending,
                                items_processed: 0,
                                items_failed: 0,
                                items_skipped: 0,
                                started_at: None,
                                completed_at: None,
                            },
                        );
                    }
                }

                PipelineState {
                    run_id,
                    pipeline_name: topology.spec.name.clone(),
                    started_at: now,
                    last_checkpoint: now,
                    status: PipelineStatus::Pending,
                    current_batch: 0,
                    sources: BTreeMap::new(),
                    stages,
                    stats: PipelineStats::default(),
                }
            }
        };

        Ok(Self {
            topology,
            state,
            store,
            checkpoint_sequence: 0,
        })
    }

    /// Execute the pipeline.
    ///
    /// Lifecycle:
    /// 1. Enumerate items from all sources (if not already done).
    /// 2. Apply incrementality (skip unchanged items).
    /// 3. Execute batches in order, checkpointing after each.
    /// 4. Save completed hashes and finalize state.
    ///
    /// Returns a reference to the final pipeline state.
    pub async fn run(&mut self) -> Result<&PipelineState> {
        // Phase 1: Enumerate items from all sources (if not already done).
        if self.state.current_batch == 0 && self.state.stats.total_items_discovered == 0 {
            self.enumerate_sources().await?;
            self.apply_incrementality().await?;
            self.checkpoint().await?;
        }

        // Phase 2: Execute batches.
        let schedule = self.topology.schedule.clone();
        for (batch_idx, batch) in schedule.iter().enumerate() {
            if batch_idx < self.state.current_batch {
                // Already completed in a prior run — skip.
                continue;
            }

            self.execute_batch(batch_idx, batch).await?;
            self.state.current_batch = batch_idx + 1;
            self.checkpoint().await?;
        }

        // Phase 3: Finalize.
        self.save_completed_hashes().await?;
        self.state.status = PipelineStatus::Completed {
            finished_at: Utc::now(),
        };
        self.checkpoint().await?;

        Ok(&self.state)
    }

    /// Execute a single batch: stages in this batch run concurrently.
    ///
    /// Builds an immutable `StageContext` snapshot before execution.
    /// Each stage in the batch gets the same view. After all stages
    /// complete, their results are merged into the shared state.
    async fn execute_batch(
        &mut self,
        _batch_idx: usize,
        stages: &[StageId],
    ) -> Result<()> {
        // Filter out stages whose conditions are not met.
        // For now, all stages with conditions are treated as always-run
        // (condition evaluation is deferred to a future milestone).
        let active_stages: Vec<&StageId> = stages
            .iter()
            .filter(|id| self.should_execute_stage(id))
            .collect();

        // Execute stages concurrently (one tokio task per stage).
        let mut join_set = tokio::task::JoinSet::new();
        for stage_id in &active_stages {
            let stage_name = stage_id.as_str().to_string();
            let stage = self
                .topology
                .stages
                .get(&stage_name)
                .ok_or_else(|| PipelineError::ItemFailed {
                    stage: stage_name.clone(),
                    item_id: String::new(),
                    error: format!("stage '{}' not found in topology", stage_name),
                })?
                .clone();

            let items = self.collect_items_for_stage(stage_id);
            let ctx = self.build_stage_context(&stage_name);
            let concurrency = self.topology.spec.defaults.concurrency;

            // Mark stage as running.
            if let Some(stage_state) = self.state.stages.get_mut(*stage_id) {
                stage_state.status = StageStatus::Running;
                stage_state.started_at = Some(Utc::now());
            }

            join_set.spawn(async move {
                execute_stage_items(stage, items, ctx, concurrency).await
            });
        }

        // Collect results and merge into state.
        while let Some(result) = join_set.join_next().await {
            let stage_result = result??;
            self.merge_stage_result(stage_result)?;
        }

        Ok(())
    }

    /// Enumerate all sources and populate the item list.
    ///
    /// Calls `SourceAdapter::enumerate()` for each source in the topology.
    /// Creates `ItemState` entries in `PipelineState::sources` for each
    /// discovered item.
    async fn enumerate_sources(&mut self) -> Result<()> {
        for (name, adapter) in &self.topology.sources {
            let items = adapter.enumerate().await.map_err(|e| {
                PipelineError::SourceEnumeration {
                    source: name.clone(),
                    error: e.to_string(),
                }
            })?;

            let source_state = self
                .state
                .sources
                .entry(name.clone())
                .or_insert_with(SourceState::default);

            source_state.items_discovered = items.len();

            for item in items {
                source_state.items_accepted += 1;
                source_state.items.insert(
                    item.id.clone(),
                    ItemState {
                        display_name: item.display_name.clone(),
                        source_id: item.id.clone(),
                        source_name: name.clone(),
                        content_hash: Blake3Hash::new(""),
                        status: ItemStatus::Pending,
                        completed_stages: vec![],
                        provenance: ItemProvenance {
                            source_kind: adapter.source_kind().to_string(),
                            metadata: BTreeMap::new(),
                            source_modified: item.modified_at,
                            extracted_at: Utc::now(),
                        },
                    },
                );
            }
        }
        Ok(())
    }

    /// Compare content hashes against previous run; mark unchanged items.
    ///
    /// Loads hashes from the store's most recent completed run and compares
    /// each item's content hash. Items with matching hashes are marked as
    /// `ItemStatus::Unchanged` and will be skipped during execution.
    async fn apply_incrementality(&mut self) -> Result<()> {
        let previous_hashes = self.store.load_previous_hashes().await?;

        for source_state in self.state.sources.values_mut() {
            let mut skipped = 0usize;
            for (item_id, item_state) in source_state.items.iter_mut() {
                if let Some(prev_hash) = previous_hashes.get(item_id) {
                    if !prev_hash.is_empty() && item_state.content_hash == *prev_hash {
                        item_state.status = ItemStatus::Unchanged;
                        skipped += 1;
                    }
                }
            }
            source_state.items_skipped_unchanged += skipped;
        }
        self.state.update_stats();
        Ok(())
    }

    /// Build a checkpoint and persist it via the state store.
    async fn checkpoint(&mut self) -> Result<()> {
        self.checkpoint_sequence += 1;
        let checkpoint = Checkpoint {
            version: 1,
            sequence: self.checkpoint_sequence,
            created_at: Utc::now(),
            spec: (*self.topology.spec).clone(),
            schedule: self.topology.schedule.clone(),
            spec_hash: self.topology.spec_hash.clone(),
            state: self.state.clone(),
        };
        self.store.save_checkpoint(&checkpoint).await?;
        self.state.last_checkpoint = checkpoint.created_at;
        Ok(())
    }

    /// Save all completed item content hashes for future incrementality.
    async fn save_completed_hashes(&self) -> Result<()> {
        let mut hashes = BTreeMap::new();
        for source_state in self.state.sources.values() {
            for (item_id, item_state) in &source_state.items {
                if matches!(item_state.status, ItemStatus::Completed) {
                    hashes.insert(item_id.clone(), item_state.content_hash.clone());
                }
            }
        }
        self.store
            .save_completed_hashes(&self.state.run_id, &hashes)
            .await?;
        Ok(())
    }

    /// Determine whether a stage should execute.
    ///
    /// Currently always returns true. Condition expression evaluation
    /// is deferred to a future milestone.
    fn should_execute_stage(&self, _stage_id: &StageId) -> bool {
        true
    }

    /// Collect all pending items for a given stage.
    ///
    /// Returns `PipelineItem`s for all items in Pending status across
    /// all sources. Items in Unchanged, Completed, Failed, or Skipped
    /// status are excluded.
    fn collect_items_for_stage(&self, _stage_id: &StageId) -> Vec<PipelineItem> {
        let mut items = Vec::new();
        for source_state in self.state.sources.values() {
            for (_, item_state) in &source_state.items {
                if matches!(item_state.status, ItemStatus::Pending) {
                    items.push(PipelineItem {
                        id: item_state.source_id.clone(),
                        display_name: item_state.display_name.clone(),
                        content: Arc::from(Vec::new().as_slice()),
                        mime_type: String::new(),
                        source_name: item_state.source_name.clone(),
                        source_content_hash: item_state.content_hash.clone(),
                        provenance: item_state.provenance.clone(),
                        metadata: BTreeMap::new(),
                    });
                }
            }
        }
        items
    }

    /// Build a read-only `StageContext` for a stage.
    fn build_stage_context(&self, stage_name: &str) -> StageContext {
        let params = self
            .topology
            .stages
            .get(stage_name)
            .map(|s| {
                self.topology
                    .spec
                    .stages
                    .get(stage_name)
                    .map(|spec| spec.params.clone())
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null);

        StageContext {
            spec: self.topology.spec.clone(),
            output_dir: self.topology.output_dir.clone(),
            params,
            span: tracing::info_span!("stage", name = stage_name),
        }
    }

    /// Merge a stage's results into the pipeline state.
    ///
    /// Updates item statuses (Completed, Failed, Skipped) and stage
    /// aggregate counters.
    fn merge_stage_result(&mut self, result: StageResult) -> Result<()> {
        let stage_id = &result.stage_id;

        let mut items_processed = 0usize;
        let mut items_failed = 0usize;
        let mut items_skipped = 0usize;

        // Record successes.
        for success in &result.successes {
            items_processed += 1;
            for source_state in self.state.sources.values_mut() {
                if let Some(item_state) = source_state.items.get_mut(&success.item_id) {
                    item_state.status = ItemStatus::Completed;
                    item_state.completed_stages.push(
                        ecl_pipeline_state::CompletedStageRecord {
                            stage: stage_id.clone(),
                            completed_at: Utc::now(),
                            duration_ms: 0, // TODO: track actual duration
                        },
                    );
                }
            }
        }

        // Record skips.
        for skipped in &result.skipped {
            items_skipped += 1;
            for source_state in self.state.sources.values_mut() {
                if let Some(item_state) = source_state.items.get_mut(&skipped.item_id) {
                    item_state.status = ItemStatus::Skipped {
                        stage: stage_id.as_str().to_string(),
                        reason: skipped.error.to_string(),
                    };
                }
            }
        }

        // Record failures.
        for failure in &result.failures {
            items_failed += 1;
            for source_state in self.state.sources.values_mut() {
                if let Some(item_state) = source_state.items.get_mut(&failure.item_id) {
                    item_state.status = ItemStatus::Failed {
                        stage: stage_id.as_str().to_string(),
                        error: failure.error.to_string(),
                        attempts: 0, // TODO: track actual attempts
                    };
                }
            }
        }

        // Update stage aggregate state.
        if let Some(stage_state) = self.state.stages.get_mut(stage_id) {
            stage_state.items_processed += items_processed;
            stage_state.items_failed += items_failed;
            stage_state.items_skipped += items_skipped;
            stage_state.completed_at = Some(Utc::now());

            if items_failed > 0 && result.has_failures() {
                // If there are hard failures (not skipped), mark stage as failed.
                stage_state.status = StageStatus::Failed {
                    error: format!(
                        "{} item(s) failed in stage '{}'",
                        items_failed,
                        stage_id.as_str()
                    ),
                };
            } else {
                stage_state.status = StageStatus::Completed;
            }
        }

        // If any items hard-failed, propagate as error.
        if let Some(first_failure) = result.failures.first() {
            return Err(PipelineError::ItemFailed {
                stage: stage_id.as_str().to_string(),
                item_id: first_failure.item_id.clone(),
                error: first_failure.error.to_string(),
            });
        }

        self.state.update_stats();
        Ok(())
    }

    /// Get a reference to the current pipeline state.
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    /// Get a reference to the pipeline topology.
    pub fn topology(&self) -> &PipelineTopology {
        &self.topology
    }
}
```

### `src/lib.rs`

```rust
//! Pipeline execution engine for the ECL pipeline runner.
//!
//! This crate contains the `PipelineRunner` — the orchestrator that
//! executes a resolved pipeline topology. It handles:
//!
//! - Source enumeration
//! - Incremental processing (content hash comparison)
//! - Batch execution with concurrent stages
//! - Per-item bounded concurrency within stages
//! - Retry with exponential backoff
//! - Checkpointing at batch boundaries
//! - Resume from checkpoint after interruption
//!
//! # Usage
//!
//! ```ignore
//! let topology = /* resolve from PipelineSpec */;
//! let store = Box::new(InMemoryStateStore::new());
//! let mut runner = PipelineRunner::new(topology, store).await?;
//! let state = runner.run().await?;
//! ```

pub mod batch;
pub mod error;
pub mod runner;

pub use batch::{
    execute_stage_items, execute_with_retry, StageItemFailure, StageItemSkipped,
    StageItemSuccess, StageResult,
};
pub use error::{PipelineError, Result};
pub use runner::PipelineRunner;
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold crate and verify compilation

- [ ] Create `crates/ecl-pipeline/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create minimal `src/lib.rs` with just `//! Pipeline execution engine.`
- [ ] Add `"crates/ecl-pipeline"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-pipeline` — must pass
- [ ] Commit: `feat(ecl-pipeline): scaffold crate`

### Step 2: Error types

- [ ] Create `src/error.rs` with `PipelineError` enum (from section 6)
- [ ] Add `pub mod error;` and `pub use error::{PipelineError, Result};` to `lib.rs`
- [ ] Write tests:
  - `test_error_display_source_enumeration` — verify Display output includes source name and error
  - `test_error_display_config_drift` — verify Display output includes both hashes
  - `test_error_display_item_failed` — verify Display includes stage, item_id, error
  - `test_error_implements_send_sync` — `PipelineError: Send + Sync` (required for async)
  - `test_error_from_state_error` — `StateError` converts to `PipelineError::StateStore`
  - `test_error_from_stage_error` — `StageError` converts to `PipelineError::StageExecution`
  - `test_error_from_source_error` — `SourceError` converts to `PipelineError::Source`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add PipelineError types`

### Step 3: StageResult type

- [ ] Create `src/batch.rs` with `StageResult`, `StageItemSuccess`, `StageItemSkipped`, `StageItemFailure` (from section 6)
- [ ] Add `pub mod batch;` and re-exports to `lib.rs`
- [ ] Write tests:
  - `test_stage_result_new_is_empty` — new StageResult has empty vecs
  - `test_stage_result_record_success` — record_success adds to successes
  - `test_stage_result_record_skipped` — record_skipped adds to skipped
  - `test_stage_result_record_failure` — record_failure adds to failures
  - `test_stage_result_has_failures_false_when_empty` — no failures
  - `test_stage_result_has_failures_true_when_failure_exists` — has failures
  - `test_stage_result_has_failures_false_when_only_skipped` — skipped items do not count as failures
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add StageResult type`

### Step 4: Mock adapters and stages for testing

Before implementing the runner, create mock types in a test helper module.
These are `#[cfg(test)]` only and live at the bottom of `src/runner.rs`
inside the tests module.

**MockSourceAdapter:**

```rust
/// A mock source adapter that returns fixed SourceItems from enumerate()
/// and fixed ExtractedDocuments from fetch().
#[derive(Debug)]
struct MockSourceAdapter {
    /// The source kind string to return.
    kind: String,
    /// Items to return from enumerate().
    items: Vec<SourceItem>,
}

impl MockSourceAdapter {
    fn new(kind: &str, items: Vec<SourceItem>) -> Self {
        Self {
            kind: kind.to_string(),
            items,
        }
    }
}

#[async_trait::async_trait]
impl SourceAdapter for MockSourceAdapter {
    fn source_kind(&self) -> &str {
        &self.kind
    }

    async fn enumerate(&self) -> std::result::Result<Vec<SourceItem>, SourceError> {
        Ok(self.items.clone())
    }

    async fn fetch(
        &self,
        item: &SourceItem,
    ) -> std::result::Result<ExtractedDocument, SourceError> {
        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content: format!("content-of-{}", item.id).into_bytes(),
            mime_type: item.mime_type.clone(),
            provenance: ItemProvenance {
                source_kind: self.kind.clone(),
                metadata: BTreeMap::new(),
                source_modified: item.modified_at,
                extracted_at: Utc::now(),
            },
            content_hash: Blake3Hash::new("mock-hash"),
        })
    }
}
```

**MockStage:**

```rust
/// A mock stage that transforms items by appending a suffix to their content.
#[derive(Debug)]
struct MockStage {
    name: String,
    suffix: String,
}

impl MockStage {
    fn new(name: &str, suffix: &str) -> Self {
        Self {
            name: name.to_string(),
            suffix: suffix.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Stage for MockStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> std::result::Result<Vec<PipelineItem>, StageError> {
        let mut new_content = item.content.to_vec();
        new_content.extend_from_slice(self.suffix.as_bytes());
        Ok(vec![PipelineItem {
            content: Arc::from(new_content.as_slice()),
            ..item
        }])
    }
}
```

**FailingStage:**

```rust
/// A stage that fails N times, then succeeds. For retry testing.
#[derive(Debug)]
struct FailingStage {
    name: String,
    /// Number of times to fail before succeeding.
    fail_count: std::sync::atomic::AtomicU32,
    /// Total failures to produce before succeeding.
    fail_until: u32,
}

impl FailingStage {
    fn new(name: &str, fail_until: u32) -> Self {
        Self {
            name: name.to_string(),
            fail_count: std::sync::atomic::AtomicU32::new(0),
            fail_until,
        }
    }
}

#[async_trait::async_trait]
impl Stage for FailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> std::result::Result<Vec<PipelineItem>, StageError> {
        let count = self
            .fail_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count < self.fail_until {
            Err(StageError::Transient {
                stage: self.name.clone(),
                item_id: item.id.clone(),
                message: format!("attempt {} failed", count + 1),
            })
        } else {
            Ok(vec![item])
        }
    }
}
```

**AlwaysFailingStage:**

```rust
/// A stage that always fails. For testing skip_on_error and failure propagation.
#[derive(Debug)]
struct AlwaysFailingStage {
    name: String,
}

impl AlwaysFailingStage {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Stage for AlwaysFailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> std::result::Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: self.name.clone(),
            item_id: item.id.clone(),
            message: "always fails".to_string(),
        })
    }
}
```

**Test helper to build a minimal topology:**

```rust
/// Build a minimal PipelineTopology for testing.
///
/// Takes sources (name -> adapter) and stages (name -> (Stage, source_name, skip_on_error))
/// and a simple linear schedule (each stage in its own batch).
fn build_test_topology(
    sources: Vec<(String, Arc<dyn SourceAdapter>)>,
    stages: Vec<(String, Arc<dyn Stage>, Option<String>, bool)>,
) -> PipelineTopology {
    use ecl_pipeline_spec::*;
    use ecl_pipeline_topo::*;
    use std::path::PathBuf;

    // Build a minimal PipelineSpec.
    let mut spec_sources = BTreeMap::new();
    for (name, _) in &sources {
        spec_sources.insert(
            name.clone(),
            SourceSpec::Filesystem(FilesystemSourceSpec {
                root: PathBuf::from("/tmp/test"),
                filters: vec![],
                extensions: vec![],
            }),
        );
    }

    let mut spec_stages = BTreeMap::new();
    for (name, _, source, skip) in &stages {
        spec_stages.insert(
            name.clone(),
            StageSpec {
                adapter: "mock".to_string(),
                source: source.clone(),
                resources: ResourceSpec::default(),
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: *skip,
                condition: None,
            },
        );
    }

    let spec = Arc::new(PipelineSpec {
        name: "test-pipeline".to_string(),
        version: 1,
        output_dir: PathBuf::from("/tmp/test-output"),
        sources: spec_sources,
        stages: spec_stages,
        defaults: DefaultsSpec::default(),
    });

    let topo_sources: BTreeMap<String, Arc<dyn SourceAdapter>> =
        sources.into_iter().collect();

    let mut resolved_stages = BTreeMap::new();
    let mut schedule = Vec::new();
    for (name, handler, source, skip_on_error) in stages {
        let stage_id = StageId::new(&name);
        schedule.push(vec![stage_id.clone()]);
        resolved_stages.insert(
            name,
            ResolvedStage {
                id: stage_id,
                handler,
                retry: RetryPolicy::default(),
                skip_on_error,
                timeout: None,
                source,
                condition: None,
            },
        );
    }

    PipelineTopology {
        spec,
        spec_hash: Blake3Hash::new("test-hash-abc123"),
        sources: topo_sources,
        stages: resolved_stages,
        schedule,
        output_dir: PathBuf::from("/tmp/test-output"),
    }
}
```

- [ ] Add all mock types and `build_test_topology` inside the `#[cfg(test)]` module in `src/runner.rs`
- [ ] Run `cargo check -p ecl-pipeline` — must pass (mocks compile)
- [ ] Commit: `feat(ecl-pipeline): add test mocks and helpers`

### Step 5: execute_with_retry

- [ ] Implement `execute_with_retry()` in `src/batch.rs` (from section 6)
- [ ] Write tests (in `src/batch.rs` test module, also using mock stages):
  - `test_execute_with_retry_succeeds_first_attempt` — MockStage succeeds, returns output
  - `test_execute_with_retry_succeeds_after_failures` — FailingStage(2) succeeds on 3rd attempt with max_attempts=3
  - `test_execute_with_retry_exhausts_retries` — FailingStage(5) fails with max_attempts=3

  Note: These tests need to create `PipelineItem` and `StageContext` instances
  and `Arc<dyn Stage>` from the mock types. Copy the mock types into the
  `batch.rs` test module as well, or use a shared `#[cfg(test)]` module.
  The simplest approach: duplicate the necessary mock types in the `batch.rs`
  test module since they are small.

- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add execute_with_retry`

### Step 6: execute_stage_items

- [ ] Implement `execute_stage_items()` in `src/batch.rs` (from section 6)
- [ ] Write tests:
  - `test_execute_stage_items_all_succeed` — 3 items through MockStage, all succeed
  - `test_execute_stage_items_with_skip_on_error` — AlwaysFailingStage with skip_on_error=true, items recorded as skipped
  - `test_execute_stage_items_without_skip_on_error` — AlwaysFailingStage with skip_on_error=false, items recorded as failures
  - `test_execute_stage_items_respects_concurrency` — verify semaphore bounds concurrent execution (use a stage with a small delay and verify timing)
  - `test_execute_stage_items_empty_items` — empty item list returns empty StageResult
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add execute_stage_items`

### Step 7: PipelineRunner::new

- [ ] Create `src/runner.rs` with `PipelineRunner` struct and `new()` (from section 6)
- [ ] Add `pub mod runner;` and `pub use runner::PipelineRunner;` to `lib.rs`
- [ ] Write tests:
  - `test_runner_new_fresh_state` — no checkpoint in store, creates fresh PipelineState with Pending status
  - `test_runner_new_resumes_from_checkpoint` — store has checkpoint, loads it, state matches
  - `test_runner_new_config_drift_error` — store has checkpoint with different spec_hash, returns ConfigDrift error
  - `test_runner_new_prepares_for_resume` — store has checkpoint with Processing items, they become Pending after new()
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add PipelineRunner::new`

### Step 8: enumerate_sources and apply_incrementality

- [ ] Implement `enumerate_sources()` and `apply_incrementality()` in `src/runner.rs` (from section 6)
- [ ] Write tests:
  - `test_enumerate_sources_populates_items` — MockSourceAdapter returns 3 items, state has 3 items in Pending status
  - `test_enumerate_sources_multiple_sources` — two sources, items from both appear in state
  - `test_enumerate_sources_error_propagates` — a failing source adapter returns SourceEnumeration error
  - `test_apply_incrementality_marks_unchanged` — store has previous hashes matching some items, those items become Unchanged
  - `test_apply_incrementality_no_previous_hashes` — empty previous hashes, all items remain Pending
  - `test_apply_incrementality_updates_stats` — stats reflect unchanged count after application
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add enumerate_sources and apply_incrementality`

### Step 9: checkpoint

- [ ] Implement `checkpoint()` and `save_completed_hashes()` in `src/runner.rs` (from section 6)
- [ ] Write tests:
  - `test_checkpoint_saves_to_store` — after checkpoint(), store.load_checkpoint() returns Some
  - `test_checkpoint_increments_sequence` — multiple checkpoints have increasing sequence numbers
  - `test_checkpoint_embeds_spec_and_schedule` — checkpoint contains topology's spec and schedule
  - `test_save_completed_hashes_stores_completed_items` — only Completed items appear in saved hashes
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline): add checkpoint and save_completed_hashes`

### Step 10: Full run() lifecycle

- [ ] Implement remaining `run()`, `execute_batch()`, `merge_stage_result()`, helper methods in `src/runner.rs` (from section 6)
- [ ] Write tests:
  - `test_run_simple_pipeline_completes` — single source, single stage, runs to completion with Completed status
  - `test_run_multi_stage_pipeline` — single source, two sequential stages, both execute
  - `test_run_sets_completed_status` — after run(), state.status is PipelineStatus::Completed
  - `test_run_checkpoints_after_each_batch` — verify store has checkpoint with correct batch progress
  - `test_run_skip_on_error_continues` — stage with skip_on_error=true, failing items skipped but pipeline completes
  - `test_run_failure_propagates` — stage without skip_on_error, failure returns ItemFailed error
  - `test_run_resume_skips_completed_batches` — create checkpoint at batch 1, resume skips batch 0
  - `test_run_incrementality_skips_unchanged` — store has previous hashes, unchanged items not processed
  - `test_run_empty_sources_completes` — source returns no items, pipeline completes with zero stats
- [ ] Run `make test`, `make lint`, `make format`
- [ ] Commit: `feat(ecl-pipeline): add full run() lifecycle`

### Step 11: Final polish and coverage

- [ ] Run `make test` — all tests pass
- [ ] Run `make lint` — no warnings
- [ ] Run `make format` — no changes
- [ ] Check code coverage, add tests to reach 95%+ if needed. Potential gap areas:
  - Error conversion paths (From impls)
  - Edge cases in merge_stage_result (skipped + failed in same batch)
  - should_execute_stage returning true (trivial but needs coverage)
  - collect_items_for_stage filtering non-Pending items
  - build_stage_context with missing stage name
- [ ] Verify all public items have doc comments
- [ ] Commit: `feat(ecl-pipeline): coverage and polish`

## 8. Test Fixtures

### Minimal SourceItem

```rust
fn make_source_item(id: &str) -> SourceItem {
    SourceItem {
        id: id.to_string(),
        display_name: format!("Item {}", id),
        mime_type: "text/plain".to_string(),
        path: format!("/test/{}", id),
        modified_at: None,
        source_hash: None,
    }
}
```

### Minimal PipelineItem

```rust
fn make_pipeline_item(id: &str) -> PipelineItem {
    PipelineItem {
        id: id.to_string(),
        display_name: format!("Item {}", id),
        content: Arc::from(format!("content-{}", id).as_bytes()),
        mime_type: "text/plain".to_string(),
        source_name: "test-source".to_string(),
        source_content_hash: Blake3Hash::new(""),
        provenance: ItemProvenance {
            source_kind: "test".to_string(),
            metadata: BTreeMap::new(),
            source_modified: None,
            extracted_at: Utc::now(),
        },
        metadata: BTreeMap::new(),
    }
}
```

### Minimal StageContext

```rust
fn make_stage_context() -> StageContext {
    StageContext {
        spec: Arc::new(PipelineSpec {
            name: "test".to_string(),
            version: 1,
            output_dir: PathBuf::from("/tmp/test"),
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            defaults: DefaultsSpec::default(),
        }),
        output_dir: PathBuf::from("/tmp/test"),
        params: serde_json::Value::Null,
        span: tracing::info_span!("test"),
    }
}
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_error_display_source_enumeration` | `error` | `PipelineError::SourceEnumeration` Display output includes source and error |
| `test_error_display_config_drift` | `error` | `PipelineError::ConfigDrift` Display includes both hashes |
| `test_error_display_item_failed` | `error` | `PipelineError::ItemFailed` Display includes stage, item_id, error |
| `test_error_implements_send_sync` | `error` | `PipelineError: Send + Sync` |
| `test_error_from_state_error` | `error` | `StateError` converts via From |
| `test_error_from_stage_error` | `error` | `StageError` converts via From |
| `test_error_from_source_error` | `error` | `SourceError` converts via From |
| `test_stage_result_new_is_empty` | `batch` | New StageResult has empty vecs |
| `test_stage_result_record_success` | `batch` | record_success adds to successes |
| `test_stage_result_record_skipped` | `batch` | record_skipped adds to skipped |
| `test_stage_result_record_failure` | `batch` | record_failure adds to failures |
| `test_stage_result_has_failures_false_when_empty` | `batch` | Empty result has no failures |
| `test_stage_result_has_failures_true_when_failure_exists` | `batch` | Failure recorded means has_failures is true |
| `test_stage_result_has_failures_false_when_only_skipped` | `batch` | Skipped items do not count as failures |
| `test_execute_with_retry_succeeds_first_attempt` | `batch` | Successful stage returns immediately |
| `test_execute_with_retry_succeeds_after_failures` | `batch` | FailingStage(2) succeeds on 3rd attempt |
| `test_execute_with_retry_exhausts_retries` | `batch` | FailingStage(5) fails after max_attempts=3 |
| `test_execute_stage_items_all_succeed` | `batch` | 3 items through MockStage, all in successes |
| `test_execute_stage_items_with_skip_on_error` | `batch` | AlwaysFailingStage + skip_on_error, items in skipped |
| `test_execute_stage_items_without_skip_on_error` | `batch` | AlwaysFailingStage, items in failures |
| `test_execute_stage_items_respects_concurrency` | `batch` | Semaphore bounds parallelism |
| `test_execute_stage_items_empty_items` | `batch` | Empty input returns empty StageResult |
| `test_runner_new_fresh_state` | `runner` | No checkpoint creates fresh PipelineState |
| `test_runner_new_resumes_from_checkpoint` | `runner` | Checkpoint loaded and state restored |
| `test_runner_new_config_drift_error` | `runner` | Mismatched spec_hash returns ConfigDrift |
| `test_runner_new_prepares_for_resume` | `runner` | Processing items reset to Pending |
| `test_enumerate_sources_populates_items` | `runner` | 3 items from adapter appear in state |
| `test_enumerate_sources_multiple_sources` | `runner` | Items from two sources both present |
| `test_enumerate_sources_error_propagates` | `runner` | Failing adapter returns SourceEnumeration |
| `test_apply_incrementality_marks_unchanged` | `runner` | Matching hashes set ItemStatus::Unchanged |
| `test_apply_incrementality_no_previous_hashes` | `runner` | Empty hashes, all remain Pending |
| `test_apply_incrementality_updates_stats` | `runner` | Stats reflect unchanged count |
| `test_checkpoint_saves_to_store` | `runner` | Store contains checkpoint after call |
| `test_checkpoint_increments_sequence` | `runner` | Sequence increases with each checkpoint |
| `test_checkpoint_embeds_spec_and_schedule` | `runner` | Checkpoint spec and schedule match topology |
| `test_save_completed_hashes_stores_completed_items` | `runner` | Only Completed items in saved hashes |
| `test_run_simple_pipeline_completes` | `runner` | Single source + stage runs to Completed |
| `test_run_multi_stage_pipeline` | `runner` | Two sequential stages both execute |
| `test_run_sets_completed_status` | `runner` | Final status is PipelineStatus::Completed |
| `test_run_checkpoints_after_each_batch` | `runner` | Store checkpoint reflects batch progress |
| `test_run_skip_on_error_continues` | `runner` | skip_on_error stage skips items, pipeline completes |
| `test_run_failure_propagates` | `runner` | Hard failure returns ItemFailed error |
| `test_run_resume_skips_completed_batches` | `runner` | Checkpoint at batch 1 skips batch 0 |
| `test_run_incrementality_skips_unchanged` | `runner` | Unchanged items not processed by stages |
| `test_run_empty_sources_completes` | `runner` | Zero items, pipeline still completes |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline` passes
- [ ] `cargo test -p ecl-pipeline` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement full topology resolution from `PipelineSpec` (that is milestone 2.3 — this crate takes a pre-resolved `PipelineTopology`)
- Do NOT implement concrete source adapters (Google Drive, Slack, Filesystem) (that is milestone 3.1+)
- Do NOT implement concrete stage implementations (normalize, emit, etc.) (that is milestone 3.1+)
- Do NOT implement the redb `StateStore` (that is milestone 2.1 — tests use `InMemoryStateStore`)
- Do NOT implement CLI commands (that is milestone 5.1)
- Do NOT implement condition expression evaluation (deferred — `should_execute_stage()` always returns true)
- Do NOT add any crates beyond `ecl-pipeline`
