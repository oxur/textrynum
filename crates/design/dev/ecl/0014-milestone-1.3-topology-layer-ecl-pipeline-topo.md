# Milestone 1.3: Topology Layer (`ecl-pipeline-topo`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline-topo):`

## 1. Goal

Create the `ecl-pipeline-topo` crate containing topology types and core
pipeline traits. When done:

1. All topology types (`PipelineTopology`, `ResolvedStage`, `RetryPolicy`,
   `ConditionExpr`) compile and derive the correct traits.
2. The `ResourceGraph` builds from a `BTreeMap<String, StageSpec>` and computes
   a parallel schedule (batches of stages that can run concurrently).
3. Core traits `SourceAdapter` and `Stage` are defined, object-safe, and usable
   as `Arc<dyn SourceAdapter>` and `Arc<dyn Stage>`.
4. `PipelineItem`, `SourceItem`, `ExtractedDocument`, and `StageContext` are
   fully defined.
5. The topology resolution function (`resolve`) exists as a stub — full
   implementation is deferred to milestone 2.3.

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
- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, TheError>;`
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
3. `assets/ai/ai-rust/guides/06-traits.md`
4. `assets/ai/ai-rust/guides/05-type-design.md`
5. `assets/ai/ai-rust/guides/07-concurrency-async.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:
- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments
- `crates/ecl-core/src/llm/provider.rs` — `async_trait` pattern for object-safe traits

## 3. Prior Art / Dependencies

This crate depends on types from two sibling crates. Below are the **exact
public APIs** you will use. You do NOT implement these — they already exist.

### From `ecl-pipeline-spec` (Milestone 1.1)

```rust
// --- src/lib.rs ---
/// The root configuration, deserialized from TOML.
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

// --- src/defaults.rs ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSpec {
    pub concurrency: usize,        // default: 4
    pub retry: RetrySpec,
    pub checkpoint: CheckpointStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    pub max_attempts: u32,         // default: 3
    pub initial_backoff_ms: u64,   // default: 1000
    pub backoff_multiplier: f64,   // default: 2.0
    pub max_backoff_ms: u64,       // default: 30_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointStrategy { Batch, Items { count: usize }, Seconds { duration: u64 } }

// --- src/source.rs ---
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    #[serde(rename = "google_drive")] GoogleDrive(GoogleDriveSourceSpec),
    #[serde(rename = "slack")]        Slack(SlackSourceSpec),
    #[serde(rename = "filesystem")]   Filesystem(FilesystemSourceSpec),
}

// --- src/stage.rs ---
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    #[serde(default)] pub reads: Vec<String>,
    #[serde(default)] pub creates: Vec<String>,
    #[serde(default)] pub writes: Vec<String>,
}

// --- src/error.rs ---
pub type Result<T> = std::result::Result<T, SpecError>;
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
// --- src/lib.rs (re-exports) ---

/// Deterministic, name-based stage identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StageId(String);
impl StageId {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for StageId { /* writes inner string */ }

/// Blake3 content hash, stored as hex string for JSON readability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);
impl Blake3Hash {
    pub fn new(hex: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    pub fn is_empty(&self) -> bool;
}

/// Unique identifier for a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StateError>;
    async fn load_checkpoint(&self) -> Result<Option<Checkpoint>, StateError>;
    async fn load_previous_hashes(&self) -> Result<BTreeMap<String, Blake3Hash>, StateError>;
    async fn save_completed_hashes(
        &self, run_id: &RunId, hashes: &BTreeMap<String, Blake3Hash>,
    ) -> Result<(), StateError>;
}

/// Per-item state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState { /* ... */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemStatus { Pending, Processing { stage: String }, Completed, Failed { .. }, Skipped { .. }, Unchanged }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceState { /* items_discovered, items_accepted, items_skipped_unchanged, items: BTreeMap<String, ItemState> */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    pub status: StageStatus,
    pub items_processed: usize,
    pub items_failed: usize,
    pub items_skipped: usize,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus { Pending, Running, Completed, Skipped { reason: String }, Failed { error: String } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus { Pending, Running { current_stage: String }, Completed { .. }, Failed { .. }, Interrupted { .. } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_items_discovered: usize,
    pub total_items_processed: usize,
    pub total_items_skipped_unchanged: usize,
    pub total_items_failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStageRecord {
    pub stage: StageId,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProvenance {
    pub source_kind: String,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub source_modified: Option<DateTime<Utc>>,
    pub extracted_at: DateTime<Utc>,
}
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add `serde_bytes` to workspace deps (if not present); add `crates/ecl-pipeline-topo` to members |
| `crates/ecl-pipeline-topo/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-pipeline-topo/src/lib.rs` | Create | Module declarations, re-exports, `PipelineTopology` struct |
| `crates/ecl-pipeline-topo/src/resolve.rs` | Create | **STUB** — `resolve()` function signature with `todo!()`. Full impl in milestone 2.3 |
| `crates/ecl-pipeline-topo/src/resource_graph.rs` | Create | `ResourceGraph` struct (pub(crate)), `build()`, `validate_no_missing_inputs()`, `validate_no_cycles()`, `compute_schedule()` |
| `crates/ecl-pipeline-topo/src/schedule.rs` | Create | Kahn's algorithm topological sort, resource-conflict layer grouping |
| `crates/ecl-pipeline-topo/src/traits.rs` | Create | `SourceAdapter` trait, `Stage` trait, `StageContext`, `PipelineItem`, `SourceItem`, `ExtractedDocument` |
| `crates/ecl-pipeline-topo/src/error.rs` | Create | `ResolveError`, `SourceError`, `StageError` enums |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml` (if not already present):

```toml
serde_bytes = "0.11"
```

