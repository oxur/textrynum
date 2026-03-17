//! `ecl pipeline inspect` — dump full pipeline state as JSON.

use std::path::PathBuf;

use anyhow::Result;

use ecl_pipeline_state::{RedbStateStore, StateStore};

/// Execute `ecl pipeline inspect <output-dir>`.
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

    let json = serde_json::to_string_pretty(&checkpoint.state)
        .map_err(|e| anyhow::anyhow!("failed to serialize state: {e}"))?;

    println!("{json}");
    Ok(())
}
