//! `ecl pipeline resume` — resume a previously interrupted pipeline run.

use std::path::PathBuf;

use anyhow::{Context, Result};

use ecl_pipeline::PipelineRunner;
use ecl_pipeline_state::{Checkpoint, PipelineStatus, RedbStateStore, StateStore};
use ecl_pipeline_topo::resolve::resolve;

use super::registry;
use super::status::print_summary;

/// Execute `ecl pipeline resume [--force] <output-dir>`.
pub async fn execute(output_dir: PathBuf, force: bool) -> Result<()> {
    let store_path = output_dir.join("checkpoints.redb");
    if !store_path.exists() {
        anyhow::bail!(
            "no checkpoints.redb found in {}; nothing to resume",
            output_dir.display()
        );
    }

    // Load checkpoint in a scoped block so the store handle is dropped
    // before we reopen it for the runner.
    let checkpoint: Checkpoint = {
        let store = RedbStateStore::open(&store_path)?;
        store
            .load_checkpoint()
            .await?
            .ok_or_else(|| anyhow::anyhow!("checkpoint database is empty"))?
    };

    let spec = checkpoint.spec.clone();
    let pipeline_name = spec.name.clone();

    println!("Resuming pipeline: {pipeline_name}");
    println!("  Output: {}", output_dir.display());
    println!("  Checkpoint sequence: {}", checkpoint.sequence);
    println!("  Run ID: {}", checkpoint.state.run_id);

    // Check for config drift.
    let spec_bytes =
        serde_json::to_string(&spec).context("failed to serialize spec for hash comparison")?;
    let current_hash =
        ecl_pipeline_state::Blake3Hash::new(blake3::hash(spec_bytes.as_bytes()).to_hex().as_str());

    if checkpoint.config_drifted(&current_hash) {
        if force {
            println!("  WARNING: config has changed since checkpoint (--force used)");
        } else {
            anyhow::bail!("config has drifted since checkpoint. Use --force to resume anyway.");
        }
    }

    println!();

    // Re-resolve the topology from the checkpointed spec.
    let adapters = registry::resolve_adapters(&spec)?;
    let adapter_fn = registry::adapter_lookup_fn(&adapters);
    let stage_fn = registry::stage_lookup_fn(&adapters);

    let topology = resolve(spec, adapter_fn, stage_fn).await?;

    let store = Box::new(RedbStateStore::open(&store_path)?);
    let mut runner = PipelineRunner::new(topology, store).await?;
    let state = runner.run().await?;

    println!();
    print_summary(state);

    match &state.status {
        PipelineStatus::Completed { .. } => std::process::exit(0),
        PipelineStatus::Failed { .. } => std::process::exit(1),
        _ => {
            if state.stats.total_items_failed > 0 {
                std::process::exit(2);
            }
            std::process::exit(0);
        }
    }
}
