//! `ecl pipeline items` — list pipeline items with status.

use std::path::PathBuf;

use anyhow::Result;

use ecl_pipeline_state::{ItemStatus, RedbStateStore, StateStore};

/// Execute `ecl pipeline items <output-dir> [--status <filter>]`.
pub async fn execute(output_dir: PathBuf, status_filter: Option<String>) -> Result<()> {
    let store_path = output_dir.join("checkpoints.redb");
    if !store_path.exists() {
        anyhow::bail!("no checkpoints.redb found in {}", output_dir.display());
    }

    let store = RedbStateStore::open(&store_path)?;
    let checkpoint = store
        .load_checkpoint()
        .await?
        .ok_or_else(|| anyhow::anyhow!("checkpoint database is empty"))?;

    // Print header.
    println!(
        "{:<40} {:<20} {:<12} {:<40}",
        "ID", "NAME", "STATUS", "HASH"
    );
    println!("{}", "-".repeat(112));

    let mut count = 0usize;
    for source in checkpoint.state.sources.values() {
        for (id, item) in &source.items {
            let status_str = status_display(&item.status);

            // Apply filter if specified.
            if let Some(ref filter) = status_filter
                && !status_str.eq_ignore_ascii_case(filter)
            {
                continue;
            }

            let hash = item.content_hash.as_str();
            let hash_short = if hash.len() > 16 { &hash[..16] } else { hash };

            println!(
                "{:<40} {:<20} {:<12} {hash_short}",
                truncate(id, 39),
                truncate(&item.display_name, 19),
                status_str,
            );
            count += 1;
        }
    }

    println!();
    println!("{count} item(s)");

    Ok(())
}

/// Convert an `ItemStatus` to a display string.
fn status_display(status: &ItemStatus) -> &'static str {
    match status {
        ItemStatus::Pending => "Pending",
        ItemStatus::Processing { .. } => "Processing",
        ItemStatus::Completed => "Completed",
        ItemStatus::Failed { .. } => "Failed",
        ItemStatus::Skipped { .. } => "Skipped",
        ItemStatus::Unchanged => "Unchanged",
    }
}

/// Truncate a string to fit a column width.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
