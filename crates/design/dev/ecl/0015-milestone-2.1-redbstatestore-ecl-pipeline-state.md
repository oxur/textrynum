# Milestone 2.1: RedbStateStore (`ecl-pipeline-state`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline-state):`

## 1. Goal

Add a redb-backed implementation of the `StateStore` trait to the existing
`ecl-pipeline-state` crate. When done:

- `RedbStateStore::open(path)` creates or opens a redb database file
- `save_checkpoint()` atomically writes a serialized `Checkpoint` to redb via ACID transaction
- `load_checkpoint()` retrieves the most recent checkpoint
- `load_previous_hashes()` returns content hashes from the most recent completed run
- `save_completed_hashes()` persists content hashes for cross-run incrementality
- All async trait methods use `tokio::task::spawn_blocking` to avoid blocking the async runtime (redb is sync I/O)
- Crash safety is verified: write data, drop without explicit close, reopen, data persists
- `RedbStateStore` passes all the same trait tests as `InMemoryStateStore`
- The existing `InMemoryStateStore` and all other code in the crate remain unchanged

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
3. `assets/ai/ai-rust/guides/03-error-handling.md`
4. `assets/ai/ai-rust/guides/07-concurrency-async.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:
- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments

## 3. Prior Art / Dependencies

This milestone extends the `ecl-pipeline-state` crate (created in milestone 1.2).
The following public types from that crate are used by `RedbStateStore`. They are
reproduced here exactly so you have zero ambiguity about what you are implementing
against.

### `StateStore` trait (`src/store.rs`)

```rust
//! StateStore trait: pluggable persistence for pipeline state.

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

### `Checkpoint` type (`src/checkpoint.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{Blake3Hash, StageId};
use crate::PipelineState;
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
```

### `StateError` type (`src/error.rs`)

```rust
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

### `Blake3Hash` type (`src/ids.rs`)

```rust
use serde::{Deserialize, Serialize};

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
```

### `RunId` type (`src/ids.rs`)

```rust
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

### `StageId` type (`src/ids.rs`)

```rust
/// Deterministic, name-based stage identifier.
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
```

### `PipelineState` type (`src/lib.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Complete pipeline execution state.
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
    /// Per-source extraction state.
    pub sources: BTreeMap<String, SourceState>,
    /// Per-stage execution state.
    pub stages: BTreeMap<StageId, StageState>,
    /// Summary statistics.
    pub stats: PipelineStats,
}
```

### Existing `lib.rs` module structure

```rust
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
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add `redb = "3"` and `serde_bytes = "0.11"` to `[workspace.dependencies]` (if not already present from milestone 1.1) |
| `crates/ecl-pipeline-state/Cargo.toml` | Modify | Add `redb = { workspace = true }` to `[dependencies]` |
| `crates/ecl-pipeline-state/src/redb_store.rs` | Create | `RedbStateStore` implementation |
| `crates/ecl-pipeline-state/src/lib.rs` | Modify | Add `pub mod redb_store;` and `pub use redb_store::RedbStateStore;` |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml` (if not already present from milestone 1.1):

```toml
redb = "3"
serde_bytes = "0.11"
```

### Crate Cargo.toml Modification

Add `redb` to the existing `[dependencies]` section in `crates/ecl-pipeline-state/Cargo.toml`:

```toml
redb = { workspace = true }
```

The full `[dependencies]` section should look like:

```toml
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
redb = { workspace = true }
```

The `[dev-dependencies]` section already has `tempfile` (from milestone 1.2):

```toml
[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }
```

## 6. Type Definitions and Signatures

### `src/redb_store.rs`

```rust
//! Redb-backed StateStore implementation.
//!
//! Provides crash-safe, ACID-transactional persistence for pipeline
//! checkpoints and content hashes using [redb](https://docs.rs/redb).
//! All redb operations are synchronous I/O; this module wraps them in
//! `tokio::task::spawn_blocking` to avoid blocking the async runtime
//! (per AP-18: blocking I/O in async context).

use async_trait::async_trait;
use redb::{Database, TableDefinition};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::checkpoint::Checkpoint;
use crate::error::StateError;
use crate::ids::{Blake3Hash, RunId};
use crate::store::StateStore;

/// redb table: run_id (str) -> serialized JSON checkpoint (bytes).
const CHECKPOINTS: TableDefinition<&str, &[u8]> = TableDefinition::new("checkpoints");

