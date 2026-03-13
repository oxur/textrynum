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
    async fn save_checkpoint(&self, checkpoint: &Checkpoint)
    -> std::result::Result<(), StateError>;

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
