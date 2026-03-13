//! Pipeline execution state types for the ECL pipeline runner.
//!
//! This crate defines the mutable state layer: everything that changes
//! during pipeline execution. All types derive `Serialize + Deserialize`
//! for checkpointing. The `StateStore` trait provides pluggable persistence.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn test_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 13, 10, 0, 0).unwrap()
    }

    fn make_item(name: &str, status: ItemStatus) -> ItemState {
        ItemState {
            display_name: name.to_string(),
            source_id: name.to_string(),
            source_name: "local".to_string(),
            content_hash: Blake3Hash::new("aabb"),
            status,
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: test_time(),
            },
        }
    }

    fn make_pipeline_state() -> PipelineState {
        let mut sources = BTreeMap::new();
        let mut items = BTreeMap::new();
        items.insert(
            "file1.txt".to_string(),
            make_item("file1.txt", ItemStatus::Completed),
        );
        items.insert(
            "file2.txt".to_string(),
            make_item(
                "file2.txt",
                ItemStatus::Processing {
                    stage: "extract".to_string(),
                },
            ),
        );
        sources.insert(
            "local".to_string(),
            SourceState {
                items_discovered: 3,
                items_accepted: 2,
                items_skipped_unchanged: 1,
                items,
            },
        );

        let mut stages = BTreeMap::new();
        stages.insert(
            StageId::new("extract"),
            StageState {
                status: StageStatus::Running,
                items_processed: 1,
                items_failed: 0,
                items_skipped: 0,
                started_at: Some(test_time()),
                completed_at: None,
            },
        );

        PipelineState {
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
        }
    }

    #[test]
    fn test_pipeline_state_serde_roundtrip() {
        let state = make_pipeline_state();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: PipelineState = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string_pretty(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_pipeline_state_update_stats_empty() {
        let mut state = PipelineState {
            run_id: RunId::new("run-empty"),
            pipeline_name: "empty".to_string(),
            started_at: test_time(),
            last_checkpoint: test_time(),
            status: PipelineStatus::Pending,
            current_batch: 0,
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            stats: PipelineStats::default(),
        };
        state.update_stats();
        assert_eq!(
            state.stats,
            PipelineStats {
                total_items_discovered: 0,
                total_items_processed: 0,
                total_items_skipped_unchanged: 0,
                total_items_failed: 0,
            }
        );
    }

    #[test]
    fn test_pipeline_state_update_stats_mixed_items() {
        let mut state = make_pipeline_state();
        // Add more items with different statuses
        let source = state.sources.get_mut("local").unwrap();
        source.items.insert(
            "failed.txt".to_string(),
            make_item(
                "failed.txt",
                ItemStatus::Failed {
                    stage: "extract".to_string(),
                    error: "boom".to_string(),
                    attempts: 1,
                },
            ),
        );
        source.items.insert(
            "unchanged.txt".to_string(),
            make_item("unchanged.txt", ItemStatus::Unchanged),
        );

        state.update_stats();
        // 1 Completed
        assert_eq!(state.stats.total_items_processed, 1);
        // 1 Failed
        assert_eq!(state.stats.total_items_failed, 1);
        // 1 from source.items_skipped_unchanged + 1 from ItemStatus::Unchanged
        assert_eq!(state.stats.total_items_skipped_unchanged, 2);
        // 3 from source.items_discovered
        assert_eq!(state.stats.total_items_discovered, 3);
    }

    #[test]
    fn test_pipeline_state_update_stats_counts_discovered_from_source() {
        let mut state = PipelineState {
            run_id: RunId::new("run-disc"),
            pipeline_name: "test".to_string(),
            started_at: test_time(),
            last_checkpoint: test_time(),
            status: PipelineStatus::Pending,
            current_batch: 0,
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            stats: PipelineStats::default(),
        };
        state.sources.insert(
            "source-a".to_string(),
            SourceState {
                items_discovered: 10,
                items_accepted: 5,
                items_skipped_unchanged: 0,
                items: BTreeMap::new(),
            },
        );
        state.sources.insert(
            "source-b".to_string(),
            SourceState {
                items_discovered: 7,
                items_accepted: 3,
                items_skipped_unchanged: 0,
                items: BTreeMap::new(),
            },
        );
        state.update_stats();
        assert_eq!(state.stats.total_items_discovered, 17);
    }

    #[test]
    fn test_pipeline_state_update_stats_counts_unchanged_from_both() {
        let mut state = PipelineState {
            run_id: RunId::new("run-unch"),
            pipeline_name: "test".to_string(),
            started_at: test_time(),
            last_checkpoint: test_time(),
            status: PipelineStatus::Pending,
            current_batch: 0,
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            stats: PipelineStats::default(),
        };
        let mut items = BTreeMap::new();
        items.insert(
            "unch1.txt".to_string(),
            make_item("unch1.txt", ItemStatus::Unchanged),
        );
        items.insert(
            "unch2.txt".to_string(),
            make_item("unch2.txt", ItemStatus::Unchanged),
        );
        state.sources.insert(
            "local".to_string(),
            SourceState {
                items_discovered: 5,
                items_accepted: 3,
                items_skipped_unchanged: 3, // source-level count
                items,
            },
        );
        state.update_stats();
        // 3 from source.items_skipped_unchanged + 2 from ItemStatus::Unchanged
        assert_eq!(state.stats.total_items_skipped_unchanged, 5);
    }
}
