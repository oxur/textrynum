# Milestone 1.2: State Layer (`ecl-pipeline-state`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline-state):`

## 1. Goal

Create the `ecl-pipeline-state` crate containing all pipeline execution state
types, ID newtypes, checkpoint logic, and the `StateStore` trait. When done:

- All state types compile, derive `Debug + Clone + Serialize + Deserialize`
- `StageId`, `RunId`, and `Blake3Hash` newtypes are defined with `new()`, `as_str()`, and `Display`
- `Checkpoint` round-trips through JSON without data loss
- `Checkpoint::prepare_for_resume()` resets Processing items to Pending and Running stages to Pending
- `Checkpoint::config_drifted()` compares `Blake3Hash` values
- `PipelineState::update_stats()` recomputes `PipelineStats` from source/item data
- `StateStore` async trait is defined and object-safe
- `InMemoryStateStore` passes all trait tests

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
- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, StateError>;`
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
3. `assets/ai/ai-rust/guides/05-type-design.md`
4. `assets/ai/ai-rust/guides/06-traits.md`
5. `assets/ai/ai-rust/guides/07-concurrency-async.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:
- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments

## 3. Prior Art / Dependencies

This crate depends on `ecl-pipeline-spec` (milestone 1.1). The following
public types are imported from that crate:

```rust
// From ecl-pipeline-spec — the TOML-driven configuration layer

/// The root configuration, deserialized from TOML.
pub struct PipelineSpec {
    pub name: String,
    pub version: u32,
    pub output_dir: PathBuf,
    pub sources: BTreeMap<String, SourceSpec>,
    pub stages: BTreeMap<String, StageSpec>,
    pub defaults: DefaultsSpec,
}

/// A source specification (internally tagged enum: google_drive, slack, filesystem).
pub enum SourceSpec { GoogleDrive(..), Slack(..), Filesystem(..) }

/// A stage specification with adapter, resources, retry, etc.
pub struct StageSpec {
    pub adapter: String,
    pub source: Option<String>,
    pub resources: ResourceSpec,
    pub params: serde_json::Value,
    pub retry: Option<RetrySpec>,
    pub timeout_secs: Option<u64>,
    pub skip_on_error: bool,
    pub condition: Option<String>,
}

/// Retry policy configuration.
pub struct RetrySpec {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub max_backoff_ms: u64,
}

/// Global defaults (concurrency, retry, checkpoint strategy).
pub struct DefaultsSpec {
    pub concurrency: usize,
    pub retry: RetrySpec,
    pub checkpoint: CheckpointStrategy,
}

/// Resource access declarations (reads, creates, writes).
pub struct ResourceSpec {
    pub reads: Vec<String>,
    pub creates: Vec<String>,
    pub writes: Vec<String>,
}

/// When to write checkpoints.
pub enum CheckpointStrategy { Batch, Items { count: usize }, Seconds { duration: u64 } }

/// How to resolve credentials for a source.
pub enum CredentialRef { File { path: PathBuf }, EnvVar { env: String }, ApplicationDefault }

/// Google Drive source config.
pub struct GoogleDriveSourceSpec { .. }

/// Slack workspace source config.
pub struct SlackSourceSpec { .. }

/// Local filesystem source config.
pub struct FilesystemSourceSpec { .. }

/// Filter rule (pattern + include/exclude action).
pub struct FilterRule { pub pattern: String, pub action: FilterAction }

/// Filter action.
pub enum FilterAction { Include, Exclude }

/// File type filter.
pub struct FileTypeFilter { pub extension: Option<String>, pub mime: Option<String> }
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add workspace deps `serde_bytes`, `blake3`, `async-trait`; add `crates/ecl-pipeline-state` to members |
| `crates/ecl-pipeline-state/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-pipeline-state/src/lib.rs` | Create | Module declarations, re-exports, `PipelineState` struct and `update_stats()` |
| `crates/ecl-pipeline-state/src/types.rs` | Create | `SourceState`, `ItemState`, `ItemStatus`, `CompletedStageRecord`, `ItemProvenance`, `StageState`, `StageStatus`, `PipelineStats`, `PipelineStatus` |
| `crates/ecl-pipeline-state/src/ids.rs` | Create | `RunId`, `StageId`, `Blake3Hash` newtypes |
| `crates/ecl-pipeline-state/src/checkpoint.rs` | Create | `Checkpoint` struct, `prepare_for_resume()`, `config_drifted()` |
| `crates/ecl-pipeline-state/src/store.rs` | Create | `StateStore` async trait |
| `crates/ecl-pipeline-state/src/memory_store.rs` | Create | `InMemoryStateStore` implementation |
| `crates/ecl-pipeline-state/src/error.rs` | Create | `StateError` enum |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml` (if not already present):

