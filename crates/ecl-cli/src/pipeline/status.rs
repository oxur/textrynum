//! `ecl pipeline status` — human-readable pipeline status summary.

use std::path::PathBuf;

use anyhow::Result;

use ecl_pipeline_state::{
    ItemStatus, PipelineState, PipelineStatus, RedbStateStore, StageStatus, StateStore,
};

/// Execute `ecl pipeline status <output-dir>`.
pub async fn execute(output_dir: PathBuf) -> Result<()> {
    let store_path = output_dir.join("checkpoints.redb");
    if !store_path.exists() {
        anyhow::bail!("no checkpoints.redb found in {}", output_dir.display());
    }

    let store = RedbStateStore::open(&store_path)?;
    let checkpoint = store
        .load_checkpoint()
        .await?
        .ok_or_else(|| anyhow::anyhow!("checkpoint database is empty"))?;

    print_summary(&checkpoint.state);
    Ok(())
}

/// Print a human-readable summary of pipeline state.
pub fn print_summary(state: &PipelineState) {
    println!("Pipeline: {}", state.pipeline_name);
    println!("Run ID:   {}", state.run_id);
    println!("Started:  {}", state.started_at);

    let status_str = match &state.status {
        PipelineStatus::Pending => "Pending".to_string(),
        PipelineStatus::Running { current_stage } => format!("Running ({current_stage})"),
        PipelineStatus::Completed { finished_at } => format!("Completed ({finished_at})"),
        PipelineStatus::Failed { error, failed_at } => {
            format!("Failed at {failed_at}: {error}")
        }
        PipelineStatus::Interrupted { interrupted_at } => {
            format!("Interrupted ({interrupted_at})")
        }
    };
    println!("Status:   {status_str}");
    println!();

    // Item statistics.
    println!("Items:");
    println!("  Discovered: {}", state.stats.total_items_discovered);
    println!("  Processed:  {}", state.stats.total_items_processed);
    println!(
        "  Unchanged:  {}",
        state.stats.total_items_skipped_unchanged
    );
    println!("  Failed:     {}", state.stats.total_items_failed);
    println!();

    // Per-source summary.
    if !state.sources.is_empty() {
        println!("Sources:");
        for (name, source) in &state.sources {
            let failed = source
                .items
                .values()
                .filter(|i| matches!(i.status, ItemStatus::Failed { .. }))
                .count();
            let completed = source
                .items
                .values()
                .filter(|i| matches!(i.status, ItemStatus::Completed))
                .count();
            println!(
                "  {name}: {discovered} discovered, {completed} completed, {failed} failed",
                discovered = source.items_discovered,
            );
        }
        println!();
    }

    // Per-stage summary.
    if !state.stages.is_empty() {
        println!("Stages:");
        for (id, stage) in &state.stages {
            let status = match &stage.status {
                StageStatus::Pending => "Pending",
                StageStatus::Running => "Running",
                StageStatus::Completed => "Completed",
                StageStatus::Skipped { .. } => "Skipped",
                StageStatus::Failed { .. } => "Failed",
            };
            println!(
                "  {id}: {status} ({processed} processed, {failed} failed, {skipped} skipped)",
                processed = stage.items_processed,
                failed = stage.items_failed,
                skipped = stage.items_skipped,
            );
        }
    }
}
