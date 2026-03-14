//! Redb-backed StateStore implementation.
//!
//! Provides crash-safe, ACID-transactional persistence for pipeline
//! checkpoints and content hashes using [redb](https://docs.rs/redb).
//! All redb operations are synchronous I/O; this module wraps them in
//! `tokio::task::spawn_blocking` to avoid blocking the async runtime.

use async_trait::async_trait;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
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
        let json_bytes =
            serde_json::to_vec(checkpoint).map_err(|e| StateError::SerializationError {
                message: format!("failed to serialize checkpoint: {e}"),
            })?;
        let run_id = checkpoint.state.run_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let write_txn = db.begin_write().map_err(|e| StateError::StoreError {
                message: format!("failed to begin write transaction: {e}"),
            })?;
            {
                let mut table =
                    write_txn
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
                let mut meta =
                    write_txn
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
            // If the metadata table doesn't exist yet, no checkpoint was saved.
            let meta: redb::ReadOnlyTable<&str, &str> = match read_txn.open_table(METADATA) {
                Ok(table) => table,
                Err(_) => return Ok(None),
            };

            let run_id: String =
                match meta
                    .get(KEY_LATEST_RUN_ID)
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to read latest_run_id: {e}"),
                    })? {
                    Some(value) => value.value().to_owned(),
                    None => return Ok(None),
                };

            // Read the checkpoint bytes.
            let table: redb::ReadOnlyTable<&str, &[u8]> = read_txn
                .open_table(CHECKPOINTS)
                .map_err(|e| StateError::StoreError {
                    message: format!("failed to open checkpoints table: {e}"),
                })?;

            let checkpoint_bytes: Vec<u8> =
                match table
                    .get(run_id.as_str())
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to read checkpoint: {e}"),
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
            let meta: redb::ReadOnlyTable<&str, &str> = match read_txn.open_table(METADATA) {
                Ok(table) => table,
                Err(_) => return Ok(BTreeMap::new()),
            };

            let _completed_run_id: String =
                match meta
                    .get(KEY_LATEST_COMPLETED_RUN_ID)
                    .map_err(|e| StateError::StoreError {
                        message: format!("failed to read latest_completed_run_id: {e}"),
                    })? {
                    Some(value) => value.value().to_owned(),
                    None => return Ok(BTreeMap::new()),
                };

            // Read all hashes from the HASHES table.
            let table: redb::ReadOnlyTable<&str, &str> = match read_txn.open_table(HASHES) {
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
                let mut table =
                    write_txn
                        .open_table(HASHES)
                        .map_err(|e| StateError::StoreError {
                            message: format!("failed to open hashes table: {e}"),
                        })?;

                // Drain the table by collecting keys first, then removing.
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
                let mut meta =
                    write_txn
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{
        ItemProvenance, ItemState, ItemStatus, PipelineStats, SourceState, StageState, StageStatus,
    };
    use crate::{PipelineState, PipelineStatus};
    use chrono::Utc;
    use ecl_pipeline_spec::PipelineSpec;
    use tempfile::TempDir;

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

    fn make_test_checkpoint(run_id: &str, sequence: u64) -> Checkpoint {
        let spec = PipelineSpec::from_toml(MINIMAL_TOML).unwrap();
        let now = Utc::now();

        let mut stages = BTreeMap::new();
        stages.insert(
            crate::StageId::new("extract"),
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
                vec![crate::StageId::new("extract")],
                vec![crate::StageId::new("emit")],
            ],
            spec_hash: Blake3Hash::new("abc123def456"),
            state,
        }
    }

    fn make_test_hashes(entries: &[(&str, &str)]) -> BTreeMap<String, Blake3Hash> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), Blake3Hash::new(*v)))
            .collect()
    }

    // --- open() tests ---

    #[test]
    fn test_redb_store_open_creates_new_db() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        assert!(!db_path.exists());
        let _store = RedbStateStore::open(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_redb_store_open_existing_db() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        {
            let _store = RedbStateStore::open(&db_path).unwrap();
        }
        // Reopen — should succeed.
        let _store2 = RedbStateStore::open(&db_path).unwrap();
    }

    #[test]
    fn test_redb_store_open_invalid_path() {
        let result = RedbStateStore::open("/nonexistent/directory/test.redb");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, StateError::StoreError { .. }));
    }

    // --- save/load checkpoint tests ---

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

    #[tokio::test]
    async fn test_redb_store_load_checkpoint_empty() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_redb_store_save_overwrites_previous() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let cp1 = make_test_checkpoint("run-001", 1);
        store.save_checkpoint(&cp1).await.unwrap();

        let cp2 = make_test_checkpoint("run-001", 2);
        store.save_checkpoint(&cp2).await.unwrap();

        let loaded = store.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.sequence, 2);
    }

    #[tokio::test]
    async fn test_redb_store_save_checkpoint_serialization_roundtrip() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        // Build a checkpoint with complex nested state.
        let mut checkpoint = make_test_checkpoint("run-complex", 42);
        checkpoint.state.status = PipelineStatus::Running {
            current_stage: "extract".to_string(),
        };

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
                    extracted_at: Utc::now(),
                },
            },
        );
        source_items.insert(
            "file2.txt".to_string(),
            ItemState {
                display_name: "file2.txt".to_string(),
                source_id: "file2.txt".to_string(),
                source_name: "local".to_string(),
                content_hash: Blake3Hash::new("ccdd"),
                status: ItemStatus::Failed {
                    stage: "extract".to_string(),
                    error: "parse error".to_string(),
                    attempts: 3,
                },
                completed_stages: vec![],
                provenance: ItemProvenance {
                    source_kind: "filesystem".to_string(),
                    metadata: BTreeMap::new(),
                    source_modified: None,
                    extracted_at: Utc::now(),
                },
            },
        );
        checkpoint.state.sources.insert(
            "local".to_string(),
            SourceState {
                items_discovered: 5,
                items_accepted: 3,
                items_skipped_unchanged: 2,
                items: source_items,
            },
        );

        store.save_checkpoint(&checkpoint).await.unwrap();
        let loaded = store.load_checkpoint().await.unwrap().unwrap();

        assert_eq!(loaded.state.run_id.as_str(), "run-complex");
        assert_eq!(loaded.sequence, 42);
        assert!(matches!(
            loaded.state.status,
            PipelineStatus::Running { .. }
        ));
        let source = &loaded.state.sources["local"];
        assert_eq!(source.items_discovered, 5);
        assert_eq!(source.items.len(), 2);
        assert!(matches!(
            source.items["file1.txt"].status,
            ItemStatus::Completed
        ));
        assert!(matches!(
            source.items["file2.txt"].status,
            ItemStatus::Failed { .. }
        ));
    }

    // --- save/load hashes tests ---

    #[tokio::test]
    async fn test_redb_store_save_and_load_hashes() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let hashes = make_test_hashes(&[("file1.txt", "aabb"), ("file2.txt", "ccdd")]);
        let run_id = RunId::new("run-001");
        store.save_completed_hashes(&run_id, &hashes).await.unwrap();

        let loaded = store.load_previous_hashes().await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded["file1.txt"].as_str(), "aabb");
        assert_eq!(loaded["file2.txt"].as_str(), "ccdd");
    }

    #[tokio::test]
    async fn test_redb_store_load_hashes_empty() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let loaded = store.load_previous_hashes().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_redb_store_save_hashes_replaces_previous() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let hashes1 = make_test_hashes(&[("file1.txt", "aabb"), ("file2.txt", "ccdd")]);
        store
            .save_completed_hashes(&RunId::new("run-001"), &hashes1)
            .await
            .unwrap();

        let hashes2 = make_test_hashes(&[("file3.txt", "eeff")]);
        store
            .save_completed_hashes(&RunId::new("run-002"), &hashes2)
            .await
            .unwrap();

        let loaded = store.load_previous_hashes().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded["file3.txt"].as_str(), "eeff");
        assert!(!loaded.contains_key("file1.txt"));
    }

    #[tokio::test]
    async fn test_redb_store_save_hashes_empty_map() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let hashes = BTreeMap::new();
        store
            .save_completed_hashes(&RunId::new("run-001"), &hashes)
            .await
            .unwrap();

        let loaded = store.load_previous_hashes().await.unwrap();
        assert!(loaded.is_empty());
    }

    // --- crash safety tests ---

    #[tokio::test]
    async fn test_redb_store_crash_safety_checkpoint() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");

        {
            let store = RedbStateStore::open(&db_path).unwrap();
            let checkpoint = make_test_checkpoint("run-crash", 99);
            store.save_checkpoint(&checkpoint).await.unwrap();
            // Drop store — simulates crash.
        }

        // Reopen and verify.
        let store2 = RedbStateStore::open(&db_path).unwrap();
        let loaded = store2.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.state.run_id.as_str(), "run-crash");
        assert_eq!(loaded.sequence, 99);
    }

    #[tokio::test]
    async fn test_redb_store_crash_safety_hashes() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");

        {
            let store = RedbStateStore::open(&db_path).unwrap();
            let hashes = make_test_hashes(&[("a.txt", "1111"), ("b.txt", "2222")]);
            store
                .save_completed_hashes(&RunId::new("run-001"), &hashes)
                .await
                .unwrap();
        }

        let store2 = RedbStateStore::open(&db_path).unwrap();
        let loaded = store2.load_previous_hashes().await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded["a.txt"].as_str(), "1111");
    }

    #[tokio::test]
    async fn test_redb_store_checkpoint_and_hashes_independent() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let checkpoint = make_test_checkpoint("run-001", 1);
        store.save_checkpoint(&checkpoint).await.unwrap();

        let hashes = make_test_hashes(&[("file.txt", "abcd")]);
        store
            .save_completed_hashes(&RunId::new("run-002"), &hashes)
            .await
            .unwrap();

        // Both should be independently retrievable.
        let loaded_cp = store.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded_cp.state.run_id.as_str(), "run-001");

        let loaded_hashes = store.load_previous_hashes().await.unwrap();
        assert_eq!(loaded_hashes.len(), 1);
        assert_eq!(loaded_hashes["file.txt"].as_str(), "abcd");
    }

    #[tokio::test]
    async fn test_redb_store_multiple_runs_checkpoint_overwrite() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        let cp1 = make_test_checkpoint("run-001", 1);
        store.save_checkpoint(&cp1).await.unwrap();

        let cp2 = make_test_checkpoint("run-002", 1);
        store.save_checkpoint(&cp2).await.unwrap();

        let loaded = store.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.state.run_id.as_str(), "run-002");
    }

    // --- trait compatibility tests ---

    #[tokio::test]
    async fn test_redb_store_object_safety() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store: Box<dyn StateStore> = Box::new(RedbStateStore::open(&db_path).unwrap());

        let checkpoint = make_test_checkpoint("run-obj", 1);
        store.save_checkpoint(&checkpoint).await.unwrap();

        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn test_redb_store_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RedbStateStore>();
    }

    #[tokio::test]
    async fn test_redb_store_load_checkpoint_metadata_exists_but_no_run_id() {
        // Create a db with metadata table but without latest_run_id key.
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        {
            let db = Database::create(&db_path).unwrap();
            let write_txn = db.begin_write().unwrap();
            {
                let mut meta = write_txn.open_table(METADATA).unwrap();
                // Write some other key, not latest_run_id.
                meta.insert("other_key", "other_value").unwrap();
            }
            write_txn.commit().unwrap();
        }

        let store = RedbStateStore::open(&db_path).unwrap();
        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_redb_store_load_checkpoint_run_id_exists_but_no_checkpoint_data() {
        // Create a db with metadata pointing to a run_id, but no checkpoint data.
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        {
            let db = Database::create(&db_path).unwrap();
            let write_txn = db.begin_write().unwrap();
            {
                let mut meta = write_txn.open_table(METADATA).unwrap();
                meta.insert(KEY_LATEST_RUN_ID, "ghost-run").unwrap();
            }
            // Create the checkpoints table but don't insert anything.
            {
                let _table = write_txn.open_table(CHECKPOINTS).unwrap();
            }
            write_txn.commit().unwrap();
        }

        let store = RedbStateStore::open(&db_path).unwrap();
        let loaded = store.load_checkpoint().await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_redb_store_load_hashes_metadata_exists_but_no_completed_run() {
        // Metadata table exists (from a checkpoint save) but no completed run.
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store = RedbStateStore::open(&db_path).unwrap();

        // Save a checkpoint — this creates the metadata table with latest_run_id.
        let checkpoint = make_test_checkpoint("run-001", 1);
        store.save_checkpoint(&checkpoint).await.unwrap();

        // Now load hashes — metadata table exists, but no latest_completed_run_id.
        let loaded = store.load_previous_hashes().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_redb_store_load_hashes_completed_run_exists_but_no_hashes_table() {
        // Set metadata with completed run but don't create the hashes table.
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        {
            let db = Database::create(&db_path).unwrap();
            let write_txn = db.begin_write().unwrap();
            {
                let mut meta = write_txn.open_table(METADATA).unwrap();
                meta.insert(KEY_LATEST_COMPLETED_RUN_ID, "run-001").unwrap();
            }
            write_txn.commit().unwrap();
        }

        let store = RedbStateStore::open(&db_path).unwrap();
        let loaded = store.load_previous_hashes().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_redb_store_load_checkpoint_corrupt_data() {
        // Write garbage bytes to the checkpoints table to trigger deserialization error.
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        {
            let db = Database::create(&db_path).unwrap();
            let write_txn = db.begin_write().unwrap();
            {
                let mut table = write_txn.open_table(CHECKPOINTS).unwrap();
                table
                    .insert("corrupt-run", b"not valid json".as_slice())
                    .unwrap();
            }
            {
                let mut meta = write_txn.open_table(METADATA).unwrap();
                meta.insert(KEY_LATEST_RUN_ID, "corrupt-run").unwrap();
            }
            write_txn.commit().unwrap();
        }

        let store = RedbStateStore::open(&db_path).unwrap();
        let result = store.load_checkpoint().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, StateError::SerializationError { .. }));
    }

    #[tokio::test]
    async fn test_redb_store_clone() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.redb");
        let store1 = RedbStateStore::open(&db_path).unwrap();
        let store2 = store1.clone();

        let checkpoint = make_test_checkpoint("run-clone", 1);
        store1.save_checkpoint(&checkpoint).await.unwrap();

        // Load from the clone — should see the data (Arc sharing).
        let loaded = store2.load_checkpoint().await.unwrap().unwrap();
        assert_eq!(loaded.state.run_id.as_str(), "run-clone");
    }
}