/// redb table: item_id (str) -> blake3 hex hash (str).
const HASHES: TableDefinition<&str, &str> = TableDefinition::new("hashes");

/// redb table: metadata key (str) -> metadata value (str).
/// Keys used:
/// - "latest_run_id" — the run_id of the most recent checkpoint
/// - "latest_completed_run_id" — the run_id of the most recent completed run
const METADATA: TableDefinition<&str, &str> = TableDefinition::new("metadata");

/// Metadata key for the run ID of the most recent checkpoint.
const KEY_LATEST_RUN_ID: &str = "latest_run_id";

/// Metadata key for the run ID of the most recent completed run.
const KEY_LATEST_COMPLETED_RUN_ID: &str = "latest_completed_run_id";

/// Redb-backed state store providing crash-safe, ACID persistence.
///
/// Uses three tables:
/// - `checkpoints`: maps run_id -> serialized JSON checkpoint
/// - `hashes`: maps item_id -> blake3 hex hash (for the latest completed run)
/// - `metadata`: maps string keys -> string values (for tracking latest run IDs)
///
/// All operations run inside `tokio::task::spawn_blocking` because redb
/// performs synchronous disk I/O.
#[derive(Debug, Clone)]
pub struct RedbStateStore {
    /// The redb database handle, wrapped in Arc for Clone + Send + Sync.
    db: Arc<Database>,
}

