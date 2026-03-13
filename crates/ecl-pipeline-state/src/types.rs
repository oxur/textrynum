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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn test_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 13, 10, 0, 0).unwrap()
    }

    #[test]
    fn test_pipeline_status_all_variants_serde() {
        let variants: Vec<PipelineStatus> = vec![
            PipelineStatus::Pending,
            PipelineStatus::Running {
                current_stage: "extract".to_string(),
            },
            PipelineStatus::Completed {
                finished_at: test_time(),
            },
            PipelineStatus::Failed {
                error: "boom".to_string(),
                failed_at: test_time(),
            },
            PipelineStatus::Interrupted {
                interrupted_at: test_time(),
            },
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let _deserialized: PipelineStatus = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_source_state_default() {
        let state = SourceState::default();
        assert_eq!(state.items_discovered, 0);
        assert_eq!(state.items_accepted, 0);
        assert_eq!(state.items_skipped_unchanged, 0);
        assert!(state.items.is_empty());
    }

    #[test]
    fn test_item_state_serde_roundtrip() {
        let item = ItemState {
            display_name: "doc.pdf".to_string(),
            source_id: "file-123".to_string(),
            source_name: "local".to_string(),
            content_hash: Blake3Hash::new("aabb"),
            status: ItemStatus::Completed,
            completed_stages: vec![CompletedStageRecord {
                stage: StageId::new("extract"),
                completed_at: test_time(),
                duration_ms: 150,
            }],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: Some(test_time()),
                extracted_at: test_time(),
            },
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: ItemState = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_item_status_all_variants_serde() {
        let variants: Vec<ItemStatus> = vec![
            ItemStatus::Pending,
            ItemStatus::Processing {
                stage: "extract".to_string(),
            },
            ItemStatus::Completed,
            ItemStatus::Failed {
                stage: "normalize".to_string(),
                error: "parse error".to_string(),
                attempts: 3,
            },
            ItemStatus::Skipped {
                stage: "emit".to_string(),
                reason: "condition false".to_string(),
            },
            ItemStatus::Unchanged,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let _deserialized: ItemStatus = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_completed_stage_record_serde() {
        let record = CompletedStageRecord {
            stage: StageId::new("extract"),
            completed_at: test_time(),
            duration_ms: 200,
        };
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: CompletedStageRecord = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_item_provenance_serde_roundtrip() {
        let mut metadata = BTreeMap::new();
        metadata.insert("file_id".to_string(), serde_json::json!("abc123"));
        metadata.insert(
            "path".to_string(),
            serde_json::json!("/Engineering/doc.pdf"),
        );

        let prov = ItemProvenance {
            source_kind: "google_drive".to_string(),
            metadata,
            source_modified: Some(test_time()),
            extracted_at: test_time(),
        };
        let json = serde_json::to_string(&prov).unwrap();
        let deserialized: ItemProvenance = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_stage_state_serde_roundtrip() {
        let state = StageState {
            status: StageStatus::Running,
            items_processed: 5,
            items_failed: 1,
            items_skipped: 2,
            started_at: Some(test_time()),
            completed_at: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: StageState = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_stage_status_all_variants_serde() {
        let variants: Vec<StageStatus> = vec![
            StageStatus::Pending,
            StageStatus::Running,
            StageStatus::Completed,
            StageStatus::Skipped {
                reason: "condition".to_string(),
            },
            StageStatus::Failed {
                error: "timeout".to_string(),
            },
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let _deserialized: StageStatus = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_pipeline_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.total_items_discovered, 0);
        assert_eq!(stats.total_items_processed, 0);
        assert_eq!(stats.total_items_skipped_unchanged, 0);
        assert_eq!(stats.total_items_failed, 0);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_pipeline_stats_proptest_roundtrip(
            discovered in 0..10_000usize,
            processed in 0..10_000usize,
            skipped in 0..10_000usize,
            failed in 0..10_000usize,
        ) {
            let stats = PipelineStats {
                total_items_discovered: discovered,
                total_items_processed: processed,
                total_items_skipped_unchanged: skipped,
                total_items_failed: failed,
            };
            let json = serde_json::to_string(&stats).unwrap();
            let deserialized: PipelineStats = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(stats, deserialized);
        }
    }
}