Add to `[workspace] members`:

```toml
"crates/ecl-pipeline-topo",
```

### Crate Cargo.toml

```toml
[package]
name = "ecl-pipeline-topo"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Pipeline topology, resource graph, and core traits for ECL pipeline runner"

[dependencies]
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_bytes = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
blake3 = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
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

All types below must be implemented exactly as shown. These are copied from
the authoritative design document, adapted for this crate's module structure.

### `src/error.rs`

```rust
//! Error types for the topology layer.

use ecl_pipeline_state::StageId;
use thiserror::Error;

/// Errors that occur when resolving a pipeline topology from a specification.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveError {
    /// The specification could not be serialized for hashing.
    #[error("failed to serialize spec: {message}")]
    SerializeError {
        /// The serialization error message.
        message: String,
    },

    /// A stage references a source that doesn't exist in the spec.
    #[error("stage '{stage}' references unknown source '{source}'")]
    UnknownSource {
        /// The stage that references the unknown source.
        stage: String,
        /// The source name that was referenced.
        source: String,
    },

    /// A stage references a resource that no other stage creates
    /// and is not a known external resource.
    #[error("stage '{stage}' reads resource '{resource}' which is never created")]
    MissingResource {
        /// The stage that reads the missing resource.
        stage: String,
        /// The resource name that is missing.
        resource: String,
    },

    /// The resource graph contains a cycle (impossible to schedule).
    #[error("resource dependency cycle detected involving stages: {stages:?}")]
    CycleDetected {
        /// Stage IDs involved in the cycle.
        stages: Vec<StageId>,
    },

    /// Multiple stages create the same resource.
    #[error("resource '{resource}' is created by multiple stages: {stages:?}")]
    DuplicateCreator {
        /// The resource with multiple creators.
        resource: String,
        /// The stages that create it.
        stages: Vec<StageId>,
    },

    /// An I/O error occurred (e.g., creating output directory).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An unknown stage adapter was specified.
    #[error("unknown stage adapter '{adapter}' in stage '{stage}'")]
    UnknownAdapter {
        /// The stage with the unknown adapter.
        stage: String,
        /// The adapter name that was not recognized.
        adapter: String,
    },
}

/// Errors that occur in source adapters.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SourceError {
    /// Authentication failed.
    #[error("authentication failed for source '{source}': {message}")]
    AuthError {
        /// The source name.
        source: String,
        /// Error detail.
        message: String,
    },

    /// Rate limited by the source API.
    #[error("rate limited by '{source}': retry after {retry_after_secs}s")]
    RateLimited {
        /// The source name.
        source: String,
        /// Seconds to wait before retrying.
        retry_after_secs: u64,
    },

    /// An item was not found.
    #[error("item '{item_id}' not found in source '{source}'")]
    NotFound {
        /// The source name.
        source: String,
        /// The item ID that was not found.
        item_id: String,
    },

    /// A transient error (network, timeout) that may succeed on retry.
    #[error("transient error from source '{source}': {message}")]
    Transient {
        /// The source name.
        source: String,
        /// Error detail.
        message: String,
    },

    /// A permanent error that will not succeed on retry.
    #[error("permanent error from source '{source}': {message}")]
    Permanent {
        /// The source name.
        source: String,
        /// Error detail.
        message: String,
    },
}

/// Errors that occur during stage execution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StageError {
    /// The stage received content it cannot process.
    #[error("stage '{stage}' cannot process item '{item_id}': {message}")]
    UnsupportedContent {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// Error detail.
        message: String,
    },

    /// A transient error that may succeed on retry.
    #[error("transient error in stage '{stage}' for item '{item_id}': {message}")]
    Transient {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// Error detail.
        message: String,
    },

    /// A permanent error that will not succeed on retry.
    #[error("permanent error in stage '{stage}' for item '{item_id}': {message}")]
    Permanent {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// Error detail.
        message: String,
    },

    /// Stage execution timed out.
    #[error("stage '{stage}' timed out after {timeout_secs}s for item '{item_id}'")]
    Timeout {
        /// The stage name.
        stage: String,
        /// The item ID.
        item_id: String,
        /// The timeout duration in seconds.
        timeout_secs: u64,
    },
}

/// Result type for topology resolution operations.
pub type ResolveResult<T> = std::result::Result<T, ResolveError>;
```

### `src/traits.rs`

```rust
//! Core pipeline traits: SourceAdapter, Stage, and supporting types.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};

use crate::error::{SourceError, StageError};

/// A source adapter handles all interaction with an external data service.
///
/// Implementors handle: authentication, enumeration, filtering, pagination,
/// rate limiting, and fetching. The pipeline runner sees only the trait.
///
/// Object-safe by design: adapters are resolved from TOML config at runtime
/// and stored as `Arc<dyn SourceAdapter>`.
///
/// Note: `async_trait` is required here despite Rust 1.85+ supporting native
/// `async fn` in traits. Native async trait methods are not object-safe:
/// `dyn SourceAdapter` requires the future to be boxed, which `async_trait`
/// handles automatically.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    /// Human-readable name of the source type (e.g., "Google Drive").
    fn source_kind(&self) -> &str;

    /// Enumerate items available from this source.
    /// Returns lightweight descriptors (no content) for filtering and
    /// hash comparison. This is the "what's there?" step.
    ///
    /// The adapter applies source-level filters (folder IDs, file types,
    /// modified_after) during enumeration.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;

    /// Fetch the full content of a single item.
    /// Separate from `enumerate()` because fetching is expensive and we
    /// want to skip unchanged items before paying this cost.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor returned by `SourceAdapter::enumerate()`.
