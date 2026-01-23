#![forbid(unsafe_code)]

//! ECL CLI
//!
//! Command-line interface for ECL.

use anyhow::Result;
use clap::Parser;

/// ECL Command-Line Interface
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("ECL CLI - Coming in later stages!");

    Ok(())
}