```toml
serde_bytes = "0.11"
```

**Note:** `blake3`, `async-trait`, `chrono`, `serde`, `serde_json`, `thiserror`,
`tokio`, `proptest`, and `tempfile` are already in `[workspace.dependencies]`.

Add to `[workspace] members`:

```toml
"crates/ecl-pipeline-state",
```

### Crate Cargo.toml

```toml
[package]
name = "ecl-pipeline-state"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Pipeline execution state types and checkpoint logic for ECL pipeline runner"

[dependencies]
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
serde = { workspace = true }
serde_json = { workspace = true }
serde_bytes = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
blake3 = { workspace = true }
tokio = { workspace = true }

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
the authoritative design document (design doc 0017, sections 2 and 3).

### `src/ids.rs`

```rust
//! Newtype identifiers for pipeline state.
//!
//! All IDs are name-based strings (not auto-incremented integers) for
//! human readability and checkpoint stability across runs.

use serde::{Deserialize, Serialize};

/// Deterministic, name-based stage identifier.
/// NOT an auto-incremented integer (dagrs anti-pattern).
/// Stable across runs for checkpoint compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StageId(String);

impl StageId {
    /// Create a new stage ID from a name.
    pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }

    /// Get the stage ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for StageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Blake3 content hash, stored as hex string for JSON readability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);

impl Blake3Hash {
    /// Create a new Blake3Hash from a hex string.
    pub fn new(hex: impl Into<String>) -> Self { Self(hex.into()) }

    /// Get the hash as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }

    /// Returns true if the hash is empty (not yet computed).
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}

impl std::fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Unique identifier for a pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(String);