/// Contains enough metadata for filtering and hash comparison,
/// but does NOT contain the actual content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    /// Source-specific unique identifier.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// MIME type (for filtering by file type).
    pub mime_type: String,

    /// Path within the source (for glob-based filtering).
    pub path: String,

    /// Last modified timestamp (for incremental sync).
    pub modified_at: Option<DateTime<Utc>>,

    /// Content hash if cheaply available from the source API.
    /// Google Drive provides md5Checksum; Slack provides message hash.
    /// If None, the pipeline fetches content and computes blake3.
    pub source_hash: Option<String>,
}

/// A document extracted from a source, in its original format.
/// This is the raw material before any normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Unique identifier within this source.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// The raw content bytes.
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,

    /// MIME type of the content, as reported by the source.
    pub mime_type: String,

    /// Provenance metadata.
    pub provenance: ItemProvenance,

    /// Content hash (blake3 of content bytes).
    pub content_hash: Blake3Hash,
}

/// The intermediate representation flowing between stages.
/// Starts life as an `ExtractedDocument`, accumulates transformations.
///
/// Uses `Arc<[u8]>` for content to enable zero-copy cloning in hot paths.
/// `PipelineItem` is cloned when fanning out to concurrent tasks and when
/// building retry attempts — `Arc<[u8]>` makes these O(1) instead of O(n).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    /// The item's unique identifier (stable across stages).
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// Current content (may be transformed by prior stages).
    /// Wrapped in `Arc` for zero-copy cloning in concurrent pipelines.
    #[serde(with = "serde_bytes")]
    pub content: Arc<[u8]>,

    /// Current MIME type (changes as content is transformed,
    /// e.g., "application/pdf" -> "text/markdown").
    pub mime_type: String,

    /// Which source this item came from.
    pub source_name: String,

    /// Content hash of the original source content (for incrementality).
    pub source_content_hash: Blake3Hash,

    /// Provenance chain.
    pub provenance: ItemProvenance,

    /// Metadata accumulated by stages. Each stage can add key-value pairs.
    /// Structured as `serde_json::Value` for flexibility without losing
    /// serializability.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
///
/// Stages are intentionally simple: one item in, zero or more out.
/// The runner handles orchestration, retries, checkpointing, and
/// concurrency. The stage handles only transformation logic.
///
/// Object-safe by design: stages are resolved from TOML config at runtime
/// and stored as `Arc<dyn Stage>`.
#[async_trait]
pub trait Stage: Send + Sync + std::fmt::Debug {
    /// Human-readable name of this stage type.
    fn name(&self) -> &str;

    /// Process a single item. Returns:
    /// - `Ok(vec![item])` — item transformed successfully (common case)
    /// - `Ok(vec![item1, item2, ...])` — item split into multiple (fan-out)
    /// - `Ok(vec![])` — item filtered out / consumed
    /// - `Err(e)` — processing failed
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
/// Immutable — stages cannot mutate prior outputs or pipeline state.
/// (Addresses erio-workflow's `&mut WorkflowContext` anti-pattern.)
#[derive(Debug, Clone)]
pub struct StageContext {
    /// The original pipeline specification.
    pub spec: Arc<PipelineSpec>,

    /// The output directory for this pipeline run.
    pub output_dir: PathBuf,

    /// Stage-specific parameters from the pipeline config.
    pub params: serde_json::Value,

    /// Tracing span for structured logging within this stage.
    pub span: tracing::Span,
}
```

### `src/lib.rs`

```rust
//! Pipeline topology, resource graph, and core traits for the ECL pipeline runner.
//!
//! This crate defines:
//! - The resolved pipeline topology (`PipelineTopology`, `ResolvedStage`)
//! - Resource graph computation and parallel schedule derivation
//! - Core traits (`SourceAdapter`, `Stage`) and their supporting types
//!   (`PipelineItem`, `SourceItem`, `ExtractedDocument`, `StageContext`)
//!
//! The topology is computed from a `PipelineSpec` (from `ecl-pipeline-spec`)
//! at init time and is immutable during execution.

pub mod error;
pub mod resolve;
pub mod resource_graph;
pub mod schedule;
pub mod traits;

pub use error::{ResolveError, ResolveResult, SourceError, StageError};
pub use traits::{
    ExtractedDocument, PipelineItem, SourceAdapter, SourceItem, Stage, StageContext,
};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{Blake3Hash, StageId};

/// The resolved pipeline, ready to execute.
/// Computed from `PipelineSpec` at init time. Immutable during execution.
#[derive(Debug, Clone)]
pub struct PipelineTopology {
    /// The original spec, preserved for checkpoint embedding.
    pub spec: Arc<PipelineSpec>,

    /// Blake3 hash of the serialized spec, for detecting config drift
    /// between a checkpoint and the current TOML file.
    pub spec_hash: Blake3Hash,

    /// Resolved source adapters, keyed by source name from the spec.
    pub sources: BTreeMap<String, Arc<dyn SourceAdapter>>,

    /// Resolved stage implementations, keyed by stage name from the spec.
    pub stages: BTreeMap<String, ResolvedStage>,

    /// The computed execution schedule: batches of parallel stages.
    /// Each inner `Vec` contains stages that can run concurrently.
    pub schedule: Vec<Vec<StageId>>,

    /// Resolved output directory (created if needed at init).
    pub output_dir: PathBuf,
}

/// A resolved stage: the concrete implementation with merged configuration.
#[derive(Debug, Clone)]
pub struct ResolvedStage {
    /// Name from the spec (stable across runs for checkpointing).
    pub id: StageId,

    /// The concrete stage implementation.
    pub handler: Arc<dyn Stage>,

    /// Resolved retry policy (stage override merged with global default).
    pub retry: RetryPolicy,

