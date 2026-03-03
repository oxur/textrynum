#![forbid(unsafe_code)]

//! Textyl — Textrynum workspace utility CLI.
//!
//! Provides version management and workspace inspection commands
//! for the Textrynum monorepo.

use anyhow::Result;
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
        None => workspace::find_workspace_root(&std::env::current_dir()?)?,
    };

    match args.command {
        cli::Command::Crates(crates_args) => commands::crates::run(&root, crates_args),
        cli::Command::Crate(crate_args) => commands::crate_cmd::run(&root, crate_args),
        cli::Command::SetVersion { version, check } => {
            commands::set_version::run(&root, &version, check)
        }
        cli::Command::CheckVersions => {
            let ws_version = workspace::read_workspace_version(&root)?;
            commands::set_version::run(&root, &ws_version, true)
        }
    }
}
