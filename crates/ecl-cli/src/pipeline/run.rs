//! `ecl pipeline run` — run a pipeline from a TOML configuration file.

use std::path::PathBuf;

use anyhow::{Context, Result};

use ecl_pipeline::PipelineRunner;
use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{PipelineStatus, RedbStateStore};
use ecl_pipeline_topo::resolve::resolve;

use super::registry;
use super::status::print_summary;

/// Execute `ecl pipeline run <config.toml>`.
pub async fn execute(config_path: PathBuf) -> Result<()> {
    let toml_content = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;

    let spec = PipelineSpec::from_toml(&toml_content)
        .with_context(|| format!("failed to parse config: {}", config_path.display()))?;

    let output_dir = spec.output_dir.clone();
    let pipeline_name = spec.name.clone();

    println!("Running pipeline: {pipeline_name}");
    println!("  Config: {}", config_path.display());
    println!("  Output: {}", output_dir.display());
    println!();

    // Pre-resolve adapters, then use them for both lookups.
    let adapters = registry::resolve_adapters(&spec)?;
    let adapter_fn = registry::adapter_lookup_fn(&adapters);
    let stage_fn = registry::stage_lookup_fn(&adapters);

    let topology = resolve(spec, adapter_fn, stage_fn).await?;

    let store_path = output_dir.join("checkpoints.redb");
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