impl RedbStateStore {
    /// Open or create a redb database at the given path.
    ///
    /// If the file does not exist, it is created. If it exists, it is
    /// opened and any existing data is preserved.
    ///
    /// # Errors
    ///
    /// Returns `StateError::StoreError` if the database cannot be opened
    /// or created.
    pub fn open(path: impl AsRef<Path>) -> std::result::Result<Self, StateError> {
        let db = Database::create(path.as_ref()).map_err(|e| StateError::StoreError {
            message: format!("failed to open redb database: {e}"),
        })?;
        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait]
impl StateStore for RedbStateStore {
    /// Save a checkpoint atomically.
    ///
    /// Serializes the checkpoint to JSON, then writes it to the
    /// `CHECKPOINTS` table keyed by run_id. Also updates
    /// `METADATA["latest_run_id"]`. All writes happen in a single
    /// redb write transaction (ACID).
    async fn save_checkpoint(
        &self,
        checkpoint: &Checkpoint,
    ) -> std::result::Result<(), StateError> {
        let db = self.db.clone();
        let json_bytes = serde_json::to_vec(checkpoint).map_err(|e| {
            StateError::SerializationError {
                message: format!("failed to serialize checkpoint: {e}"),
            }
        })?;
        let run_id = checkpoint.state.run_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let write_txn = db.begin_write().map_err(|e| StateError::StoreError {
                message: format!("failed to begin write transaction: {e}"),
            })?;
            {
                let mut table = write_txn
                    .open_table(CHECKPOINTS)
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to open checkpoints table: {e}"),
                    })?;
                table
                    .insert(run_id.as_str(), json_bytes.as_slice())
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to insert checkpoint: {e}"),
                    })?;
            }
            {
                let mut meta = write_txn
                    .open_table(METADATA)
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to open metadata table: {e}"),
                    })?;
                meta.insert(KEY_LATEST_RUN_ID, run_id.as_str())
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to insert latest_run_id: {e}"),
                    })?;
            }
            write_txn.commit().map_err(|e| StateError::StoreError {
                message: format!("failed to commit transaction: {e}"),
            })?;
            Ok(())
        })
        .await
        .map_err(|e| StateError::StoreError {
            message: format!("spawn_blocking join error: {e}"),
        })?
    }

    /// Load the most recent checkpoint.
    ///
    /// Reads `METADATA["latest_run_id"]` to find the current run,
    /// then reads `CHECKPOINTS[run_id]` and deserializes from JSON.
    /// Returns `Ok(None)` if no checkpoint has been saved yet.
    async fn load_checkpoint(&self) -> std::result::Result<Option<Checkpoint>, StateError> {
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let read_txn = db.begin_read().map_err(|e| StateError::StoreError {
                message: format!("failed to begin read transaction: {e}"),
            })?;

            // Read the latest run_id from metadata.
            let meta = read_txn
                .open_table(METADATA)
                .map_err(|e| StateError::StoreError {
                    message: format!("failed to open metadata table: {e}"),
                });

            // If the metadata table doesn't exist yet, no checkpoint was saved.
            let meta = match meta {
                Ok(table) => table,
                Err(_) => return Ok(None),
            };

            let run_id = match meta.get(KEY_LATEST_RUN_ID).map_err(|e| {
                StateError::StoreError {
                    message: format!("failed to read latest_run_id: {e}"),
                }
            })? {
                Some(value) => value.value().to_owned(),
                None => return Ok(None),
            };

            // Read the checkpoint bytes.
            let table = read_txn
                .open_table(CHECKPOINTS)
                .map_err(|e| StateError::StoreError {
                    message: format!("failed to open checkpoints table: {e}"),
                })?;

            let checkpoint_bytes = match table.get(run_id.as_str()).map_err(|e| {
                StateError::StoreError {
                    message: format!("failed to read checkpoint: {e}"),
                }
            })? {
                Some(value) => value.value().to_owned(),
                None => return Ok(None),
            };

            // Deserialize from JSON.
            let checkpoint: Checkpoint =
                serde_json::from_slice(&checkpoint_bytes).map_err(|e| {
                    StateError::SerializationError {
                        message: format!("failed to deserialize checkpoint: {e}"),
                    }
                })?;

            Ok(Some(checkpoint))
        })
        .await
        .map_err(|e| StateError::StoreError {
            message: format!("spawn_blocking join error: {e}"),
        })?
    }

    /// Load content hashes from the most recent completed run.
    ///
    /// Reads `METADATA["latest_completed_run_id"]`. If present, reads
    /// all entries from the `HASHES` table. Returns an empty map if no
    /// completed run exists.
    async fn load_previous_hashes(
        &self,
    ) -> std::result::Result<BTreeMap<String, Blake3Hash>, StateError> {
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let read_txn = db.begin_read().map_err(|e| StateError::StoreError {
                message: format!("failed to begin read transaction: {e}"),
            })?;

            // Check if metadata table exists and has a completed run.
            let meta = match read_txn.open_table(METADATA) {
                Ok(table) => table,
                Err(_) => return Ok(BTreeMap::new()),
            };

            let _completed_run_id = match meta
                .get(KEY_LATEST_COMPLETED_RUN_ID)
                .map_err(|e| StateError::StoreError {
                    message: format!("failed to read latest_completed_run_id: {e}"),
                })? {
                Some(value) => value.value().to_owned(),
                None => return Ok(BTreeMap::new()),
            };

            // Read all hashes from the HASHES table.
            let table = match read_txn.open_table(HASHES) {
                Ok(table) => table,
                Err(_) => return Ok(BTreeMap::new()),
            };

            let mut hashes = BTreeMap::new();
            let iter = table.iter().map_err(|e| StateError::StoreError {
                message: format!("failed to iterate hashes table: {e}"),
            })?;

            for entry in iter {
                let entry = entry.map_err(|e| StateError::StoreError {
                    message: format!("failed to read hash entry: {e}"),
                })?;
                let item_id = entry.0.value().to_owned();
                let hash_hex = entry.1.value().to_owned();
                hashes.insert(item_id, Blake3Hash::new(hash_hex));
            }

            Ok(hashes)
        })
        .await
        .map_err(|e| StateError::StoreError {
            message: format!("spawn_blocking join error: {e}"),
        })?
    }

    /// Save content hashes at the end of a successful run.
    ///
    /// Clears the `HASHES` table, writes all new hashes, and updates
    /// `METADATA["latest_completed_run_id"]`. All in a single ACID
    /// transaction.
    async fn save_completed_hashes(
        &self,
        run_id: &RunId,
        hashes: &BTreeMap<String, Blake3Hash>,
    ) -> std::result::Result<(), StateError> {
        let db = self.db.clone();
        let run_id_str = run_id.as_str().to_owned();
        let hashes_owned: Vec<(String, String)> = hashes
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().to_owned()))
            .collect();

        tokio::task::spawn_blocking(move || {
            let write_txn = db.begin_write().map_err(|e| StateError::StoreError {
                message: format!("failed to begin write transaction: {e}"),
            })?;
            {
                // Clear existing hashes and write new ones.
                let mut table = write_txn
                    .open_table(HASHES)
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to open hashes table: {e}"),
                    })?;

                // Drain the table by collecting keys first, then removing.
                // redb doesn't have a truncate, so we collect keys and remove.
                let keys: Vec<String> = {
                    let iter = table.iter().map_err(|e| StateError::StoreError {
                        message: format!("failed to iterate hashes for clear: {e}"),
                    })?;
                    let mut keys = Vec::new();
                    for entry in iter {
                        let entry = entry.map_err(|e| StateError::StoreError {
                            message: format!("failed to read hash key: {e}"),
                        })?;
                        keys.push(entry.0.value().to_owned());
                    }
                    keys
                };
                for key in &keys {
                    table
                        .remove(key.as_str())
                        .map_err(|e| StateError::StoreError {
                            message: format!("failed to remove old hash: {e}"),
                        })?;
                }

                // Write new hashes.
                for (item_id, hash_hex) in &hashes_owned {
                    table
                        .insert(item_id.as_str(), hash_hex.as_str())
                        .map_err(|e| StateError::StoreError {
                            message: format!("failed to insert hash: {e}"),
                        })?;
                }
            }
            {
                let mut meta = write_txn
                    .open_table(METADATA)
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to open metadata table: {e}"),
                    })?;
                meta.insert(KEY_LATEST_COMPLETED_RUN_ID, run_id_str.as_str())
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to insert latest_completed_run_id: {e}"),
                    })?;
            }
            write_txn.commit().map_err(|e| StateError::StoreError {
                message: format!("failed to commit transaction: {e}"),
            })?;
            Ok(())
        })
        .await
        .map_err(|e| StateError::StoreError {
            message: format!("spawn_blocking join error: {e}"),
        })?
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Add redb dependency and verify compilation

