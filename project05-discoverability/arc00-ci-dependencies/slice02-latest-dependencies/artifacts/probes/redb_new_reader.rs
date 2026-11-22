use ecl_pipeline_state::{Blake3Hash, RedbStateStore, StateStore};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("copied database path required")?);

    // Read the exact old writer bytes independently before product deserialization.
    // The direct Redb handle and transaction are fully dropped before RedbStateStore opens.
    let (raw_checkpoint, latest_run_id, completed_run_id, checkpoint_count) = {
        let database = Database::open(&path)?;
        let transaction = database.begin_read()?;
        let checkpoints: TableDefinition<&str, &[u8]> = TableDefinition::new("checkpoints");
        let metadata: TableDefinition<&str, &str> = TableDefinition::new("metadata");
        let (raw, count) = {
            let table = transaction.open_table(checkpoints)?;
            let raw = table
                .get("cc-fixture-run-42")?
                .expect("old writer checkpoint exists")
                .value()
                .to_vec();
            let count = table.iter()?.count();
            (raw, count)
        };
        let (latest, completed) = {
            let table = transaction.open_table(metadata)?;
            (
                table.get("latest_run_id")?.expect("latest pointer exists").value().to_owned(),
                table.get("latest_completed_run_id")?.expect("completed pointer exists").value().to_owned(),
            )
        };
        (raw, latest, completed, count)
    };
    assert_eq!(checkpoint_count, 1, "fixture has exactly one checkpoint entry");
    assert_eq!(latest_run_id, "cc-fixture-run-42");
    assert_eq!(completed_run_id, "cc-fixture-run-42");
    let old_checkpoint: serde_json::Value = serde_json::from_slice(&raw_checkpoint)?;

    let store = RedbStateStore::open(&path)?;
    let checkpoint = store
        .load_checkpoint()
        .await?
        .expect("golden checkpoint exists");
    assert_eq!(checkpoint.version, 1);
    assert_eq!(checkpoint.sequence, 42);
    assert_eq!(checkpoint.state.run_id.as_str(), "cc-fixture-run-42");
    assert_eq!(checkpoint.spec_hash.as_str(), "spec-hash-cc-fixture-42");
    assert_eq!(checkpoint.spec.name, "cc-redb-fixture");
    assert_eq!(checkpoint.state.pipeline_name, "cc-redb-fixture");
    assert!(matches!(
        checkpoint.state.status,
        ecl_pipeline_state::PipelineStatus::Running { ref current_stage }
            if current_stage == "extract"
    ));
    assert_eq!(checkpoint.state.stats.total_items_discovered, 2);
    assert_eq!(checkpoint.state.stats.total_items_processed, 1);
    assert_eq!(checkpoint.state.stats.total_items_skipped_unchanged, 1);
    assert_eq!(checkpoint.state.stats.total_items_failed, 0);
    assert_eq!(checkpoint.state.sources["local"].items_discovered, 2);
    assert_eq!(checkpoint.state.sources["local"].items_accepted, 1);
    assert_eq!(checkpoint.state.sources["local"].items_skipped_unchanged, 1);
    let item = &checkpoint.state.sources["local"].items["file-17"];
    assert_eq!(item.display_name, "acme-report.txt");
    assert_eq!(item.content_hash.as_str(), "sha256-independent-fixture-17");
    assert_eq!(item.provenance.source_kind, "filesystem");
    assert_eq!(item.provenance.metadata["fixture"], "old-writer");
    let loaded_checkpoint = serde_json::to_value(&checkpoint)?;
    assert_eq!(loaded_checkpoint, old_checkpoint, "every checkpoint field survives semantically");

    let hashes = store.load_previous_hashes().await?;
    assert_eq!(
        hashes,
        BTreeMap::from([
            (
                "acme-report.txt".to_string(),
                Blake3Hash::new("golden-hash-acme-17"),
            ),
            (
                "schedule.json".to_string(),
                Blake3Hash::new("golden-hash-schedule-23"),
            ),
        ])
    );
    drop(store);
    println!("upgraded Redb reader whole-state JSON equality passed for checkpoint run=cc-fixture-run-42; raw_bytes={} checkpoint_entries={checkpoint_count}; both metadata pointers and two exact hashes passed", raw_checkpoint.len());
    Ok(())
}