impl RunId {
    /// Create a new run ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Get the run ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```

### `src/types.rs`

```rust
//! Pipeline execution state types.
//!
//! These types represent the mutable state that changes during pipeline
//! execution. They are the "what has happened so far" layer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ids::{Blake3Hash, StageId};

/// Overall pipeline execution status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    /// Pipeline has not started executing.
    Pending,
    /// Pipeline is currently running.
    Running {
        /// The stage currently being executed.
        current_stage: String,
    },
    /// Pipeline completed successfully.
    Completed {
        /// When the pipeline finished.
        finished_at: DateTime<Utc>,
    },
    /// Pipeline failed with an error.
    Failed {
        /// Error description.
        error: String,
        /// When the failure occurred.
        failed_at: DateTime<Utc>,
    },
    /// Pipeline was interrupted and can be resumed.
    Interrupted {
        /// When the interruption occurred.
        interrupted_at: DateTime<Utc>,
    },
}

/// Per-source state: what items were discovered, accepted, and processed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceState {
    /// How many items were discovered by enumeration.
    pub items_discovered: usize,

    /// How many items passed filters.
    pub items_accepted: usize,

    /// How many items were skipped due to unchanged content hash.
    pub items_skipped_unchanged: usize,

    /// Per-item state for items that entered the pipeline.
    /// Key: source-specific item ID.
    pub items: BTreeMap<String, ItemState>,
}

/// The state of a single item flowing through the pipeline.
/// This is the atomic unit of work and the atomic unit of checkpointing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState {
    /// Human-readable identifier (filename, message preview, etc.).
    pub display_name: String,

    /// Source-specific unique identifier.
    pub source_id: String,

    /// Which source this item came from (key into PipelineState.sources).
    pub source_name: String,

    /// Blake3 hash of the source content, for incrementality.
    pub content_hash: Blake3Hash,

    /// Current processing status.
    pub status: ItemStatus,

    /// Which stages have been completed for this item, with timing.
    pub completed_stages: Vec<CompletedStageRecord>,

    /// Provenance: where did this item come from and when?
    pub provenance: ItemProvenance,
}

/// Processing status of a single pipeline item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemStatus {
    /// Discovered but not yet processed.
    Pending,
    /// Currently being processed by a stage.
    Processing {
        /// The stage currently processing this item.
        stage: String,
    },
    /// All applicable stages completed successfully.
    Completed,
    /// Failed at a stage; may be retryable.
    Failed {
        /// The stage where failure occurred.
        stage: String,
        /// Error description.
        error: String,
        /// Number of attempts made.
        attempts: u32,
    },
    /// Skipped due to skip_on_error or conditional stage.
    Skipped {
        /// The stage that was skipped.
        stage: String,
        /// Reason for skipping.
        reason: String,
    },
    /// Skipped due to unchanged content hash (incrementality).
    Unchanged,
}

/// Record of a completed stage for an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStageRecord {
    /// Which stage completed.
    pub stage: StageId,
    /// When the stage completed for this item.
    pub completed_at: DateTime<Utc>,
    /// How long the stage took for this item, in milliseconds.
    pub duration_ms: u64,
}

/// Provenance information for a pipeline item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProvenance {
    /// Source service type (e.g., "google_drive", "slack").
    pub source_kind: String,

    /// Source-specific metadata.
    /// For Drive: { "file_id": "...", "path": "/Engineering/doc.docx", "owner": "..." }
    /// For Slack: { "channel": "...", "thread_ts": "...", "author": "..." }
    pub metadata: BTreeMap<String, serde_json::Value>,

    /// When the source item was last modified (per the source API).
    pub source_modified: Option<DateTime<Utc>>,

    /// When we extracted it.
    pub extracted_at: DateTime<Utc>,
}

/// Per-stage aggregate state (derived from item states, cached).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    /// Current status of the stage.
    pub status: StageStatus,
    /// Number of items processed by this stage.
    pub items_processed: usize,
    /// Number of items that failed in this stage.
    pub items_failed: usize,
    /// Number of items skipped in this stage.
    pub items_skipped: usize,
    /// When the stage started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the stage finished executing.
    pub completed_at: Option<DateTime<Utc>>,
}

/// Execution status of a pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    /// Stage has not started.
    Pending,
    /// Stage is currently executing.
    Running,
    /// Stage completed successfully.
    Completed,
    /// Stage was skipped.
    Skipped {
        /// Reason the stage was skipped.
        reason: String,
    },
    /// Stage failed.
    Failed {
        /// Error description.
        error: String,
    },
}

/// Summary statistics for the pipeline run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    /// Total items discovered across all sources.
    pub total_items_discovered: usize,
    /// Total items that completed all stages.
    pub total_items_processed: usize,
    /// Total items skipped due to unchanged content hash.
    pub total_items_skipped_unchanged: usize,
    /// Total items that failed processing.
    pub total_items_failed: usize,
}
```

### `src/lib.rs`

```rust
//! Pipeline execution state types for the ECL pipeline runner.
//!
//! This crate defines the mutable state layer: everything that changes
//! during pipeline execution. All types derive `Serialize + Deserialize`
//! for checkpointing. The `StateStore` trait provides pluggable persistence.

pub mod checkpoint;
pub mod error;
pub mod ids;
pub mod memory_store;
pub mod store;
pub mod types;

pub use checkpoint::Checkpoint;
pub use error::{Result, StateError};
pub use ids::{Blake3Hash, RunId, StageId};
pub use memory_store::InMemoryStateStore;
pub use store::StateStore;
pub use types::{
    CompletedStageRecord, ItemProvenance, ItemState, ItemStatus, PipelineStats, PipelineStatus,
    SourceState, StageState, StageStatus,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Complete pipeline execution state.
/// Serialize this at any point for a perfect checkpoint.
/// Hand it to Claude for full system understanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Unique identifier for this run.
    pub run_id: RunId,

    /// Pipeline identity (from spec).
    pub pipeline_name: String,

    /// When this run started.
    pub started_at: DateTime<Utc>,

    /// When the last checkpoint was written.
    pub last_checkpoint: DateTime<Utc>,

    /// Overall pipeline status.
    pub status: PipelineStatus,

    /// Which batch is currently executing (index into schedule).
    pub current_batch: usize,

    /// Per-source extraction state (items, discovery counts, etc.).
    pub sources: BTreeMap<String, SourceState>,

    /// Per-stage execution state (timing, counts, errors).
    pub stages: BTreeMap<StageId, StageState>,

    /// Summary statistics (derived, but cached for observability).
    pub stats: PipelineStats,
}

impl PipelineState {
    /// Recompute `PipelineStats` from source/item data.
    ///
    /// Call this after modifying item states to keep stats consistent.
    /// Stats are derived from source-level counters and individual item
    /// statuses.
    pub fn update_stats(&mut self) {
        let mut total_discovered = 0usize;
        let mut total_processed = 0usize;
        let mut total_skipped_unchanged = 0usize;
        let mut total_failed = 0usize;

        for source_state in self.sources.values() {
            total_discovered += source_state.items_discovered;
            total_skipped_unchanged += source_state.items_skipped_unchanged;

            for item in source_state.items.values() {
                match &item.status {
                    ItemStatus::Completed => total_processed += 1,
                    ItemStatus::Failed { .. } => total_failed += 1,
                    ItemStatus::Unchanged => total_skipped_unchanged += 1,
                    _ => {}
                }
            }
        }

        self.stats = PipelineStats {
            total_items_discovered: total_discovered,
            total_items_processed: total_processed,
            total_items_skipped_unchanged: total_skipped_unchanged,
            total_items_failed: total_failed,
        };
    }
}
```

### `src/checkpoint.rs`

```rust
//! Checkpoint: self-contained recovery artifact.
//!
//! Bundles all three layers (spec, topology schedule, state) so resume
//! needs nothing external.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{Blake3Hash, StageId};
use crate::types::{ItemStatus, StageStatus};
use crate::PipelineState;
use crate::PipelineStatus;
use ecl_pipeline_spec::PipelineSpec;

/// The checkpoint: a complete, self-contained recovery artifact.
/// Bundles all three layers so resume needs nothing external.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint format version for forward compatibility.
    pub version: u32,

    /// Monotonically increasing sequence number within a run.
    pub sequence: u64,

    /// When this checkpoint was created.
    pub created_at: DateTime<Utc>,

    /// The full specification (immutable, embedded for self-containedness).
    pub spec: PipelineSpec,

    /// The computed schedule (immutable, embedded).
    pub schedule: Vec<Vec<StageId>>,

    /// The spec hash at the time this run began.
    pub spec_hash: Blake3Hash,

    /// The mutable execution state.
    pub state: PipelineState,
}

impl Checkpoint {
    /// Prepare a checkpoint for resume after a crash.
    /// Resets any items stuck in Processing back to Pending.
    /// Resets any stages stuck in Running back to Pending.
    /// Sets pipeline status to Interrupted.
    pub fn prepare_for_resume(&mut self) {
        // Reset pipeline status.
        self.state.status = PipelineStatus::Interrupted {
            interrupted_at: self.created_at,
        };

        // Reset any items stuck mid-processing.
        for source_state in self.state.sources.values_mut() {
            for item_state in source_state.items.values_mut() {
                if matches!(item_state.status, ItemStatus::Processing { .. }) {
                    item_state.status = ItemStatus::Pending;
                }
            }
        }

        // Reset any stages stuck in Running.
        for stage_state in self.state.stages.values_mut() {
            if matches!(stage_state.status, StageStatus::Running) {
                stage_state.status = StageStatus::Pending;
            }
        }
    }

    /// Check whether the current TOML config has drifted from the
    /// checkpoint's embedded spec.
    pub fn config_drifted(&self, current_spec_hash: &Blake3Hash) -> bool {
        self.spec_hash != *current_spec_hash
    }
}
```

### `src/error.rs`

```rust
//! Error types for the state layer.

use thiserror::Error;

/// Errors that can occur in the state layer.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    /// Serialization or deserialization failed.
    #[error("serialization error: {message}")]
    SerializationError {
        /// The serialization error message.
        message: String,
    },

    /// State store I/O failed.
    #[error("store I/O error: {message}")]
    StoreError {
        /// The I/O error message.
        message: String,
    },

    /// Checkpoint version is unsupported.
    #[error("unsupported checkpoint version: {version}")]
    UnsupportedVersion {
        /// The unsupported version number.
        version: u32,
    },

    /// Config drift detected on resume.
    #[error("config drift detected: spec hash changed from {expected} to {actual}")]
    ConfigDrift {
        /// The expected spec hash (from checkpoint).
        expected: String,
        /// The actual spec hash (from current TOML).
        actual: String,
    },

    /// Checkpoint not found.
    #[error("no checkpoint found")]
    NotFound,
}

/// Result type for state operations.
pub type Result<T> = std::result::Result<T, StateError>;
```

### `src/store.rs`

```rust
//! StateStore trait: pluggable persistence for pipeline state.
//!
//! Trait-based so the persistence backend can be swapped without
//! changing the runner. Production: redb (milestone 2.1). Testing:
//! InMemoryStateStore (this crate).

use async_trait::async_trait;
use std::collections::BTreeMap;

use crate::checkpoint::Checkpoint;
use crate::error::StateError;
use crate::ids::{Blake3Hash, RunId};

/// Persistent state storage for pipeline checkpoints and content hashes.
///
/// Implementations must be crash-safe: either the full checkpoint
/// is persisted or none of it is. redb provides this via ACID
/// transactions; filesystem impls must use write-then-rename.
///
/// Uses `async_trait` for object safety (`dyn StateStore`).
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save a checkpoint (atomic write).
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> std::result::Result<(), StateError>;

    /// Load the most recent checkpoint, if one exists.
    async fn load_checkpoint(&self) -> std::result::Result<Option<Checkpoint>, StateError>;

    /// Load content hashes from the most recent *completed* run.
    /// Used for cross-run incrementality.
    async fn load_previous_hashes(
        &self,
    ) -> std::result::Result<BTreeMap<String, Blake3Hash>, StateError>;

    /// Save content hashes at the end of a successful run.
    async fn save_completed_hashes(
        &self,
        run_id: &RunId,
        hashes: &BTreeMap<String, Blake3Hash>,
    ) -> std::result::Result<(), StateError>;
}
```

### `src/memory_store.rs`

```rust
//! In-memory StateStore implementation for testing.

use async_trait::async_trait;
use std::collections::BTreeMap;
use tokio::sync::RwLock;

use crate::checkpoint::Checkpoint;
use crate::error::StateError;
use crate::ids::{Blake3Hash, RunId};
use crate::store::StateStore;

/// In-memory state store for unit and integration testing.
///
/// Stores checkpoints and hashes in memory behind a `RwLock`.
/// Not suitable for production use — no crash durability.
#[derive(Debug, Default)]
pub struct InMemoryStateStore {
    /// The most recent checkpoint.
    checkpoint: RwLock<Option<Checkpoint>>,
    /// Content hashes from the most recent completed run.
    hashes: RwLock<BTreeMap<String, Blake3Hash>>,
}

impl InMemoryStateStore {
    /// Create a new empty in-memory state store.
    pub fn new() -> Self {
        Self {
            checkpoint: RwLock::new(None),
            hashes: RwLock::new(BTreeMap::new()),
        }
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> std::result::Result<(), StateError> {
        let mut guard = self.checkpoint.write().await;
        *guard = Some(checkpoint.clone());
        Ok(())
    }

    async fn load_checkpoint(&self) -> std::result::Result<Option<Checkpoint>, StateError> {
        let guard = self.checkpoint.read().await;
        Ok(guard.clone())
    }

    async fn load_previous_hashes(
        &self,
    ) -> std::result::Result<BTreeMap<String, Blake3Hash>, StateError> {
        let guard = self.hashes.read().await;
        Ok(guard.clone())
    }

    async fn save_completed_hashes(
        &self,
        _run_id: &RunId,
        hashes: &BTreeMap<String, Blake3Hash>,
    ) -> std::result::Result<(), StateError> {
        let mut guard = self.hashes.write().await;
        *guard = hashes.clone();
        Ok(())
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold crate and verify compilation

- [ ] Create `crates/ecl-pipeline-state/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create minimal `src/lib.rs` with just `//! Pipeline state crate.`
- [ ] Add `serde_bytes = "0.11"` to root `Cargo.toml` `[workspace.dependencies]` (if not already present)
- [ ] Add `"crates/ecl-pipeline-state"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-pipeline-state` — must pass
- [ ] Commit: `feat(ecl-pipeline-state): scaffold crate`

### Step 2: Error types

- [ ] Create `src/error.rs` with `StateError` enum (from section 6)
- [ ] Add `pub mod error;` and `pub use error::{Result, StateError};` to `lib.rs`
- [ ] Write tests:
  - `test_error_display_serialization_error` — verify Display output
  - `test_error_display_store_error` — verify Display output
  - `test_error_display_config_drift` — includes both hash values
  - `test_error_implements_send_sync` — `StateError: Send + Sync`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add StateError types`

### Step 3: ID newtypes (StageId, RunId, Blake3Hash)

- [ ] Create `src/ids.rs` with `StageId`, `RunId`, `Blake3Hash` (from section 6)
- [ ] Add module declaration and re-exports to `lib.rs`
- [ ] Write tests:
  - `test_stage_id_new_and_as_str` — create and read back
  - `test_stage_id_display` — Display trait output matches inner string
  - `test_stage_id_ord` — ordering is alphabetical
  - `test_stage_id_serde_roundtrip` — JSON serialize/deserialize
  - `test_run_id_new_and_as_str` — create and read back
  - `test_run_id_display` — Display trait output
  - `test_run_id_serde_roundtrip` — JSON serialize/deserialize
  - `test_blake3_hash_new_and_as_str` — create and read back
  - `test_blake3_hash_is_empty` — empty string returns true
  - `test_blake3_hash_display` — Display trait output
  - `test_blake3_hash_equality` — PartialEq works correctly
  - `test_blake3_hash_serde_roundtrip` — JSON serialize/deserialize
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add ID newtypes`

### Step 4: State types

- [ ] Create `src/types.rs` with all state types (from section 6)
- [ ] Add module declaration and re-exports to `lib.rs`
- [ ] Write tests:
  - `test_pipeline_status_all_variants_serde` — all 5 PipelineStatus variants roundtrip
  - `test_source_state_default` — Default impl has zero counts and empty items map
  - `test_item_state_serde_roundtrip` — full ItemState with all fields
  - `test_item_status_all_variants_serde` — all 6 ItemStatus variants roundtrip
  - `test_completed_stage_record_serde` — roundtrip
  - `test_item_provenance_serde_roundtrip` — with metadata BTreeMap
  - `test_stage_state_serde_roundtrip` — roundtrip
  - `test_stage_status_all_variants_serde` — all 5 StageStatus variants
  - `test_pipeline_stats_default` — Default impl has all zeros
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add state types`

### Step 5: PipelineState with update_stats()

- [ ] Add `PipelineState` struct and `update_stats()` method to `lib.rs` (from section 6)
- [ ] Write tests:
  - `test_pipeline_state_serde_roundtrip` — full PipelineState with sources and stages
  - `test_pipeline_state_update_stats_empty` — no sources yields all zeros
  - `test_pipeline_state_update_stats_mixed_items` — items in various statuses produce correct counts
  - `test_pipeline_state_update_stats_counts_discovered_from_source` — items_discovered comes from SourceState
  - `test_pipeline_state_update_stats_counts_unchanged_from_both` — unchanged counted from both items_skipped_unchanged and ItemStatus::Unchanged
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add PipelineState with update_stats`

### Step 6: Checkpoint with prepare_for_resume and config_drifted

- [ ] Create `src/checkpoint.rs` with `Checkpoint`, `prepare_for_resume()`, `config_drifted()` (from section 6)
- [ ] Add module declaration and re-export to `lib.rs`
- [ ] Write tests:
  - `test_checkpoint_serde_roundtrip_json` — full Checkpoint serializes to JSON and back without data loss
  - `test_checkpoint_prepare_for_resume_resets_processing_items` — items in Processing become Pending
  - `test_checkpoint_prepare_for_resume_resets_running_stages` — stages in Running become Pending
  - `test_checkpoint_prepare_for_resume_sets_interrupted` — pipeline status becomes Interrupted
  - `test_checkpoint_prepare_for_resume_leaves_completed_items` — items in Completed/Failed/Skipped are not changed
  - `test_checkpoint_config_drifted_same_hash` — returns false for matching hashes
  - `test_checkpoint_config_drifted_different_hash` — returns true for mismatched hashes
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add Checkpoint with prepare_for_resume`

### Step 7: StateStore trait and InMemoryStateStore

- [ ] Create `src/store.rs` with `StateStore` trait (from section 6)
- [ ] Create `src/memory_store.rs` with `InMemoryStateStore` (from section 6)
- [ ] Add module declarations and re-exports to `lib.rs`
- [ ] Write tests (all async with `#[tokio::test]`):
  - `test_memory_store_save_and_load_checkpoint` — save then load returns same checkpoint
  - `test_memory_store_load_checkpoint_empty` — load from fresh store returns None
  - `test_memory_store_save_overwrites_previous` — second save replaces first
  - `test_memory_store_save_and_load_hashes` — save then load returns same hashes
  - `test_memory_store_load_hashes_empty` — load from fresh store returns empty map
  - `test_memory_store_object_safety` — `Box<dyn StateStore>` compiles and works
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add StateStore trait and InMemoryStateStore`

### Step 8: Property tests and final polish

- [ ] Add proptest for `RunId` — random strings survive serde roundtrip
- [ ] Add proptest for `StageId` — random strings survive serde roundtrip
- [ ] Add proptest for `Blake3Hash` — random hex strings survive serde roundtrip
- [ ] Add proptest for `PipelineStats` — random values survive serde roundtrip
- [ ] Run `make test`, `make lint`, `make format`
- [ ] Verify all public items have doc comments
- [ ] Commit: `feat(ecl-pipeline-state): add property tests`

## 8. Test Fixtures

### Minimal PipelineSpec TOML (for constructing test Checkpoints)

Use this TOML to create a `PipelineSpec` for test fixtures:

```toml
name = "test-pipeline"
version = 1
output_dir = "./output/test"

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["raw-docs"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-docs"] }
```

### Example JSON for Checkpoint round-trip test

```json
{
  "version": 1,
  "sequence": 42,
  "created_at": "2026-03-13T10:00:00Z",
  "spec": {
    "name": "test-pipeline",
    "version": 1,
    "output_dir": "./output/test",
    "sources": {
      "local": {
        "kind": "filesystem",
        "root": "/tmp/test-data",
        "filters": [],
        "extensions": []
      }
    },
    "stages": {
      "extract": {
        "adapter": "extract",
        "source": "local",
        "resources": { "reads": [], "creates": ["raw-docs"], "writes": [] },
        "params": null,
        "retry": null,
        "timeout_secs": null,
        "skip_on_error": false,
        "condition": null
      },
      "emit": {
        "adapter": "emit",
        "source": null,
        "resources": { "reads": ["raw-docs"], "creates": [], "writes": [] },
        "params": null,
        "retry": null,
        "timeout_secs": null,
        "skip_on_error": false,
        "condition": null
      }
    },
    "defaults": {
      "concurrency": 4,
      "retry": {
        "max_attempts": 3,
        "initial_backoff_ms": 1000,
        "backoff_multiplier": 2.0,
        "max_backoff_ms": 30000
      },
      "checkpoint": { "every": "Batch" }
    }
  },
  "schedule": [["extract"], ["emit"]],
  "spec_hash": "abc123def456",
  "state": {
    "run_id": "run-001",
    "pipeline_name": "test-pipeline",
    "started_at": "2026-03-13T10:00:00Z",
    "last_checkpoint": "2026-03-13T10:05:00Z",
    "status": { "Running": { "current_stage": "extract" } },
    "current_batch": 0,
    "sources": {
      "local": {
        "items_discovered": 3,
        "items_accepted": 2,
        "items_skipped_unchanged": 1,
        "items": {
          "file1.txt": {
            "display_name": "file1.txt",
            "source_id": "file1.txt",
            "source_name": "local",
            "content_hash": "aabbccdd",
            "status": "Completed",
            "completed_stages": [
              {
                "stage": "extract",
                "completed_at": "2026-03-13T10:02:00Z",
                "duration_ms": 150
              }
            ],
            "provenance": {
              "source_kind": "filesystem",
              "metadata": {},
              "source_modified": null,
              "extracted_at": "2026-03-13T10:01:00Z"
            }
          },
          "file2.txt": {
            "display_name": "file2.txt",
            "source_id": "file2.txt",
            "source_name": "local",
            "content_hash": "eeff0011",
            "status": { "Processing": { "stage": "extract" } },
            "completed_stages": [],
            "provenance": {
              "source_kind": "filesystem",
              "metadata": {},
              "source_modified": null,
              "extracted_at": "2026-03-13T10:03:00Z"
            }
          }
        }
      }
    },
    "stages": {
      "extract": {
        "status": "Running",
        "items_processed": 1,
        "items_failed": 0,
        "items_skipped": 0,
        "started_at": "2026-03-13T10:01:00Z",
        "completed_at": null
      },
      "emit": {
        "status": "Pending",
        "items_processed": 0,
        "items_failed": 0,
        "items_skipped": 0,
        "started_at": null,
        "completed_at": null
      }
    },
    "stats": {
      "total_items_discovered": 3,
      "total_items_processed": 1,
      "total_items_skipped_unchanged": 1,
      "total_items_failed": 0
    }
  }
}
```

**Note:** The JSON above is illustrative. During implementation, build the
test fixture programmatically using the Rust types, then serialize to JSON
and back to verify round-trip fidelity. Do not hard-code JSON strings if the
exact serde representation differs (e.g., enum tagging format). Build the
struct in Rust, serialize, deserialize, and compare.

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_error_display_serialization_error` | `error` | `StateError::SerializationError` Display output |
| `test_error_display_store_error` | `error` | `StateError::StoreError` Display output |
| `test_error_display_config_drift` | `error` | `StateError::ConfigDrift` includes both hashes |
| `test_error_implements_send_sync` | `error` | `StateError: Send + Sync` (required for async) |
| `test_stage_id_new_and_as_str` | `ids` | Create StageId and read back |
| `test_stage_id_display` | `ids` | Display trait output matches inner string |
| `test_stage_id_ord` | `ids` | Ordering is alphabetical |
| `test_stage_id_serde_roundtrip` | `ids` | JSON serialize/deserialize |
| `test_run_id_new_and_as_str` | `ids` | Create RunId and read back |
| `test_run_id_display` | `ids` | Display trait output |
| `test_run_id_serde_roundtrip` | `ids` | JSON serialize/deserialize |
| `test_blake3_hash_new_and_as_str` | `ids` | Create Blake3Hash and read back |
| `test_blake3_hash_is_empty` | `ids` | Empty string returns true, non-empty returns false |
| `test_blake3_hash_display` | `ids` | Display trait output |
| `test_blake3_hash_equality` | `ids` | PartialEq works correctly |
| `test_blake3_hash_serde_roundtrip` | `ids` | JSON serialize/deserialize |
| `test_pipeline_status_all_variants_serde` | `types` | All 5 PipelineStatus variants roundtrip through JSON |
| `test_source_state_default` | `types` | Default impl has zero counts and empty items map |
| `test_item_state_serde_roundtrip` | `types` | Full ItemState with all fields |
| `test_item_status_all_variants_serde` | `types` | All 6 ItemStatus variants roundtrip through JSON |
| `test_completed_stage_record_serde` | `types` | Roundtrip through JSON |
| `test_item_provenance_serde_roundtrip` | `types` | With metadata BTreeMap |
| `test_stage_state_serde_roundtrip` | `types` | Roundtrip through JSON |
| `test_stage_status_all_variants_serde` | `types` | All 5 StageStatus variants roundtrip through JSON |
| `test_pipeline_stats_default` | `types` | Default impl has all zeros |
| `test_pipeline_state_serde_roundtrip` | `lib` | Full PipelineState with sources and stages |
| `test_pipeline_state_update_stats_empty` | `lib` | No sources yields all zeros |
| `test_pipeline_state_update_stats_mixed_items` | `lib` | Items in various statuses produce correct counts |
| `test_pipeline_state_update_stats_counts_discovered_from_source` | `lib` | items_discovered comes from SourceState |
| `test_pipeline_state_update_stats_counts_unchanged_from_both` | `lib` | Unchanged counted from both source and item status |
| `test_checkpoint_serde_roundtrip_json` | `checkpoint` | Full Checkpoint serializes to JSON and back |
| `test_checkpoint_prepare_for_resume_resets_processing_items` | `checkpoint` | Processing items become Pending |
| `test_checkpoint_prepare_for_resume_resets_running_stages` | `checkpoint` | Running stages become Pending |
| `test_checkpoint_prepare_for_resume_sets_interrupted` | `checkpoint` | Pipeline status becomes Interrupted |
| `test_checkpoint_prepare_for_resume_leaves_completed_items` | `checkpoint` | Completed/Failed/Skipped items unchanged |
| `test_checkpoint_config_drifted_same_hash` | `checkpoint` | Returns false for matching hashes |
| `test_checkpoint_config_drifted_different_hash` | `checkpoint` | Returns true for mismatched hashes |
| `test_memory_store_save_and_load_checkpoint` | `memory_store` | Save then load returns same data |
| `test_memory_store_load_checkpoint_empty` | `memory_store` | Fresh store returns None |
| `test_memory_store_save_overwrites_previous` | `memory_store` | Second save replaces first |
| `test_memory_store_save_and_load_hashes` | `memory_store` | Save then load returns same hashes |
| `test_memory_store_load_hashes_empty` | `memory_store` | Fresh store returns empty map |
| `test_memory_store_object_safety` | `memory_store` | `Box<dyn StateStore>` compiles and works |
| `test_run_id_proptest_roundtrip` | `ids` | Random strings survive serde roundtrip (proptest) |
| `test_stage_id_proptest_roundtrip` | `ids` | Random strings survive serde roundtrip (proptest) |
| `test_blake3_hash_proptest_roundtrip` | `ids` | Random hex strings survive serde roundtrip (proptest) |
| `test_pipeline_stats_proptest_roundtrip` | `types` | Random values survive serde roundtrip (proptest) |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline-state` passes
- [ ] `cargo test -p ecl-pipeline-state` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement redb persistence (that is milestone 2.1)
- Do NOT implement execution logic or the pipeline runner (that is milestone 2.2)
- Do NOT implement topology types or resource graph (that is milestone 1.3)
- Do NOT implement CLI commands (that is milestone 5.1)
- Do NOT create adapter or stage implementations (that is milestone 3.1+)
- Do NOT add any crates beyond `ecl-pipeline-state`
- Do NOT move `StageId`, `RunId`, or `Blake3Hash` to another crate — they live here
- Do NOT use `HashMap` anywhere — always `BTreeMap` for deterministic serialization
- Do NOT use `unwrap()` in library code — only in `#[cfg(test)]` blocks