- [ ] Add `redb = "3"` and `serde_bytes = "0.11"` to root `Cargo.toml` `[workspace.dependencies]` (if not already present from milestone 1.1)
- [ ] Add `redb = { workspace = true }` to `crates/ecl-pipeline-state/Cargo.toml` `[dependencies]`
- [ ] Create empty `crates/ecl-pipeline-state/src/redb_store.rs` with just: `//! Redb-backed StateStore implementation.`
- [ ] Add `pub mod redb_store;` to `src/lib.rs` (after the existing `pub mod memory_store;` line)
- [ ] Run `cargo check -p ecl-pipeline-state` — must pass
- [ ] Commit: `feat(ecl-pipeline-state): add redb dependency for state store`

### Step 2: RedbStateStore struct and open()

- [ ] Add the `RedbStateStore` struct with `db: Arc<Database>` field to `src/redb_store.rs`
- [ ] Add table definitions as module-level constants: `CHECKPOINTS`, `HASHES`, `METADATA`
- [ ] Add metadata key constants: `KEY_LATEST_RUN_ID`, `KEY_LATEST_COMPLETED_RUN_ID`
- [ ] Implement `RedbStateStore::open(path: impl AsRef<Path>)` using `Database::create()`
- [ ] Add `pub use redb_store::RedbStateStore;` to `src/lib.rs`
- [ ] Write tests:
  - `test_redb_store_open_creates_new_db` — open a path in a tempdir, verify the file exists afterward
  - `test_redb_store_open_existing_db` — open twice at the same path (sequentially — drop first, then reopen), no error
  - `test_redb_store_open_invalid_path` — open at a path inside a nonexistent directory, returns `StoreError`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add RedbStateStore struct with open()`

### Step 3: save_checkpoint() and load_checkpoint()

- [ ] Implement `save_checkpoint()` — serialize to JSON, write to `CHECKPOINTS` table, update `METADATA["latest_run_id"]`, all in one transaction, wrapped in `spawn_blocking`
- [ ] Implement `load_checkpoint()` — read `METADATA["latest_run_id"]`, read `CHECKPOINTS[run_id]`, deserialize from JSON, wrapped in `spawn_blocking`. Return `Ok(None)` if no checkpoint exists
- [ ] Write tests (all async with `#[tokio::test]`):
  - `test_redb_store_save_and_load_checkpoint` — save a checkpoint, load it back, verify all fields match
  - `test_redb_store_load_checkpoint_empty` — fresh db returns `Ok(None)`
  - `test_redb_store_save_overwrites_previous` — save two checkpoints with different sequence numbers (same run_id), load returns the second one
  - `test_redb_store_save_checkpoint_serialization_roundtrip` — save a checkpoint with complex nested state (multiple sources, items in various statuses), load and verify every field
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add save/load checkpoint to RedbStateStore`

### Step 4: save_completed_hashes() and load_previous_hashes()

- [ ] Implement `save_completed_hashes()` — clear `HASHES` table, write all new hashes, update `METADATA["latest_completed_run_id"]`, all in one transaction, wrapped in `spawn_blocking`
- [ ] Implement `load_previous_hashes()` — check `METADATA["latest_completed_run_id"]` exists, read all `HASHES` entries, wrapped in `spawn_blocking`. Return empty map if no completed run
- [ ] Write tests (all async with `#[tokio::test]`):
  - `test_redb_store_save_and_load_hashes` — save hashes, load them back, verify all entries match
  - `test_redb_store_load_hashes_empty` — fresh db returns empty map
  - `test_redb_store_save_hashes_replaces_previous` — save hashes for run-1, save different hashes for run-2, load returns only run-2 hashes
  - `test_redb_store_save_hashes_empty_map` — save an empty hash map, load returns empty map
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add save/load hashes to RedbStateStore`

### Step 5: Crash safety and durability tests

- [ ] Write tests (all async with `#[tokio::test]`):
  - `test_redb_store_crash_safety_checkpoint` — open db, save checkpoint, drop `RedbStateStore` (simulating crash), reopen same path, load checkpoint, verify data persists
  - `test_redb_store_crash_safety_hashes` — open db, save hashes, drop store, reopen, load hashes, verify data persists
  - `test_redb_store_checkpoint_and_hashes_independent` — save checkpoint, save hashes for a different run, verify both are independently retrievable
  - `test_redb_store_multiple_runs_checkpoint_overwrite` — save checkpoint for run-1, save checkpoint for run-2, load returns run-2 (latest_run_id updated)
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add crash safety tests for RedbStateStore`

### Step 6: Object safety and trait compatibility

- [ ] Write tests:
  - `test_redb_store_object_safety` — `Box<dyn StateStore>` using `RedbStateStore` compiles and works (save + load)
  - `test_redb_store_implements_send_sync` — static assertion: `fn assert_send_sync<T: Send + Sync>() {}; assert_send_sync::<RedbStateStore>();`
  - `test_redb_store_clone` — clone the store, use both clones to save/load (verifying Arc sharing)
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-state): add trait compatibility tests for RedbStateStore`