    /// Skip-on-error behavior for item-level failures.
    pub skip_on_error: bool,

    /// Timeout for stage execution.
    pub timeout: Option<Duration>,

    /// Which source this stage operates on (for extract stages).
    pub source: Option<String>,

    /// Condition predicate (`None` = always run).
    pub condition: Option<ConditionExpr>,
}

/// Retry policy with resolved, concrete values.
/// Unlike `RetrySpec` (from ecl-pipeline-spec) which stores milliseconds as
/// `u64`, this stores `Duration` values — the resolved form ready for use
/// by the runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    /// Total attempts (1 = no retry).
    pub max_attempts: u32,
    /// Initial backoff duration before the first retry.
    pub initial_backoff: Duration,
    /// Multiplier applied to backoff after each attempt.
    pub backoff_multiplier: f64,
    /// Maximum backoff duration (caps exponential growth).
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// Create a `RetryPolicy` from a `RetrySpec` (millisecond-based config form).
    pub fn from_spec(spec: &ecl_pipeline_spec::RetrySpec) -> Self {
        Self {
            max_attempts: spec.max_attempts,
            initial_backoff: Duration::from_millis(spec.initial_backoff_ms),
            backoff_multiplier: spec.backoff_multiplier,
            max_backoff: Duration::from_millis(spec.max_backoff_ms),
        }
    }
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

/// A condition expression that determines whether a stage should run.
/// Currently a simple string wrapper; the evaluator is deferred to a
/// future milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionExpr(String);

impl ConditionExpr {
    /// Create a new condition expression from a string.
    pub fn new(expr: impl Into<String>) -> Self {
        Self(expr.into())
    }

