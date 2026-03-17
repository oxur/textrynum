#![forbid(unsafe_code)]

//! ECL CLI
//!
//! Command-line interface for ECL workflow management and pipeline execution.

mod pipeline;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// ECL Command-Line Interface
#[derive(Parser, Debug)]
#[command(author, version, about = "ECL — Extract, Curate, Load")]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Top-level commands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Pipeline execution and inspection commands.
    Pipeline {
        #[command(subcommand)]
        command: pipeline::PipelineCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity.
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    match cli.command {
        Commands::Pipeline { command } => pipeline::execute(command).await,
    }
}
