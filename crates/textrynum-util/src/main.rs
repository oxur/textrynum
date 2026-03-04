#![forbid(unsafe_code)]

//! Textyl — Textrynum workspace utility CLI.
//!
//! Provides version management and workspace inspection commands
//! for the Textrynum monorepo and any Rust project using fabryk/ecl crates.

use anyhow::{Result, bail};
use clap::Parser;

mod cli;
mod commands;
mod crate_info;
mod editor;
mod error;
mod output;
mod workspace;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    let root = match &args.workspace_root {
        Some(p) => std::path::PathBuf::from(p),
        None => workspace::find_project_root(&std::env::current_dir()?)?,
    };

    match args.command {
        cli::Command::Crates(crates_args) => commands::crates::run(&root, crates_args),
        cli::Command::Crate(crate_args) => commands::crate_cmd::run(&root, crate_args),
        cli::Command::SetVersion {
            version,
            project_version,
            deps_version,
            check,
        } => {
            // Resolve versions: positional `version` is shorthand for both.
            let pv = project_version.as_deref().or(version.as_deref());
            let dv = deps_version.as_deref().or(version.as_deref());

            if pv.is_none() && dv.is_none() {
                bail!(
                    "at least one of <VERSION>, --project-version, or --deps-version is required"
                );
            }

            commands::set_version::run(&root, pv, dv, check)
        }
        cli::Command::CheckVersions => {
            let ws_version = workspace::read_workspace_version(&root)?;
            commands::set_version::run(&root, Some(&ws_version), Some(&ws_version), true)
        }
    }
}