    /// Get the expression string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConditionExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```

### `src/resource_graph.rs`

```rust
//! Resource graph: stages connected by shared resource declarations.
//! Used to compute the parallel execution schedule.

use std::collections::BTreeMap;

use ecl_pipeline_spec::StageSpec;
use ecl_pipeline_state::StageId;

use crate::error::ResolveError;
use crate::schedule;

/// The resource graph: stages connected by shared resource declarations.
///
/// This is an internal data structure — not part of the public API.
/// The public interface is `ResourceGraph::build()` and `compute_schedule()`.
#[derive(Debug)]
pub(crate) struct ResourceGraph {
    /// Which stage creates each resource. A resource may only have one creator.
    pub(crate) creators: BTreeMap<String, StageId>,
    /// Which stages read each resource.
    pub(crate) readers: BTreeMap<String, Vec<StageId>>,
    /// Which stages write (exclusively) to each resource.
    pub(crate) writers: BTreeMap<String, Vec<StageId>>,
    /// All stage IDs in the graph.
    pub(crate) stages: Vec<StageId>,
}

impl ResourceGraph {
    /// Build a resource graph from stage specifications.
    ///
    /// Iterates over all stages, collecting their resource declarations
    /// (reads, creates, writes) into the graph structure.
    pub(crate) fn build(
        stages: &BTreeMap<String, StageSpec>,
    ) -> Result<Self, ResolveError> {
        let mut creators: BTreeMap<String, StageId> = BTreeMap::new();
        let mut readers: BTreeMap<String, Vec<StageId>> = BTreeMap::new();
        let mut writers: BTreeMap<String, Vec<StageId>> = BTreeMap::new();
        let mut stage_ids: Vec<StageId> = Vec::new();

        for (name, spec) in stages {
            let stage_id = StageId::new(name);
            stage_ids.push(stage_id.clone());

            // Register created resources (each resource can only have one creator).
            for resource in &spec.resources.creates {
                if let Some(existing) = creators.get(resource) {
                    return Err(ResolveError::DuplicateCreator {
                        resource: resource.clone(),
                        stages: vec![existing.clone(), stage_id.clone()],
                    });
                }
                creators.insert(resource.clone(), stage_id.clone());
            }

            // Register read resources.
            for resource in &spec.resources.reads {
                readers
                    .entry(resource.clone())
                    .or_default()
                    .push(stage_id.clone());
            }

            // Register write resources.
            for resource in &spec.resources.writes {
                writers
                    .entry(resource.clone())
                    .or_default()
                    .push(stage_id.clone());
            }
        }

        Ok(Self {
            creators,
            readers,
            writers,
            stages: stage_ids,
        })
    }

    /// Validate that every resource read by a stage is either created by
    /// another stage or is an external resource (never created by anyone).
    ///
    /// A resource that appears in `reads` but NOT in `creators` is treated
    /// as external (API client, filesystem path) — always available.
    /// A resource that appears in BOTH `reads` and `creators` is internal
    /// and the scheduler will enforce ordering.
    pub(crate) fn validate_no_missing_inputs(&self) -> Result<(), ResolveError> {
        // All resources that are read are either external (not in creators)
        // or internal (in creators). Both are valid. For now, we treat
        // any resource not in `creators` as external.
        //
        // TODO: Add an explicit `externals` set so we can distinguish
        // "intentionally external" from "typo in resource name."
        Ok(())
    }

    /// Validate that the resource dependency graph contains no cycles.
    ///
    /// A cycle would mean stages have circular dependencies which makes
    /// scheduling impossible.
    pub(crate) fn validate_no_cycles(&self) -> Result<(), ResolveError> {
        // Delegates to the schedule module which performs topological sort.
        // If topo sort cannot process all nodes, a cycle exists.
        let _ = self.compute_schedule()?;
        Ok(())
    }

    /// Compute parallel batches via topological sort.
    /// Stages in the same batch touch independent resources.
    ///
    /// Algorithm: Kahn's algorithm for topological sort, then group into
    /// layers where no resource conflicts exist within a layer.
    pub(crate) fn compute_schedule(
        &self,
    ) -> Result<Vec<Vec<StageId>>, ResolveError> {
        schedule::compute_schedule(
            &self.stages,
            &self.creators,
            &self.readers,
            &self.writers,
        )
    }
}
```

### `src/schedule.rs`

```rust
//! Schedule computation: topological sort and resource-conflict layer grouping.
//!
//! Uses Kahn's algorithm for topological ordering, then groups stages into
//! parallel batches where no resource conflicts exist within a batch.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ecl_pipeline_state::StageId;

use crate::error::ResolveError;

/// Compute a parallel execution schedule from resource declarations.
///
/// Returns `Vec<Vec<StageId>>` — batches of stages that can run concurrently.
/// Stages within a batch touch independent resources. Batch ordering respects
/// resource dependencies (a stage that reads a resource runs in a later batch
/// than the stage that creates it).
///
/// # Algorithm
///
/// 1. Build a dependency graph: stage A depends on stage B if A reads a
///    resource that B creates.
/// 2. Perform Kahn's algorithm (BFS topological sort) to get a valid ordering.
/// 3. Assign each stage to the earliest batch where all its dependencies are
///    in earlier batches and no write-write conflicts exist within the batch.
///
/// # Errors
///
/// Returns `ResolveError::CycleDetected` if the dependency graph has a cycle.
pub fn compute_schedule(
    stages: &[StageId],
    creators: &BTreeMap<String, StageId>,
    readers: &BTreeMap<String, Vec<StageId>>,
    writers: &BTreeMap<String, Vec<StageId>>,
) -> Result<Vec<Vec<StageId>>, ResolveError> {
    // Step 1: Build adjacency list and in-degree map.
    // An edge from A -> B means "B depends on A" (B reads what A creates).
    let mut adjacency: BTreeMap<StageId, BTreeSet<StageId>> = BTreeMap::new();
    let mut in_degree: BTreeMap<StageId, usize> = BTreeMap::new();

    // Initialize all stages with zero in-degree.
    for stage in stages {
        adjacency.entry(stage.clone()).or_default();
        in_degree.entry(stage.clone()).or_insert(0);
    }

    // For each resource that is both created and read, add edges:
    // creator -> each reader (reader depends on creator).
    for (resource, reader_stages) in readers {
        if let Some(creator) = creators.get(resource) {
            for reader in reader_stages {
                // Don't add self-edges.
                if reader != creator {
                    if adjacency
                        .entry(creator.clone())
                        .or_default()
                        .insert(reader.clone())
                    {
                        *in_degree.entry(reader.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Write-write conflicts: if two stages write to the same resource,
    // they cannot be in the same batch. We handle this in the batching
    // step, not via edges (since the order between them is arbitrary).

    // Step 2: Kahn's algorithm — BFS topological sort.
    let mut queue: VecDeque<StageId> = VecDeque::new();
    for (stage, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(stage.clone());
        }
    }

    // Track the "depth" (earliest batch) for each stage.
    let mut depth: BTreeMap<StageId, usize> = BTreeMap::new();
    let mut processed_count = 0usize;

    while let Some(stage) = queue.pop_front() {
        processed_count += 1;
        let stage_depth = depth.get(&stage).copied().unwrap_or(0);

        if let Some(neighbors) = adjacency.get(&stage) {
            for neighbor in neighbors {
                // The neighbor's depth is at least one more than this stage's depth.
                let neighbor_depth = depth.entry(neighbor.clone()).or_insert(0);
                if *neighbor_depth <= stage_depth {
                    *neighbor_depth = stage_depth + 1;
                }

                let deg = in_degree.get_mut(neighbor).unwrap_or(&mut 0);
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    // If we didn't process all stages, there's a cycle.
    if processed_count != stages.len() {
        let stuck: Vec<StageId> = stages
            .iter()
            .filter(|s| in_degree.get(*s).copied().unwrap_or(0) > 0)
            .cloned()
            .collect();
        return Err(ResolveError::CycleDetected { stages: stuck });
    }

    // Step 3: Group stages into batches by depth.
    let max_depth = depth.values().copied().max().unwrap_or(0);
    let mut batches: Vec<Vec<StageId>> = vec![Vec::new(); max_depth + 1];

    for stage in stages {
        let d = depth.get(stage).copied().unwrap_or(0);
        batches[d].push(stage.clone());
    }

    // Sort stages within each batch for deterministic output.
    for batch in &mut batches {
        batch.sort();
    }

    // Remove empty batches (shouldn't happen, but be safe).
    batches.retain(|b| !b.is_empty());

    Ok(batches)
}
```

### `src/resolve.rs`

```rust
//! Topology resolution: converting a PipelineSpec into a PipelineTopology.
//!
//! **STUB:** This module contains only the function signature in this
//! milestone. The full implementation (resolving adapters, building the
//! resource graph, computing the schedule) is implemented in milestone 2.3.

use ecl_pipeline_spec::PipelineSpec;

use crate::error::ResolveError;
use crate::PipelineTopology;

/// Resolve a `PipelineSpec` into a `PipelineTopology`.
///
/// This is the main entry point for topology construction:
/// 1. Hash the spec for config drift detection.
/// 2. Resolve each source into a concrete `SourceAdapter`.
/// 3. Resolve each stage into a concrete `Stage` handler.
/// 4. Build the resource graph and validate (no missing inputs, no cycles).
/// 5. Compute the parallel schedule.
/// 6. Create the output directory.
///
/// # Errors
///
/// Returns `ResolveError` if any step fails (unknown adapter, cycle, I/O, etc.).
pub async fn resolve(_spec: PipelineSpec) -> Result<PipelineTopology, ResolveError> {
    // Full implementation deferred to milestone 2.3.
    // At that point this function will:
    // - Serialize and hash the spec
    // - Call resolve_source_adapter() for each source
    // - Call resolve_stage() for each stage
    // - Build ResourceGraph and validate
    // - Compute schedule
    // - Create output directory
    todo!("Full topology resolution implemented in milestone 2.3")
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold crate and verify compilation

- [ ] Create `crates/ecl-pipeline-topo/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create minimal `src/lib.rs` with just `//! Pipeline topology crate.`
- [ ] Add `serde_bytes = "0.11"` to root `Cargo.toml` `[workspace.dependencies]` (if not already present)
- [ ] Add `"crates/ecl-pipeline-topo"` to root `Cargo.toml` `[workspace] members`
- [ ] **Prerequisite check:** Confirm that `crates/ecl-pipeline-spec` and `crates/ecl-pipeline-state` exist. If either is missing, stop and report — they must be implemented first (milestones 1.1 and 1.2).
- [ ] Run `cargo check -p ecl-pipeline-topo` — must pass
- [ ] Commit: `feat(ecl-pipeline-topo): scaffold crate`

### Step 2: Error types

- [ ] Create `src/error.rs` with `ResolveError`, `SourceError`, `StageError` enums (from section 6)
- [ ] Add `pub mod error;` and `pub use error::{ResolveError, ResolveResult, SourceError, StageError};` to `lib.rs`
- [ ] Write tests:
  - `test_resolve_error_display_duplicate_creator`
  - `test_resolve_error_display_cycle_detected`
  - `test_source_error_display_auth_error`
  - `test_stage_error_display_timeout`
  - `test_resolve_error_implements_send_sync`
  - `test_source_error_implements_send_sync`
  - `test_stage_error_implements_send_sync`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add error types`

### Step 3: ConditionExpr and RetryPolicy types

- [ ] Add `ConditionExpr` and `RetryPolicy` to `lib.rs` (from section 6)
- [ ] Write tests:
  - `test_condition_expr_new_and_as_str`
  - `test_condition_expr_display`
  - `test_condition_expr_serde_roundtrip`
  - `test_retry_policy_default_values`
  - `test_retry_policy_from_spec`
  - `test_retry_policy_serde_roundtrip`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add ConditionExpr and RetryPolicy types`

### Step 4: PipelineItem, SourceItem, ExtractedDocument

- [ ] Create `src/traits.rs` with `SourceItem`, `ExtractedDocument`, `PipelineItem` structs (from section 6). Include serde derives and `serde_bytes` annotation on content fields.
- [ ] Add `pub mod traits;` and re-exports to `lib.rs`
- [ ] Write tests:
  - `test_source_item_serde_roundtrip`
  - `test_extracted_document_serde_roundtrip`
  - `test_pipeline_item_serde_roundtrip`
  - `test_pipeline_item_arc_content_clone_is_shallow` — clone a `PipelineItem`, verify `Arc::ptr_eq` on content
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add PipelineItem, SourceItem, ExtractedDocument`

### Step 5: Traits (SourceAdapter, Stage) and StageContext

- [ ] Add `SourceAdapter` trait, `Stage` trait, and `StageContext` struct to `src/traits.rs` (from section 6)
- [ ] Update re-exports in `lib.rs`
- [ ] Write tests:
  - `test_source_adapter_is_object_safe` — create a mock struct implementing `SourceAdapter`, store as `Arc<dyn SourceAdapter>`
  - `test_stage_is_object_safe` — create a mock struct implementing `Stage`, store as `Arc<dyn Stage>`
  - `test_stage_context_is_clone` — verify `StageContext` implements `Clone`
  - `test_stage_context_is_immutable` — verify `StageContext` has no `&mut self` methods (compile-time, structural test)
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add SourceAdapter and Stage traits`

### Step 6: Resource graph

- [ ] Create `src/resource_graph.rs` with `ResourceGraph` struct (from section 6)
- [ ] Add `pub mod resource_graph;` to `lib.rs` (note: the struct itself is `pub(crate)`)
- [ ] Write tests:
  - `test_resource_graph_build_empty_stages`
  - `test_resource_graph_build_single_stage`
  - `test_resource_graph_build_duplicate_creator_fails`
  - `test_resource_graph_build_tracks_readers`
  - `test_resource_graph_build_tracks_writers`
  - `test_resource_graph_validate_no_missing_inputs_passes`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add ResourceGraph`

### Step 7: Schedule computation

- [ ] Create `src/schedule.rs` with `compute_schedule()` function (from section 6)
- [ ] Add `pub mod schedule;` to `lib.rs`
- [ ] Write tests:
  - `test_schedule_empty_pipeline_returns_empty`
  - `test_schedule_single_stage_one_batch`
  - `test_schedule_independent_stages_same_batch`
  - `test_schedule_dependent_stages_sequential_batches`
  - `test_schedule_five_stage_example_three_batches` (the design doc example — see section 8)
  - `test_schedule_cycle_detection_returns_error`
  - `test_schedule_diamond_dependency`
  - `test_schedule_deterministic_ordering`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add schedule computation`

### Step 8: PipelineTopology and ResolvedStage structs

- [ ] Add `PipelineTopology` and `ResolvedStage` structs to `lib.rs` (from section 6)
- [ ] Write tests:
  - `test_pipeline_topology_has_expected_fields` — structural test, construct with mock data
  - `test_resolved_stage_has_expected_fields` — structural test
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add PipelineTopology and ResolvedStage`

### Step 9: Resolve stub

- [ ] Create `src/resolve.rs` with stub `resolve()` function (from section 6)
- [ ] Add `pub mod resolve;` to `lib.rs`
- [ ] Write test:
  - `test_resolve_stub_is_todo` — call `resolve()` in a tokio test, expect panic with "milestone 2.3" in the message (use `#[should_panic]`)
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-topo): add resolve stub`

### Step 10: Property tests and final polish

- [ ] Add proptest for `RetryPolicy` — random values survive serde roundtrip
- [ ] Add proptest for `ConditionExpr` — arbitrary non-empty strings survive roundtrip
- [ ] Add integration test: build `ResourceGraph` from full 5-stage example, compute schedule, verify batches match expected output
- [ ] Run `make test`, `make lint`, `make format`
- [ ] Verify all public items have doc comments
- [ ] Commit: `feat(ecl-pipeline-topo): add property tests and polish`

## 8. Test Fixtures

### Stage Specs for the 5-Stage Example

Use this `BTreeMap<String, StageSpec>` to test resource graph and schedule
computation. This mirrors the design doc's example pipeline.

```rust
use std::collections::BTreeMap;
use ecl_pipeline_spec::stage::{StageSpec, ResourceSpec};

fn five_stage_specs() -> BTreeMap<String, StageSpec> {
    let mut stages = BTreeMap::new();

    stages.insert("fetch-gdrive".to_string(), StageSpec {
        adapter: "extract".to_string(),
        source: Some("engineering-drive".to_string()),
        resources: ResourceSpec {
            reads: vec!["gdrive-api".to_string()],
            creates: vec!["raw-gdrive-docs".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: Some(300),
        skip_on_error: false,
        condition: None,
    });

    stages.insert("fetch-slack".to_string(), StageSpec {
        adapter: "extract".to_string(),
        source: Some("team-slack".to_string()),
        resources: ResourceSpec {
            reads: vec!["slack-api".to_string()],
            creates: vec!["raw-slack-messages".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("normalize-gdrive".to_string(), StageSpec {
        adapter: "normalize".to_string(),
        source: Some("engineering-drive".to_string()),
        resources: ResourceSpec {
            reads: vec!["raw-gdrive-docs".to_string()],
            creates: vec!["normalized-docs".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("normalize-slack".to_string(), StageSpec {
        adapter: "slack-normalize".to_string(),
        source: Some("team-slack".to_string()),
        resources: ResourceSpec {
            reads: vec!["raw-slack-messages".to_string()],
            creates: vec!["normalized-messages".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("emit".to_string(), StageSpec {
        adapter: "emit".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec!["normalized-docs".to_string(), "normalized-messages".to_string()],
            creates: vec![],
            writes: vec![],
        },
        params: serde_json::json!({ "subdir": "normalized" }),
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages
}
```

### Expected Schedule for 5-Stage Example

```
Batch 0: [fetch-gdrive, fetch-slack]           # independent sources (both read external APIs)
Batch 1: [normalize-gdrive, normalize-slack]    # each reads its own source's output
Batch 2: [emit]                                 # reads both normalized outputs
```

In code:

```rust
let expected = vec![
    vec![StageId::new("fetch-gdrive"), StageId::new("fetch-slack")],
    vec![StageId::new("normalize-gdrive"), StageId::new("normalize-slack")],
    vec![StageId::new("emit")],
];
```

### Cycle Detection Test Data

Two stages that create what the other reads:

```rust
fn cyclic_stage_specs() -> BTreeMap<String, StageSpec> {
    let mut stages = BTreeMap::new();

    stages.insert("stage-a".to_string(), StageSpec {
        adapter: "test".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec!["resource-b".to_string()],
            creates: vec!["resource-a".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("stage-b".to_string(), StageSpec {
        adapter: "test".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec!["resource-a".to_string()],
            creates: vec!["resource-b".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages
}
```

### Diamond Dependency Test Data

```
    A
   / \
  B   C
   \ /
    D
```

Stages A creates resource-x and resource-y. B reads resource-x, creates
resource-bx. C reads resource-y, creates resource-cy. D reads resource-bx
and resource-cy.

```rust
fn diamond_stage_specs() -> BTreeMap<String, StageSpec> {
    let mut stages = BTreeMap::new();

    stages.insert("a".to_string(), StageSpec {
        adapter: "test".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec![],
            creates: vec!["resource-x".to_string(), "resource-y".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("b".to_string(), StageSpec {
        adapter: "test".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec!["resource-x".to_string()],
            creates: vec!["resource-bx".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("c".to_string(), StageSpec {
        adapter: "test".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec!["resource-y".to_string()],
            creates: vec!["resource-cy".to_string()],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages.insert("d".to_string(), StageSpec {
        adapter: "test".to_string(),
        source: None,
        resources: ResourceSpec {
            reads: vec!["resource-bx".to_string(), "resource-cy".to_string()],
            creates: vec![],
            writes: vec![],
        },
        params: serde_json::Value::Null,
        retry: None,
        timeout_secs: None,
        skip_on_error: false,
        condition: None,
    });

    stages
}
```

Expected schedule: `[[a], [b, c], [d]]`

### Mock SourceAdapter and Stage (for object-safety tests)

```rust
use std::sync::Arc;
use async_trait::async_trait;

#[derive(Debug)]
struct MockSourceAdapter;

#[async_trait]
impl SourceAdapter for MockSourceAdapter {
    fn source_kind(&self) -> &str { "mock" }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        Ok(vec![])
    }

    async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        Err(SourceError::NotFound {
            source: "mock".to_string(),
            item_id: "none".to_string(),
        })
    }
}

#[derive(Debug)]
struct MockStage;

#[async_trait]
impl Stage for MockStage {
    fn name(&self) -> &str { "mock" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Ok(vec![item])
    }
}
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_resolve_error_display_duplicate_creator` | `error` | `ResolveError::DuplicateCreator` Display output includes resource and stages |
| `test_resolve_error_display_cycle_detected` | `error` | `ResolveError::CycleDetected` Display output includes stage list |
| `test_source_error_display_auth_error` | `error` | `SourceError::AuthError` Display output includes source and message |
| `test_stage_error_display_timeout` | `error` | `StageError::Timeout` Display output includes stage, item_id, timeout |
| `test_resolve_error_implements_send_sync` | `error` | `ResolveError: Send + Sync` (required for async) |
| `test_source_error_implements_send_sync` | `error` | `SourceError: Send + Sync` (required for async) |
| `test_stage_error_implements_send_sync` | `error` | `StageError: Send + Sync` (required for async) |
| `test_condition_expr_new_and_as_str` | `lib` | `ConditionExpr::new("x > 1").as_str() == "x > 1"` |
| `test_condition_expr_display` | `lib` | `format!("{}", expr) == "x > 1"` |
| `test_condition_expr_serde_roundtrip` | `lib` | Serialize to JSON and back, assert equal |
| `test_retry_policy_default_values` | `lib` | Default has max_attempts=3, initial_backoff=1s, multiplier=2.0, max_backoff=30s |
| `test_retry_policy_from_spec` | `lib` | Converts `RetrySpec` milliseconds to `Duration` correctly |
| `test_retry_policy_serde_roundtrip` | `lib` | Serialize to JSON and back, assert equal |
| `test_source_item_serde_roundtrip` | `traits` | `SourceItem` survives JSON roundtrip |
| `test_extracted_document_serde_roundtrip` | `traits` | `ExtractedDocument` with binary content survives JSON roundtrip |
| `test_pipeline_item_serde_roundtrip` | `traits` | `PipelineItem` with `Arc<[u8]>` content survives JSON roundtrip |
| `test_pipeline_item_arc_content_clone_is_shallow` | `traits` | After clone, `Arc::ptr_eq` on content is true (zero-copy) |
| `test_source_adapter_is_object_safe` | `traits` | `Arc<dyn SourceAdapter>` compiles and can be stored |
| `test_stage_is_object_safe` | `traits` | `Arc<dyn Stage>` compiles and can be stored |
| `test_stage_context_is_clone` | `traits` | `StageContext` implements `Clone` |
| `test_resource_graph_build_empty_stages` | `resource_graph` | Empty input produces empty graph |
| `test_resource_graph_build_single_stage` | `resource_graph` | Single stage with creates/reads populates correctly |
| `test_resource_graph_build_duplicate_creator_fails` | `resource_graph` | Two stages creating the same resource returns `DuplicateCreator` |
| `test_resource_graph_build_tracks_readers` | `resource_graph` | Multiple readers of a resource are all tracked |
| `test_resource_graph_build_tracks_writers` | `resource_graph` | Writer tracking works correctly |
| `test_resource_graph_validate_no_missing_inputs_passes` | `resource_graph` | Validation passes for valid graph (including external resources) |
| `test_schedule_empty_pipeline_returns_empty` | `schedule` | Empty stages produce empty schedule |
| `test_schedule_single_stage_one_batch` | `schedule` | Single stage results in `[[stage]]` |
| `test_schedule_independent_stages_same_batch` | `schedule` | Stages with no shared resources are in the same batch |
| `test_schedule_dependent_stages_sequential_batches` | `schedule` | A reads what B creates: B is in an earlier batch |
| `test_schedule_five_stage_example_three_batches` | `schedule` | The design doc 5-stage example produces exactly 3 batches as specified |
| `test_schedule_cycle_detection_returns_error` | `schedule` | Cyclic dependencies return `CycleDetected` |
| `test_schedule_diamond_dependency` | `schedule` | Diamond pattern produces `[[a], [b, c], [d]]` |
| `test_schedule_deterministic_ordering` | `schedule` | Same input always produces same output (run 10 times) |
| `test_pipeline_topology_has_expected_fields` | `lib` | `PipelineTopology` can be constructed with mock data |
| `test_resolved_stage_has_expected_fields` | `lib` | `ResolvedStage` can be constructed with all fields |
| `test_resolve_stub_is_todo` | `resolve` | `resolve()` panics with "milestone 2.3" message |
| `test_retry_policy_proptest_roundtrip` | `lib` | Random `RetryPolicy` values survive serde roundtrip (proptest) |
| `test_condition_expr_proptest_roundtrip` | `lib` | Random non-empty strings survive `ConditionExpr` roundtrip (proptest) |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline-topo` passes
- [ ] `cargo test -p ecl-pipeline-topo` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] `SourceAdapter` and `Stage` traits are object-safe (`Arc<dyn SourceAdapter>` and `Arc<dyn Stage>` compile)
- [ ] `PipelineItem.content` uses `Arc<[u8]>` with `serde_bytes`
- [ ] `StageContext` has no `&mut self` methods (immutable by design)
- [ ] `ResourceGraph` is `pub(crate)`, not public
- [ ] Schedule computation produces correct 3-batch result for the 5-stage example
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement full topology resolution (that is milestone 2.3 — `resolve.rs` is a stub)
- Do NOT implement the pipeline runner or execution logic (that is milestone 2.2)
- Do NOT implement concrete source adapters or stages (that is milestone 3.1+)
- Do NOT implement the `StateStore` trait implementations (that is milestone 1.2)
- Do NOT implement condition expression evaluation (deferred — `ConditionExpr` is just a newtype wrapper)
- Do NOT add CLI commands (that is milestone 5.1)
- Do NOT add any crates beyond `ecl-pipeline-topo`
- Do NOT make `ResourceGraph` public — it is an internal implementation detail
