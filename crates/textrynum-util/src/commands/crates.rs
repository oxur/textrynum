//! Handler for `textyl crates` and `textyl crates update`.

use crate::cli::{CratesAction, CratesArgs};
use crate::crate_info::CrateInfo;
use crate::editor;
use crate::output;
use crate::workspace;
use anyhow::{Result, bail};
use std::io::Read;
use std::path::Path;

/// Run the `crates` command.
pub fn run(root: &Path, args: CratesArgs) -> Result<()> {
    if args.publish_order {
        return publish_order(root, &args.exclude);
    }
    match args.action {
        None => list(root, &args),
        Some(CratesAction::Update { data }) => update(root, data),
    }
}

/// Print publishable crates in dependency order (space-separated).
fn publish_order(root: &Path, exclude: &[String]) -> Result<()> {
    let order = workspace::publish_order(root)?;
    let filtered: Vec<&str> = order
        .iter()
        .filter(|name| !exclude.contains(name))
        .map(|s| s.as_str())
        .collect();
    println!("{}", filtered.join(" "));
    Ok(())
}

/// List all workspace crates and their internal deps.
fn list(root: &Path, args: &CratesArgs) -> Result<()> {
    let crates = workspace::scan_all_crates(root)?;
    let output = output::format_crate_list(&crates, &args.format)?;
    print!("{output}");
    Ok(())
}

/// Apply version updates from JSON input.
fn update(root: &Path, data: Option<String>) -> Result<()> {
    let json = match data {
        Some(d) => d,
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    let crates: Vec<CrateInfo> = serde_json::from_str(&json)?;
    let member_paths = workspace::list_member_paths(root)?;
    let internal_names = workspace::collect_internal_crate_names(root)?;

    let mut total_changes = 0u32;

    for crate_info in &crates {
        // Find the crate's directory.
        let crate_path = member_paths.iter().find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == crate_info.name || p.ends_with(&crate_info.path))
                .unwrap_or(false)
        });

        let Some(crate_path) = crate_path else {
            bail!("crate '{}' not found in workspace members", crate_info.name);
        };

        let cargo_toml = crate_path.join("Cargo.toml");

        for dep in &crate_info.internal_deps {
            if !internal_names.contains(&dep.name) {
                continue;
            }
            let changed = editor::update_dep_version(
                &cargo_toml,
                &dep.section,
                &dep.name,
                &dep.declared_version,
            )?;
            if changed {
                total_changes += 1;
                println!(
                    "  Updated {} -> {} = \"{}\"",
                    crate_info.name, dep.name, dep.declared_version
                );
            }
        }
    }

    println!("{total_changes} dependency version(s) updated.");
    Ok(())
}
