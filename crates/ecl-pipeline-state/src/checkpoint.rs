//! Checkpoint: self-contained recovery artifact.
//!
//! Bundles all three layers (spec, topology schedule, state) so resume
//! needs nothing external.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::PipelineState;
use crate::PipelineStatus;
use crate::ids::{Blake3Hash, StageId};
use crate::types::{ItemStatus, StageStatus};
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::ids::RunId;
    use crate::types::{
        ItemProvenance, ItemState, PipelineStats, SourceState, StageState, StageStatus,
    };
    use chrono::TimeZone;
    use ecl_pipeline_spec::PipelineSpec;
    use std::collections::BTreeMap;

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

    fn make_checkpoint() -> Checkpoint {
        let spec = PipelineSpec::from_toml(MINIMAL_TOML).unwrap();

        let mut source_items = BTreeMap::new();
        source_items.insert(
            "file1.txt".to_string(),
            make_item("file1.txt", ItemStatus::Completed),
        );
        source_items.insert(
            "file2.txt".to_string(),
            make_item(
                "file2.txt",
                ItemStatus::Processing {
                    stage: "extract".to_string(),
                },
            ),
        );

        let mut sources = BTreeMap::new();
        sources.insert(
            "local".to_string(),
            SourceState {
                items_discovered: 3,
                items_accepted: 2,
                items_skipped_unchanged: 1,
                items: source_items,
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
            sequence: 42,
            created_at: test_time(),
            spec,
            schedule: vec![vec![StageId::new("extract")], vec![StageId::new("emit")]],
            spec_hash: Blake3Hash::new("abc123def456"),
            state,
        }
    }

    #[test]
    fn test_checkpoint_serde_roundtrip_json() {
        let checkpoint = make_checkpoint();
        let json = serde_json::to_string_pretty(&checkpoint).unwrap();
        let deserialized: Checkpoint = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string_pretty(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_checkpoint_prepare_for_resume_resets_processing_items() {
        let mut checkpoint = make_checkpoint();
        checkpoint.prepare_for_resume();

        for source_state in checkpoint.state.sources.values() {
            for item in source_state.items.values() {
                assert!(
                    !matches!(item.status, ItemStatus::Processing { .. }),
                    "No items should be in Processing after prepare_for_resume"
                );
            }
        }
    }

    #[test]
    fn test_checkpoint_prepare_for_resume_resets_running_stages() {
        let mut checkpoint = make_checkpoint();
        checkpoint.prepare_for_resume();

        for stage in checkpoint.state.stages.values() {
            assert!(
                !matches!(stage.status, StageStatus::Running),
                "No stages should be in Running after prepare_for_resume"
            );
        }
    }

    #[test]
    fn test_checkpoint_prepare_for_resume_sets_interrupted() {
        let mut checkpoint = make_checkpoint();
        checkpoint.prepare_for_resume();

        assert!(matches!(
            checkpoint.state.status,
            PipelineStatus::Interrupted { .. }
        ));
    }

    #[test]
    fn test_checkpoint_prepare_for_resume_leaves_completed_items() {
        let mut checkpoint = make_checkpoint();
        // Add items in various terminal states
        let source = checkpoint.state.sources.get_mut("local").unwrap();
        source.items.insert(
            "completed.txt".to_string(),
            make_item("completed.txt", ItemStatus::Completed),
        );
        source.items.insert(
            "failed.txt".to_string(),
            make_item(
                "failed.txt",
                ItemStatus::Failed {
                    stage: "extract".to_string(),
                    error: "boom".to_string(),
                    attempts: 3,
                },
            ),
        );
        source.items.insert(
            "skipped.txt".to_string(),
            make_item(
                "skipped.txt",
                ItemStatus::Skipped {
                    stage: "emit".to_string(),
                    reason: "condition".to_string(),
                },
            ),
        );

        checkpoint.prepare_for_resume();

        let source = &checkpoint.state.sources["local"];
        assert!(matches!(
            source.items["completed.txt"].status,
            ItemStatus::Completed
        ));
        assert!(matches!(
            source.items["failed.txt"].status,
            ItemStatus::Failed { .. }
        ));
        assert!(matches!(
            source.items["skipped.txt"].status,
            ItemStatus::Skipped { .. }
        ));
    }

    #[test]
    fn test_checkpoint_config_drifted_same_hash() {
        let checkpoint = make_checkpoint();
        let same = Blake3Hash::new("abc123def456");
        assert!(!checkpoint.config_drifted(&same));
    }

    #[test]
    fn test_checkpoint_config_drifted_different_hash() {
        let checkpoint = make_checkpoint();
        let different = Blake3Hash::new("different_hash");
        assert!(checkpoint.config_drifted(&different));
    }
}
