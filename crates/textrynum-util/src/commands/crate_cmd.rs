//! Handler for `textyl crate update`.

use crate::cli::{CrateAction, CrateArgs};
use crate::crate_info::DepInfo;
use crate::editor;
use crate::error::TextylError;
use crate::workspace;
use anyhow::Result;
use std::path::Path;

const DEP_SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// Run the `crate` command.
pub fn run(root: &Path, args: CrateArgs) -> Result<()> {
    match args.action {
        CrateAction::Update {
            crate_name,
            dep,
            version,
            data,
        } => update(root, &crate_name, dep, version, data),
    }
}

/// Update dependency version(s) for a single crate.
fn update(
    root: &Path,
    crate_name: &str,
    dep: Option<String>,
    version: Option<String>,
    data: Option<String>,
) -> Result<()> {
    let member_paths = workspace::list_member_paths(root)?;
    let crate_path = member_paths
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == crate_name)
        })
        .ok_or_else(|| TextylError::CrateNotFound {
            name: crate_name.to_string(),
        })?;

    let cargo_toml = crate_path.join("Cargo.toml");

    // JSON mode: bulk update from data.
    if let Some(json) = data {
        let deps: Vec<DepInfo> = serde_json::from_str(&json)?;
        let mut changes = 0u32;
        for dep_info in &deps {
            let changed = editor::update_dep_version(
                &cargo_toml,
                &dep_info.section,
                &dep_info.name,
                &dep_info.declared_version,
            )?;
            if changed {
                changes += 1;
                println!(
                    "  Updated {} -> {} = \"{}\"",
                    crate_name, dep_info.name, dep_info.declared_version
                );
            }
        }
        println!("{changes} dependency version(s) updated in {crate_name}.");
        return Ok(());
    }

    // Flag mode: single dep update.
    let dep_name =
        dep.ok_or_else(|| anyhow::anyhow!("either --dep/--version or --data must be provided"))?;
    let new_version =
        version.ok_or_else(|| anyhow::anyhow!("--version is required when using --dep"))?;

    let mut changed = false;
    for section in &DEP_SECTIONS {
        if editor::update_dep_version(&cargo_toml, section, &dep_name, &new_version)? {
            changed = true;
            println!("  Updated {crate_name} [{section}] {dep_name} = \"{new_version}\"");
        }
    }

    if !changed {
        return Err(TextylError::DepNotFound {
            crate_name: crate_name.to_string(),
            dep: dep_name,
        }
        .into());
    }

    Ok(())
}
