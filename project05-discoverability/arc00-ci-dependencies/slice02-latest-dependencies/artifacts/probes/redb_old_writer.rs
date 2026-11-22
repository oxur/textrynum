use chrono::{TimeZone, Utc};
use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{
    Blake3Hash, Checkpoint, ItemProvenance, ItemState, ItemStatus, PipelineStats,
    PipelineStatus, PipelineState, RedbStateStore, RunId, SourceState, StageId, StageState,
    StageStatus, StateStore,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SPEC: &str = r#"
name = "cc-redb-fixture"
version = 1
output_dir = "./out"

[sources.local]
kind = "filesystem"
root = "./fixture-input"

[stages.extract]
adapter = "extract"
source = "local"
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("database path required")?);
    let at = Utc.with_ymd_and_hms(2026, 3, 13, 10, 0, 0).single().ok_or("fixed timestamp")?;
    let spec = PipelineSpec::from_toml(SPEC)?;

    let mut provenance = BTreeMap::new();
    provenance.insert("fixture".to_string(), serde_json::json!("old-writer"));
    let item = ItemState {
        display_name: "acme-report.txt".to_string(),
        source_id: "file-17".to_string(),
        source_name: "local".to_string(),
        content_hash: Blake3Hash::new("sha256-independent-fixture-17"),
        status: ItemStatus::Completed,
        completed_stages: Vec::new(),
        provenance: ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: provenance,
            source_modified: Some(at),
            extracted_at: at,
        },
    };
    let mut items = BTreeMap::new();
    items.insert("file-17".to_string(), item);
    let mut sources = BTreeMap::new();
    sources.insert("local".to_string(), SourceState {
        items_discovered: 2,
        items_accepted: 1,
        items_skipped_unchanged: 1,
        items,
    });
    let mut stages = BTreeMap::new();
    stages.insert(StageId::new("extract"), StageState {
        status: StageStatus::Running,
        items_processed: 1,
        items_failed: 0,
        items_skipped: 0,
        started_at: Some(at),
        completed_at: None,
    });
    let checkpoint = Checkpoint {
        version: 1,
        sequence: 42,
        created_at: at,
        spec,
        schedule: vec![vec![StageId::new("extract")]],
        spec_hash: Blake3Hash::new("spec-hash-cc-fixture-42"),
        state: PipelineState {
            run_id: RunId::new("cc-fixture-run-42"),
            pipeline_name: "cc-redb-fixture".to_string(),
            started_at: at,
            last_checkpoint: at,
            status: PipelineStatus::Running { current_stage: "extract".to_string() },
            current_batch: 0,
            sources,
            stages,
            stats: PipelineStats {
                total_items_discovered: 2,
                total_items_processed: 1,
                total_items_skipped_unchanged: 1,
                total_items_failed: 0,
            },
        },
    };
    let hashes = BTreeMap::from([
        ("acme-report.txt".to_string(), Blake3Hash::new("golden-hash-acme-17")),
        ("schedule.json".to_string(), Blake3Hash::new("golden-hash-schedule-23")),
    ]);

    let store = RedbStateStore::open(&path)?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        store.save_checkpoint(&checkpoint).await?;
        store.save_completed_hashes(&checkpoint.state.run_id, &hashes).await?;
        Ok::<_, ecl_pipeline_state::StateError>(())
    })?;
    drop(store);
    drop(runtime);
    println!("wrote baseline Redb checkpoint run=cc-fixture-run-42 sequence=42 hashes=2 path={}", path.display());
    Ok(())
}
