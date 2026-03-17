//! Pipeline CLI subcommands.
//!
//! Implements `ecl pipeline run|resume|status|inspect|items|diff`.

mod inspect;
mod items;
mod registry;
mod resume;
mod run;
mod status;

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

/// Pipeline subcommands.
#[derive(Subcommand, Debug)]
pub enum PipelineCommand {
    /// Run a pipeline from a TOML configuration file.
    Run {
        /// Path to the pipeline TOML configuration file.
        config: PathBuf,
    },

    /// Resume a previously interrupted pipeline run.
    Resume {
        /// Path to the pipeline output directory (contains checkpoints).
        output_dir: PathBuf,

        /// Resume even if the configuration has drifted since the checkpoint.
        #[arg(long)]
        force: bool,
    },

    /// Show a human-readable summary of pipeline status.
    Status {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,
    },

    /// Print full pipeline state as JSON.
    Inspect {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,
    },

    /// List all items with their status.
    Items {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,

        /// Filter by item status (pending, completed, failed, skipped, unchanged).
        #[arg(long)]
        status: Option<String>,
    },

    /// Compare two pipeline runs.
    Diff {
        /// Path to the first pipeline output directory.
        dir1: PathBuf,

        /// Path to the second pipeline output directory.
        dir2: PathBuf,
    },
}

/// Execute a pipeline subcommand.
pub async fn execute(command: PipelineCommand) -> Result<()> {
    match command {
        PipelineCommand::Run { config } => run::execute(config).await,
        PipelineCommand::Resume { output_dir, force } => resume::execute(output_dir, force).await,
        PipelineCommand::Status { output_dir } => status::execute(output_dir).await,
        PipelineCommand::Inspect { output_dir } => inspect::execute(output_dir).await,
        PipelineCommand::Items { output_dir, status } => items::execute(output_dir, status).await,
        PipelineCommand::Diff { dir1, dir2 } => diff_runs(dir1, dir2).await,
    }
}

/// Compare two pipeline runs.
async fn diff_runs(dir1: PathBuf, dir2: PathBuf) -> Result<()> {
    let store1 = ecl_pipeline_state::RedbStateStore::open(dir1.join("checkpoints.redb"))?;
    let store2 = ecl_pipeline_state::RedbStateStore::open(dir2.join("checkpoints.redb"))?;

    use ecl_pipeline_state::StateStore;

    let cp1 = store1
        .load_checkpoint()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no checkpoint found in {}", dir1.display()))?;
    let cp2 = store2
        .load_checkpoint()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no checkpoint found in {}", dir2.display()))?;

    println!("Comparing runs:");
    println!("  Left:  {} (run {})", dir1.display(), cp1.state.run_id);
    println!("  Right: {} (run {})", dir2.display(), cp2.state.run_id);
    println!();

    // Collect all item IDs from both runs.
    let mut left_items = std::collections::BTreeMap::new();
    let mut right_items = std::collections::BTreeMap::new();

    for source in cp1.state.sources.values() {
        for (id, item) in &source.items {
            left_items.insert(id.clone(), item);
        }
    }
    for source in cp2.state.sources.values() {
        for (id, item) in &source.items {
            right_items.insert(id.clone(), item);
        }
    }

    let mut added = 0usize;
    let mut removed = 0usize;
    let mut changed = 0usize;
    let mut unchanged = 0usize;

    // Items only in right (added).
    for id in right_items.keys() {
        if !left_items.contains_key(id) {
            added += 1;
            println!("  + {id}");
        }
    }

    // Items only in left (removed).
    for id in left_items.keys() {
        if !right_items.contains_key(id) {
            removed += 1;
            println!("  - {id}");
        }
    }

    // Items in both — compare content hash.
    for (id, left) in &left_items {
        if let Some(right) = right_items.get(id) {
            if left.content_hash != right.content_hash {
                changed += 1;
                println!("  ~ {id}");
            } else {
                unchanged += 1;
            }
        }
    }

    println!();
    println!(
        "Summary: +{added} added, -{removed} removed, ~{changed} changed, {unchanged} unchanged"
    );

    // Stage outcome differences.
    println!();
    println!("Stage comparison:");
    let all_stages: std::collections::BTreeSet<_> = cp1
        .state
        .stages
        .keys()
        .chain(cp2.state.stages.keys())
        .collect();

    for stage_id in all_stages {
        let left_stage = cp1.state.stages.get(stage_id);
        let right_stage = cp2.state.stages.get(stage_id);
        match (left_stage, right_stage) {
            (Some(l), Some(r)) => {
                let l_status = format!("{:?}", l.status);
                let r_status = format!("{:?}", r.status);
                if l_status != r_status {
                    println!("  {}: {} -> {}", stage_id, l_status, r_status);
                }
            }
            (None, Some(_)) => println!("  {stage_id}: (new)"),
            (Some(_), None) => println!("  {stage_id}: (removed)"),
            (None, None) => {}
        }
    }

    Ok(())
}