### Step 7: Final polish

- [ ] Run `make test` — all tests pass (both `InMemoryStateStore` and `RedbStateStore`)
- [ ] Run `make lint` — no warnings
- [ ] Run `make format` — no changes
- [ ] Verify all public items in `redb_store.rs` have doc comments
- [ ] Verify no `unwrap()` in library code (only in `#[cfg(test)]` blocks)
- [ ] Commit: `feat(ecl-pipeline-state): finalize RedbStateStore implementation`

## 8. Test Fixtures

### Minimal PipelineSpec TOML (for constructing test Checkpoints)

Use this TOML to create a `PipelineSpec` for test fixtures. Parse it with
`PipelineSpec::from_toml()`:

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

### Helper function to build a test Checkpoint

Use this pattern in your tests to construct a `Checkpoint` for save/load testing:

```rust
use chrono::Utc;
use ecl_pipeline_spec::PipelineSpec;
use std::collections::BTreeMap;
use crate::{
    Blake3Hash, Checkpoint, PipelineState, PipelineStats, PipelineStatus,
    RunId, StageId, StageState, StageStatus,
};

/// Build a test checkpoint with the given run_id and sequence number.
fn make_test_checkpoint(run_id: &str, sequence: u64) -> Checkpoint {
    let spec_toml = r#"
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
"#;
    let spec = PipelineSpec::from_toml(spec_toml).unwrap();
    let now = Utc::now();

    let mut stages = BTreeMap::new();
    stages.insert(
        StageId::new("extract"),
        StageState {
            status: StageStatus::Pending,
            items_processed: 0,
            items_failed: 0,
            items_skipped: 0,
            started_at: None,
            completed_at: None,
        },
    );

    let state = PipelineState {
        run_id: RunId::new(run_id),
        pipeline_name: "test-pipeline".to_owned(),
        started_at: now,
        last_checkpoint: now,
        status: PipelineStatus::Pending,
        current_batch: 0,
        sources: BTreeMap::new(),
        stages,
        stats: PipelineStats::default(),
    };

    Checkpoint {
        version: 1,
        sequence,
        created_at: now,
        spec,
        schedule: vec![
            vec![StageId::new("extract")],
            vec![StageId::new("emit")],
        ],
        spec_hash: Blake3Hash::new("abc123def456"),
        state,
    }
}
```

