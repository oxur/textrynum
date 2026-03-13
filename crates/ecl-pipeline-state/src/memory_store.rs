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
    async fn save_checkpoint(
        &self,
        checkpoint: &Checkpoint,
    ) -> std::result::Result<(), StateError> {
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::PipelineState;
    use crate::PipelineStatus;
    use crate::ids::StageId;
    use crate::types::{
        ItemProvenance, ItemState, ItemStatus, PipelineStats, SourceState, StageState, StageStatus,
    };
    use chrono::{TimeZone, Utc};
    use ecl_pipeline_spec::PipelineSpec;

    const MINIMAL_TOML: &str = r#"
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

    fn test_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 13, 10, 0, 0).unwrap()
    }

    fn make_checkpoint() -> Checkpoint {
        let spec = PipelineSpec::from_toml(MINIMAL_TOML).unwrap();

        let mut source_items = BTreeMap::new();
        source_items.insert(
            "file1.txt".to_string(),
            ItemState {
                display_name: "file1.txt".to_string(),
                source_id: "file1.txt".to_string(),
                source_name: "local".to_string(),
                content_hash: Blake3Hash::new("aabb"),
                status: ItemStatus::Completed,
                completed_stages: vec![],
                provenance: ItemProvenance {
                    source_kind: "filesystem".to_string(),
                    metadata: BTreeMap::new(),
                    source_modified: None,
                    extracted_at: test_time(),
                },
            },
        );

        let mut sources = BTreeMap::new();
        sources.insert(
            "local".to_string(),
            SourceState {
                items_discovered: 1,
                items_accepted: 1,
                items_skipped_unchanged: 0,
                items: source_items,
            },
        );

        let mut stages = BTreeMap::new();
        stages.insert(
            StageId::new("extract"),
            StageState {
                status: StageStatus::Completed,
                items_processed: 1,
                items_failed: 0,
                items_skipped: 0,
                started_at: Some(test_time()),
                completed_at: Some(test_time()),
            },
        );

        let state = PipelineState {
            run_id: RunId::new("run-001"),
            pipeline_name: "test-pipeline".to_string(),
            started_at: test_time(),
            last_checkpoint: test_time(),
            status: PipelineStatus::Running {
                current_stage: "extract".to_string(),
            },
            current_batch: 0,
            sources,
            stages,
            stats: PipelineStats::default(),
        };

        Checkpoint {
            version: 1,
            sequence: 1,
            created_at: test_time(),
            spec,
            schedule: vec![vec![StageId::new("extract")], vec![StageId::new("emit")]],
            spec_hash: Blake3Hash::new("abc123"),
            state,
        }
    }

    #[tokio::test]
    async fn test_memory_store_save_and_load_checkpoint() {
        let store = InMemoryStateStore::new();
        let checkpoint = make_checkpoint();
        store.save_checkpoint(&checkpoint).await.unwrap();
        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.version, checkpoint.version);
        assert_eq!(loaded.sequence, checkpoint.sequence);
    }

    #[tokio::test]
    async fn test_memory_store_load_checkpoint_empty() {
        let store = InMemoryStateStore::new();
        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_memory_store_save_overwrites_previous() {
        let store = InMemoryStateStore::new();
        let mut checkpoint1 = make_checkpoint();
        checkpoint1.sequence = 1;
        store.save_checkpoint(&checkpoint1).await.unwrap();

        let mut checkpoint2 = make_checkpoint();
        checkpoint2.sequence = 2;
        store.save_checkpoint(&checkpoint2).await.unwrap();

        let loaded = store.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.sequence, 2);
    }

    #[tokio::test]
    async fn test_memory_store_save_and_load_hashes() {
        let store = InMemoryStateStore::new();
        let run_id = RunId::new("run-001");
        let mut hashes = BTreeMap::new();
        hashes.insert("file1.txt".to_string(), Blake3Hash::new("aabb"));
        hashes.insert("file2.txt".to_string(), Blake3Hash::new("ccdd"));

        store.save_completed_hashes(&run_id, &hashes).await.unwrap();
        let loaded = store.load_previous_hashes().await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded["file1.txt"].as_str(), "aabb");
        assert_eq!(loaded["file2.txt"].as_str(), "ccdd");
    }

    #[tokio::test]
    async fn test_memory_store_load_hashes_empty() {
        let store = InMemoryStateStore::new();
        let loaded = store.load_previous_hashes().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_memory_store_object_safety() {
        let store: Box<dyn StateStore> = Box::new(InMemoryStateStore::new());
        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_none());
    }
}