### Helper function to build test hashes

```rust
/// Build a test hash map with the given entries.
fn make_test_hashes(entries: &[(&str, &str)]) -> BTreeMap<String, Blake3Hash> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), Blake3Hash::new(*v)))
        .collect()
}
```

### Example test using tempfile

```rust
use tempfile::TempDir;

#[tokio::test]
async fn test_redb_store_save_and_load_checkpoint() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.redb");
    let store = RedbStateStore::open(&db_path).unwrap();

    let checkpoint = make_test_checkpoint("run-001", 1);
    store.save_checkpoint(&checkpoint).await.unwrap();

    let loaded = store.load_checkpoint().await.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.state.run_id.as_str(), "run-001");
    assert_eq!(loaded.sequence, 1);
}
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_redb_store_open_creates_new_db` | `redb_store` | `open()` creates a new database file at the given path |
| `test_redb_store_open_existing_db` | `redb_store` | `open()` succeeds when the database file already exists (drop first, reopen) |
| `test_redb_store_open_invalid_path` | `redb_store` | `open()` returns `StoreError` for a path inside a nonexistent directory |
| `test_redb_store_save_and_load_checkpoint` | `redb_store` | Save a checkpoint and load it back; all fields match |
| `test_redb_store_load_checkpoint_empty` | `redb_store` | Load from a fresh database returns `Ok(None)` |
| `test_redb_store_save_overwrites_previous` | `redb_store` | Saving two checkpoints (same run_id, different sequence), load returns the second |
| `test_redb_store_save_checkpoint_serialization_roundtrip` | `redb_store` | Save a checkpoint with complex nested state, load and verify every field |
| `test_redb_store_save_and_load_hashes` | `redb_store` | Save hashes and load them back; all entries match |
| `test_redb_store_load_hashes_empty` | `redb_store` | Load hashes from a fresh database returns an empty map |
| `test_redb_store_save_hashes_replaces_previous` | `redb_store` | Save hashes for run-1, then run-2; load returns only run-2 hashes |
| `test_redb_store_save_hashes_empty_map` | `redb_store` | Save an empty hash map; load returns an empty map |
| `test_redb_store_crash_safety_checkpoint` | `redb_store` | Save checkpoint, drop store, reopen, verify data persists |
| `test_redb_store_crash_safety_hashes` | `redb_store` | Save hashes, drop store, reopen, verify data persists |
| `test_redb_store_checkpoint_and_hashes_independent` | `redb_store` | Checkpoint and hashes are independently stored and retrievable |
| `test_redb_store_multiple_runs_checkpoint_overwrite` | `redb_store` | Save checkpoint for run-1, then run-2; load returns run-2 |
| `test_redb_store_object_safety` | `redb_store` | `Box<dyn StateStore>` using `RedbStateStore` compiles and works |
| `test_redb_store_implements_send_sync` | `redb_store` | Static assertion that `RedbStateStore: Send + Sync` |
| `test_redb_store_clone` | `redb_store` | Clone the store, use both clones for save/load (Arc sharing) |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline-state` passes
- [ ] `cargo test -p ecl-pipeline-state` passes (all tests green — both InMemoryStateStore and RedbStateStore tests)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`
- [ ] All redb operations are wrapped in `tokio::task::spawn_blocking` (no sync I/O on async runtime)
- [ ] Crash safety test passes (write, drop, reopen, verify)

### What NOT to Do

- Do NOT modify any existing types (Checkpoint, StateStore, StateError, InMemoryStateStore, etc.)
- Do NOT create a new crate — this milestone adds a single file to `ecl-pipeline-state`
- Do NOT implement execution logic or the pipeline runner (that is milestone 2.2)
- Do NOT implement topology types or resource graph (that is milestone 1.3)
- Do NOT implement CLI commands (that is milestone 5.1)
- Do NOT create adapter or stage implementations (that is milestone 3.1+)
- Do NOT use `HashMap` anywhere — always `BTreeMap` for deterministic serialization
- Do NOT use `unwrap()` in library code — only in `#[cfg(test)]` blocks
- Do NOT use `Database::open()` — use `Database::create()` which handles both create and open
- Do NOT add `redb` to any crate other than `ecl-pipeline-state`
